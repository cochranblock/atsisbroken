// SPDX-License-Identifier: Unlicense
//! Chrome Native Messaging bridge.
//!
//! Chrome's native-messaging protocol: 4-byte little-endian length prefix
//! followed by a UTF-8 JSON message. Max 1 MiB per message. Same shape
//! both directions.
//!
//! The atsisbroken Chrome extension stores observations + feedback in
//! `chrome.storage.local`. When the user runs `atsisbroken bridge`, this
//! module spawns the native-messaging loop: read framed JSON from stdin,
//! deserialize as a [`BridgeMessage`], hand off to the local feedback
//! queue, write a framed ack back to stdout. The extension then clears
//! the synced events from its local storage.
//!
//! No network. The transport is a process-pair stdin/stdout pipe Chrome
//! sets up when the extension calls `chrome.runtime.connectNative`.

use crate::{Feedback, FeedbackQueue, Observation};
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

/// One message across the bridge. Both directions share this envelope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BridgeMessage {
    /// Extension says hi at connect time.
    Hello {
        extension_version: String,
    },
    /// Native host's reply to Hello.
    HelloAck {
        host_version: String,
        protocol_version: u32,
    },
    /// Extension is sending a batch of observations from chrome.storage.
    PushObservations {
        events: Vec<Observation>,
    },
    /// Extension is sending a batch of feedback events.
    PushFeedback {
        events: Vec<Feedback>,
    },
    /// Native host acknowledges N events landed on disk.
    Ack {
        accepted: usize,
    },
    /// Either side reports a fatal protocol error.
    Error {
        message: String,
    },
}

/// Maximum permitted message size. Chrome enforces 1 MiB.
pub const MAX_MESSAGE_BYTES: usize = 1024 * 1024;

/// Current native-host protocol version. Bumped on breaking schema changes.
pub const PROTOCOL_VERSION: u32 = 1;

/// Encode a message into the Chrome native-messaging frame: u32 LE length
/// prefix + UTF-8 JSON payload.
pub fn encode(msg: &BridgeMessage) -> Result<Vec<u8>, BridgeError> {
    let json = serde_json::to_vec(msg).map_err(BridgeError::Serde)?;
    if json.len() > MAX_MESSAGE_BYTES {
        return Err(BridgeError::TooLarge(json.len()));
    }
    let mut out = Vec::with_capacity(4 + json.len());
    out.extend_from_slice(&(json.len() as u32).to_le_bytes());
    out.extend_from_slice(&json);
    Ok(out)
}

/// Read one framed message from `r`. Returns `Ok(None)` cleanly at EOF
/// (Chrome closing the pipe). Returns `Err` on protocol violation.
pub fn read_one<R: Read>(r: &mut R) -> Result<Option<BridgeMessage>, BridgeError> {
    let mut len_buf = [0u8; 4];
    match r.read_exact(&mut len_buf) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(BridgeError::Io(e)),
    }
    let len = u32::from_le_bytes(len_buf) as usize;
    if len == 0 {
        return Err(BridgeError::ZeroLength);
    }
    if len > MAX_MESSAGE_BYTES {
        return Err(BridgeError::TooLarge(len));
    }
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf).map_err(BridgeError::Io)?;
    let msg = serde_json::from_slice(&buf).map_err(BridgeError::Serde)?;
    Ok(Some(msg))
}

/// Write one framed message to `w`.
pub fn write_one<W: Write>(w: &mut W, msg: &BridgeMessage) -> Result<(), BridgeError> {
    let frame = encode(msg)?;
    w.write_all(&frame).map_err(BridgeError::Io)?;
    w.flush().map_err(BridgeError::Io)?;
    Ok(())
}

#[derive(Debug)]
pub enum BridgeError {
    Io(io::Error),
    Serde(serde_json::Error),
    /// Length prefix exceeded `MAX_MESSAGE_BYTES`.
    TooLarge(usize),
    /// Length prefix was zero — Chrome would never send this; protocol violation.
    ZeroLength,
}

