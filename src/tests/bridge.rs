// SPDX-License-Identifier: Unlicense

//! Tests for `crate::bridge` (Phase 5).

use crate::bridge::{
    encode, read_one, serve_native_messaging, BridgeError, BridgeMessage, MAX_MESSAGE_BYTES,
    PROTOCOL_VERSION,
};
use crate::{Feedback, FeedbackQueue, FieldDescriptor, Observation};

use super::{case, check, check_eq, TestResult};

pub fn run() -> Vec<TestResult> {
    vec![
        case("bridge::frame_round_trip", frame_round_trip),
        case("bridge::read_one_returns_none_on_clean_eof", read_one_returns_none_on_clean_eof),
        case("bridge::zero_length_frame_is_protocol_violation",
             zero_length_frame_is_protocol_violation),
        case("bridge::oversized_frame_is_rejected", oversized_frame_is_rejected),
        case("bridge::serve_loop_acks_pushed_feedback", serve_loop_acks_pushed_feedback),
        case("bridge::protocol_version_pinned", protocol_version_pinned),
        case("bridge::ack_wire_shape_is_stable", ack_wire_shape_is_stable),
        case("bridge::hello_wire_shape_is_stable", hello_wire_shape_is_stable),
        case("bridge::hello_ack_wire_shape_is_stable", hello_ack_wire_shape_is_stable),
        case("bridge::error_wire_shape_is_stable", error_wire_shape_is_stable),
        case("bridge::push_observations_wire_shape_is_stable", push_observations_wire_shape_is_stable),
        case("bridge::push_feedback_wire_shape_is_stable", push_feedback_wire_shape_is_stable),
        case("bridge::length_prefix_is_explicitly_little_endian",
             length_prefix_is_explicitly_little_endian),
        case("bridge::truncated_payload_returns_io_error", truncated_payload_returns_io_error),
        case("bridge::invalid_utf8_payload_errors", invalid_utf8_payload_errors),
        case("bridge::serve_loop_handles_observations_and_feedback_in_one_session",
             serve_loop_handles_observations_and_feedback_in_one_session),
        case("bridge::server_ignores_helloack_from_extension_without_breaking_session",
             server_ignores_helloack_from_extension_without_breaking_session),
        case("bridge::max_message_bytes_matches_chrome_spec", max_message_bytes_matches_chrome_spec),
    ]
}

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

fn frame_round_trip() -> Result<(), String> {
    let msg = BridgeMessage::PushObservations {
        events: vec![sample_obs()],
    };
    let frame = encode(&msg).map_err(|e| format!("encode: {e}"))?;
    let len = u32::from_le_bytes([frame[0], frame[1], frame[2], frame[3]]) as usize;
    check_eq(len, frame.len() - 4, "length-prefix matches body")?;
    let mut cursor = std::io::Cursor::new(frame);
    let back = read_one(&mut cursor)
        .map_err(|e| format!("read: {e}"))?
        .ok_or_else(|| "expected Some".to_string())?;
    check_eq(msg, back, "round-trip identity")
}

fn read_one_returns_none_on_clean_eof() -> Result<(), String> {
    let mut cursor = std::io::Cursor::new(Vec::<u8>::new());
    check(
        read_one(&mut cursor).map_err(|e| format!("{e}"))?.is_none(),
        "clean EOF → None",
    )
}

fn zero_length_frame_is_protocol_violation() -> Result<(), String> {
    let mut cursor = std::io::Cursor::new(vec![0, 0, 0, 0]);
    let err = read_one(&mut cursor).err().ok_or_else(|| "expected error".to_string())?;
    check(matches!(err, BridgeError::ZeroLength), format!("expected ZeroLength, got {err}"))
}

fn oversized_frame_is_rejected() -> Result<(), String> {
    let big = (MAX_MESSAGE_BYTES + 1) as u32;
    let mut bytes = big.to_le_bytes().to_vec();
    bytes.extend(std::iter::repeat(b' ').take(8));
    let mut cursor = std::io::Cursor::new(bytes);
    let err = read_one(&mut cursor).err().ok_or_else(|| "expected error".to_string())?;
    check(matches!(err, BridgeError::TooLarge(_)), format!("expected TooLarge, got {err}"))
}