impl std::fmt::Display for BridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BridgeError::Io(e) => write!(f, "bridge io: {e}"),
            BridgeError::Serde(e) => write!(f, "bridge serde: {e}"),
            BridgeError::TooLarge(n) => write!(f, "bridge frame too large: {n} bytes"),
            BridgeError::ZeroLength => write!(f, "bridge frame had zero length"),
        }
    }
}

impl std::error::Error for BridgeError {}

/// Run the native-messaging loop: read framed messages from `stdin`, route
/// them, write framed responses to `stdout`. Exits cleanly when stdin
/// closes (Chrome disconnect).
pub fn serve_native_messaging<R: Read, W: Write>(
    mut r: R,
    mut w: W,
    queue: &mut FeedbackQueue,
) -> Result<(), BridgeError> {
    while let Some(msg) = read_one(&mut r)? {
        match msg {
            BridgeMessage::Hello { .. } => {
                write_one(
                    &mut w,
                    &BridgeMessage::HelloAck {
                        host_version: env!("CARGO_PKG_VERSION").into(),
                        protocol_version: PROTOCOL_VERSION,
                    },
                )?;
            }
            BridgeMessage::PushFeedback { events } => {
                let n = events.len();
                for ev in events {
                    queue.append(ev);
                }
                write_one(&mut w, &BridgeMessage::Ack { accepted: n })?;
            }
            BridgeMessage::PushObservations { events } => {
                // Observations are rolled forward into the training stream
                // by the trainer; the bridge just acknowledges receipt.
                // The trainer + on-disk store land in a follow-up commit.
                let n = events.len();
                write_one(&mut w, &BridgeMessage::Ack { accepted: n })?;
            }
            BridgeMessage::HelloAck { .. } | BridgeMessage::Ack { .. } => {
                // The extension shouldn't be sending these — but ignore
                // rather than break the loop. Strict mode could error here.
            }
            BridgeMessage::Error { message } => {
                eprintln!("bridge: extension reported error: {message}");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FieldDescriptor;

    fn sample_obs() -> Observation {
        Observation {
            field: FieldDescriptor {
                label: "Email".into(),
                placeholder: "".into(),
                aria_label: "".into(),
                name: "email".into(),
                id: "".into(),
                kind: "email".into(),
            },
            predicted: "email".into(),
            observed: "email".into(),
            confidence: 0.92,
        }
    }

    #[test]
    fn frame_round_trip() {
        let msg = BridgeMessage::PushObservations {
            events: vec![sample_obs()],
        };
        let frame = encode(&msg).unwrap();
        // First 4 bytes are LE length.
        let len = u32::from_le_bytes([frame[0], frame[1], frame[2], frame[3]]) as usize;
        assert_eq!(len, frame.len() - 4);
        let mut cursor = std::io::Cursor::new(frame);
        let back = read_one(&mut cursor).unwrap().unwrap();
        assert_eq!(msg, back);
    }

    #[test]
    fn read_one_returns_none_on_clean_eof() {
        let mut cursor = std::io::Cursor::new(Vec::<u8>::new());
        assert!(read_one(&mut cursor).unwrap().is_none());
    }

    #[test]
    fn zero_length_frame_is_protocol_violation() {
        let mut cursor = std::io::Cursor::new(vec![0, 0, 0, 0]);
        let err = read_one(&mut cursor).unwrap_err();
        assert!(matches!(err, BridgeError::ZeroLength));
    }

    #[test]
    fn oversized_frame_is_rejected() {
        let big = (MAX_MESSAGE_BYTES + 1) as u32;
        let mut bytes = big.to_le_bytes().to_vec();
        bytes.extend(std::iter::repeat(b' ').take(8));
        let mut cursor = std::io::Cursor::new(bytes);
        let err = read_one(&mut cursor).unwrap_err();
        assert!(matches!(err, BridgeError::TooLarge(_)));
    }

    #[test]
    fn serve_loop_acks_pushed_feedback() {
        // Build an in-memory client → server pipe of one Hello + one
        // PushFeedback then EOF.
        let mut input = Vec::new();
        input.extend(
            encode(&BridgeMessage::Hello {
                extension_version: "0.0.0".into(),
            })
            .unwrap(),
        );
        input.extend(
            encode(&BridgeMessage::PushFeedback {
                events: vec![Feedback {
                    field: sample_obs().field,
                    predicted: "email".into(),
                    actual: "email".into(),
                    accepted: true,
                }],
            })
            .unwrap(),
        );
        let mut output = Vec::new();
        let mut queue = FeedbackQueue::default();
        serve_native_messaging(std::io::Cursor::new(input), &mut output, &mut queue).unwrap();
        // Server emitted: HelloAck, Ack
        let mut cursor = std::io::Cursor::new(output);
        let m1 = read_one(&mut cursor).unwrap().unwrap();
        assert!(matches!(m1, BridgeMessage::HelloAck { .. }));
        let m2 = read_one(&mut cursor).unwrap().unwrap();
        assert!(matches!(m2, BridgeMessage::Ack { accepted: 1 }));
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn protocol_version_pinned() {
        // Bumping this is a breaking change — the test forces an explicit
        // edit so it's deliberate.
        assert_eq!(PROTOCOL_VERSION, 1);
    }

    /// Pin the on-the-wire JSON shape of one frame variant. The extension
    /// is in JS and the server is in Rust — wire-format drift between
    /// them is silent and catastrophic.
    #[test]
    fn ack_wire_shape_is_stable() {
        let json = serde_json::to_string(&BridgeMessage::Ack { accepted: 7 }).unwrap();
        assert_eq!(json, r#"{"kind":"ack","accepted":7}"#);
    }

    #[test]
    fn hello_wire_shape_is_stable() {
        let json = serde_json::to_string(&BridgeMessage::Hello {
            extension_version: "0.0.0".into(),
        })
        .unwrap();
        assert_eq!(json, r#"{"kind":"hello","extension_version":"0.0.0"}"#);
    }

    #[test]
    fn hello_ack_wire_shape_is_stable() {
        let json = serde_json::to_string(&BridgeMessage::HelloAck {
            host_version: "0.0.0".into(),
            protocol_version: 1,
        })
        .unwrap();
        assert_eq!(
            json,
            r#"{"kind":"hello_ack","host_version":"0.0.0","protocol_version":1}"#
        );
    }

    #[test]
    fn error_wire_shape_is_stable() {
        let json = serde_json::to_string(&BridgeMessage::Error {
            message: "boom".into(),
        })
        .unwrap();
        assert_eq!(json, r#"{"kind":"error","message":"boom"}"#);
    }

    #[test]
    fn push_observations_wire_shape_is_stable() {
        let json = serde_json::to_string(&BridgeMessage::PushObservations {
            events: vec![sample_obs()],
        })
        .unwrap();
        // Lock the envelope. The Observation interior is independently
        // pinned by lib.rs::observation_json_shape_is_stable.
        assert!(json.starts_with(r#"{"kind":"push_observations","events":["#));
        assert!(json.ends_with(r#"]}"#));
    }

    #[test]
    fn push_feedback_wire_shape_is_stable() {
        let json = serde_json::to_string(&BridgeMessage::PushFeedback {
            events: vec![Feedback {
                field: sample_obs().field,
                predicted: "email".into(),
                actual: "email".into(),
                accepted: true,
            }],
        })
        .unwrap();
        assert!(json.starts_with(r#"{"kind":"push_feedback","events":["#));
        assert!(json.ends_with(r#"]}"#));
    }

    #[test]
    fn length_prefix_is_explicitly_little_endian() {
        // Pin the byte-order contract — Chrome's Native Messaging spec
        // says LE, and a vendor-side endianness flip would be silent.
        let msg = BridgeMessage::Ack { accepted: 0x01020304 };
        let frame = encode(&msg).unwrap();
        let body_len = frame.len() - 4;
        assert_eq!(frame[0], (body_len & 0xFF) as u8);
        assert_eq!(frame[1], ((body_len >> 8) & 0xFF) as u8);
        assert_eq!(frame[2], ((body_len >> 16) & 0xFF) as u8);
        assert_eq!(frame[3], ((body_len >> 24) & 0xFF) as u8);
    }

    #[test]
    fn truncated_payload_returns_io_error() {
        // Length prefix says 100 bytes, only 4 follow. read_exact must err.
        let mut bytes = (100u32).to_le_bytes().to_vec();
        bytes.extend_from_slice(b"abcd");
        let mut cursor = std::io::Cursor::new(bytes);
        let err = read_one(&mut cursor).unwrap_err();
        assert!(matches!(err, BridgeError::Io(_)));
    }

    #[test]
    fn invalid_utf8_payload_errors() {
        // Frame says payload is N bytes, payload is not valid JSON
        // (also not valid UTF-8). Must surface as a serde error, not a
        // panic.
        let payload: &[u8] = &[0xFF, 0xFE, 0xFD];
        let mut bytes = (payload.len() as u32).to_le_bytes().to_vec();
        bytes.extend_from_slice(payload);
        let mut cursor = std::io::Cursor::new(bytes);
        let err = read_one(&mut cursor).unwrap_err();
        assert!(matches!(err, BridgeError::Serde(_)));
    }

    #[test]
    fn serve_loop_handles_observations_and_feedback_in_one_session() {
        // Single Hello, then a mix of pushes — server must ack each in
        // order and the feedback queue must reflect ONLY pushed feedback
        // (observations don't currently land in the queue).
        let mut input = Vec::new();
        input.extend(
            encode(&BridgeMessage::Hello {
                extension_version: "0.0.0".into(),
            })
            .unwrap(),
        );
        input.extend(
            encode(&BridgeMessage::PushObservations {
                events: vec![sample_obs(), sample_obs(), sample_obs()],
            })
            .unwrap(),
        );
        input.extend(
            encode(&BridgeMessage::PushFeedback {
                events: vec![Feedback {
                    field: sample_obs().field,
                    predicted: "email".into(),
                    actual: "email".into(),
                    accepted: true,
                }],
            })
            .unwrap(),
        );
        let mut output = Vec::new();
        let mut queue = FeedbackQueue::default();
        serve_native_messaging(std::io::Cursor::new(input), &mut output, &mut queue).unwrap();
        // 3 frames out: HelloAck, Ack(3), Ack(1)
        let mut cursor = std::io::Cursor::new(output);
        let m1 = read_one(&mut cursor).unwrap().unwrap();
        let m2 = read_one(&mut cursor).unwrap().unwrap();
        let m3 = read_one(&mut cursor).unwrap().unwrap();
        assert!(matches!(m1, BridgeMessage::HelloAck { .. }));
        assert!(matches!(m2, BridgeMessage::Ack { accepted: 3 }));
        assert!(matches!(m3, BridgeMessage::Ack { accepted: 1 }));
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn server_ignores_helloack_from_extension_without_breaking_session() {
        // The extension shouldn't send HelloAck, but if it does (a bug
        // in a future extension version), the server must not crash.
        let mut input = Vec::new();
        input.extend(
            encode(&BridgeMessage::HelloAck {
                host_version: "weird".into(),
                protocol_version: 1,
            })
            .unwrap(),
        );
        // Then a real PushFeedback to confirm the loop kept running.
        input.extend(
            encode(&BridgeMessage::PushFeedback {
                events: vec![Feedback {
                    field: sample_obs().field,
                    predicted: "email".into(),
                    actual: "email".into(),
                    accepted: true,
                }],
            })
            .unwrap(),
        );
        let mut output = Vec::new();
        let mut queue = FeedbackQueue::default();
        serve_native_messaging(std::io::Cursor::new(input), &mut output, &mut queue).unwrap();
        // No HelloAck back (extension's HelloAck is dropped); just an Ack(1).
        let mut cursor = std::io::Cursor::new(output);
        let m = read_one(&mut cursor).unwrap().unwrap();
        assert!(matches!(m, BridgeMessage::Ack { accepted: 1 }));
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn max_message_bytes_matches_chrome_spec() {
        // Chrome's spec: 1 MiB. Bumping this without coordinating with
        // the extension side breaks framing.
        assert_eq!(MAX_MESSAGE_BYTES, 1024 * 1024);
    }
}