fn serve_loop_acks_pushed_feedback() -> Result<(), String> {
    let mut input = Vec::new();
    input.extend(
        encode(&BridgeMessage::Hello {
            extension_version: "0.0.0".into(),
        })
        .map_err(|e| format!("{e}"))?,
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
        .map_err(|e| format!("{e}"))?,
    );
    let mut output = Vec::new();
    let mut queue = FeedbackQueue::default();
    serve_native_messaging(std::io::Cursor::new(input), &mut output, &mut queue)
        .map_err(|e| format!("serve: {e}"))?;
    let mut cursor = std::io::Cursor::new(output);
    let m1 = read_one(&mut cursor).map_err(|e| format!("{e}"))?
        .ok_or_else(|| "expected HelloAck".to_string())?;
    check(matches!(m1, BridgeMessage::HelloAck { .. }), "first response is HelloAck")?;
    let m2 = read_one(&mut cursor).map_err(|e| format!("{e}"))?
        .ok_or_else(|| "expected Ack".to_string())?;
    check(matches!(m2, BridgeMessage::Ack { accepted: 1 }), "Ack(1)")?;
    check_eq(queue.len(), 1usize, "queue depth")
}

fn protocol_version_pinned() -> Result<(), String> {
    check_eq(PROTOCOL_VERSION, 1u32, "PROTOCOL_VERSION")
}

fn ack_wire_shape_is_stable() -> Result<(), String> {
    let json = serde_json::to_string(&BridgeMessage::Ack { accepted: 7 })
        .map_err(|e| format!("{e}"))?;
    check_eq(json, r#"{"kind":"ack","accepted":7}"#.to_string(), "ack wire shape")
}

fn hello_wire_shape_is_stable() -> Result<(), String> {
    let json = serde_json::to_string(&BridgeMessage::Hello {
        extension_version: "0.0.0".into(),
    })
    .map_err(|e| format!("{e}"))?;
    check_eq(
        json,
        r#"{"kind":"hello","extension_version":"0.0.0"}"#.to_string(),
        "hello wire shape",
    )
}

fn hello_ack_wire_shape_is_stable() -> Result<(), String> {
    let json = serde_json::to_string(&BridgeMessage::HelloAck {
        host_version: "0.0.0".into(),
        protocol_version: 1,
    })
    .map_err(|e| format!("{e}"))?;
    check_eq(
        json,
        r#"{"kind":"hello_ack","host_version":"0.0.0","protocol_version":1}"#.to_string(),
        "hello_ack wire shape",
    )
}

fn error_wire_shape_is_stable() -> Result<(), String> {
    let json = serde_json::to_string(&BridgeMessage::Error {
        message: "boom".into(),
    })
    .map_err(|e| format!("{e}"))?;
    check_eq(
        json,
        r#"{"kind":"error","message":"boom"}"#.to_string(),
        "error wire shape",
    )
}

fn push_observations_wire_shape_is_stable() -> Result<(), String> {
    let json = serde_json::to_string(&BridgeMessage::PushObservations {
        events: vec![sample_obs()],
    })
    .map_err(|e| format!("{e}"))?;
    check(json.starts_with(r#"{"kind":"push_observations","events":["#),
          "push_observations envelope start")?;
    check(json.ends_with(r#"]}"#), "push_observations envelope end")
}

fn push_feedback_wire_shape_is_stable() -> Result<(), String> {
    let json = serde_json::to_string(&BridgeMessage::PushFeedback {
        events: vec![Feedback {
            field: sample_obs().field,
            predicted: "email".into(),
            actual: "email".into(),
            accepted: true,
        }],
    })
    .map_err(|e| format!("{e}"))?;
    check(json.starts_with(r#"{"kind":"push_feedback","events":["#), "push_feedback envelope start")?;
    check(json.ends_with(r#"]}"#), "push_feedback envelope end")
}

fn length_prefix_is_explicitly_little_endian() -> Result<(), String> {
    let msg = BridgeMessage::Ack { accepted: 0x01020304 };
    let frame = encode(&msg).map_err(|e| format!("{e}"))?;
    let body_len = frame.len() - 4;
    check_eq(frame[0], (body_len & 0xFF) as u8, "byte 0")?;
    check_eq(frame[1], ((body_len >> 8) & 0xFF) as u8, "byte 1")?;
    check_eq(frame[2], ((body_len >> 16) & 0xFF) as u8, "byte 2")?;
    check_eq(frame[3], ((body_len >> 24) & 0xFF) as u8, "byte 3")
}

fn truncated_payload_returns_io_error() -> Result<(), String> {
    let mut bytes = (100u32).to_le_bytes().to_vec();
    bytes.extend_from_slice(b"abcd");
    let mut cursor = std::io::Cursor::new(bytes);
    let err = read_one(&mut cursor).err().ok_or_else(|| "expected error".to_string())?;
    check(matches!(err, BridgeError::Io(_)), format!("expected Io, got {err}"))
}

fn invalid_utf8_payload_errors() -> Result<(), String> {
    let payload: &[u8] = &[0xFF, 0xFE, 0xFD];
    let mut bytes = (payload.len() as u32).to_le_bytes().to_vec();
    bytes.extend_from_slice(payload);
    let mut cursor = std::io::Cursor::new(bytes);
    let err = read_one(&mut cursor).err().ok_or_else(|| "expected error".to_string())?;
    check(matches!(err, BridgeError::Serde(_)), format!("expected Serde, got {err}"))
}

fn serve_loop_handles_observations_and_feedback_in_one_session() -> Result<(), String> {
    let mut input = Vec::new();
    input.extend(
        encode(&BridgeMessage::Hello {
            extension_version: "0.0.0".into(),
        })
        .map_err(|e| format!("{e}"))?,
    );
    input.extend(
        encode(&BridgeMessage::PushObservations {
            events: vec![sample_obs(), sample_obs(), sample_obs()],
        })
        .map_err(|e| format!("{e}"))?,
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
        .map_err(|e| format!("{e}"))?,
    );
    let mut output = Vec::new();
    let mut queue = FeedbackQueue::default();
    serve_native_messaging(std::io::Cursor::new(input), &mut output, &mut queue)
        .map_err(|e| format!("serve: {e}"))?;
    let mut cursor = std::io::Cursor::new(output);
    let m1 = read_one(&mut cursor).map_err(|e| format!("{e}"))?.ok_or_else(|| "m1".to_string())?;
    let m2 = read_one(&mut cursor).map_err(|e| format!("{e}"))?.ok_or_else(|| "m2".to_string())?;
    let m3 = read_one(&mut cursor).map_err(|e| format!("{e}"))?.ok_or_else(|| "m3".to_string())?;
    check(matches!(m1, BridgeMessage::HelloAck { .. }), "m1 HelloAck")?;
    check(matches!(m2, BridgeMessage::Ack { accepted: 3 }), "m2 Ack(3)")?;
    check(matches!(m3, BridgeMessage::Ack { accepted: 1 }), "m3 Ack(1)")?;
    check_eq(queue.len(), 1usize, "queue depth")
}

fn server_ignores_helloack_from_extension_without_breaking_session() -> Result<(), String> {
    let mut input = Vec::new();
    input.extend(
        encode(&BridgeMessage::HelloAck {
            host_version: "weird".into(),
            protocol_version: 1,
        })
        .map_err(|e| format!("{e}"))?,
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
        .map_err(|e| format!("{e}"))?,
    );
    let mut output = Vec::new();
    let mut queue = FeedbackQueue::default();
    serve_native_messaging(std::io::Cursor::new(input), &mut output, &mut queue)
        .map_err(|e| format!("serve: {e}"))?;
    let mut cursor = std::io::Cursor::new(output);
    let m = read_one(&mut cursor).map_err(|e| format!("{e}"))?.ok_or_else(|| "m".to_string())?;
    check(matches!(m, BridgeMessage::Ack { accepted: 1 }), "m Ack(1)")?;
    check_eq(queue.len(), 1usize, "queue depth")
}

fn max_message_bytes_matches_chrome_spec() -> Result<(), String> {
    check_eq(MAX_MESSAGE_BYTES, 1024 * 1024, "MAX_MESSAGE_BYTES = 1 MiB")
}
