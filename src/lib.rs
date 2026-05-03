// SPDX-License-Identifier: Unlicense
// Unlicense — public domain — cochranblock.org
// Contributors: GotEmCoach, KOVA, Claude Opus 4.7

//! # atsisbroken
//!
//! Browser autopilot that fills job applications using a model the user
//! trains locally from their own resume + per-field feedback. Drives
//! Chromium via CDP (chromiumoxide). Single Rust binary. Cross-compiled.
//!
//! No open-source models. No cloud. No accounts. No premium tier.
//! No baked third-party weights. The user's model is the only model.
//!
//! ## Modes
//!
//! - [`Mode::TrainingWheels`] — every fill is presented to the user for
//!   yes/no confirmation. Each response becomes a training example. The
//!   classifier learns the user's specific phrasing online.
//! - [`Mode::Chaos`] — the user has graduated. Fills happen autonomously.
//!   The user can still flag mistakes after the fact, generating retroactive
//!   training data.
//!
//! Move from TrainingWheels to Chaos with `atsisbroken graduate`.

use serde::{Deserialize, Serialize};

/// Seed corpus of generic ATS field → key pairs. Compiled into the binary.
/// Bootstrap signal for users who have not yet built up their own labelled
/// data. Augmented at `init` time with pairs derived from the user's resume.
pub const SEED_CORPUS_JSONL: &str = include_str!("../assets/seed-corpus.jsonl");

// ─── Profile schema ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Profile {
    pub full_name: String,
    pub email: String,
    pub phone: String,
    pub address: String,
    pub linkedin: String,
    pub github: String,
    pub website: String,
    pub work_authorization: String,
    pub years_experience: u8,
    pub experience: Vec<Experience>,
    pub education: Vec<Education>,
    pub skills: Vec<String>,
    pub raw_resume_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Experience {
    pub company: String,
    pub title: String,
    pub start: String,
    pub end: String,
    pub bullets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Education {
    pub school: String,
    pub degree: String,
    pub field: String,
    pub start: String,
    pub end: String,
    pub gpa: Option<String>,
}

/// One ATS form field as seen by the DOM extractor. The classifier's input.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FieldDescriptor {
    pub label: String,
    pub placeholder: String,
    pub aria_label: String,
    pub name: String,
    pub id: String,
    pub kind: String,
}

// ─── Modes & feedback ──────────────────────────────────────────────────────

/// User-facing autonomy level. Persisted in `~/.atsisbroken/config.toml`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Every fill prompts the user for yes/no. Online learning on each response.
    TrainingWheels,
    /// Autonomous fills. Post-hoc flagging still trains.
    Chaos,
}

impl Default for Mode {
    fn default() -> Self {
        Mode::TrainingWheels
    }
}

/// One feedback event from a user during TrainingWheels (or a post-hoc flag
/// during Chaos). Appended to the user's training set, used by the online
/// updater to refine the classifier.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Feedback {
    pub field: FieldDescriptor,
    pub predicted: String,
    pub actual: String,
    pub accepted: bool,
}

/// One row in the seed corpus or the user's accumulated training set.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrainingPair {
    pub field: FieldDescriptor,
    pub expected: String,
}

/// Where queued feedback goes when the user runs `atsisbroken sync`.
/// Default is `LocalOnly` — no network ever. The user has to opt in.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FeedbackDelivery {
    /// Feedback stays on disk. `sync` is a no-op.
    LocalOnly,
    /// When `sync` runs and the network is reachable, send the queue.
    /// Destination is free-form: `mailto:you@example.com` or
    /// `https://hook.example.com/feedback`. If offline, queue stays local
    /// and `sync` exits 0.
    SendWhenOnline { destination: String },
}

impl Default for FeedbackDelivery {
    fn default() -> Self {
        FeedbackDelivery::LocalOnly
    }
}

/// Append-only on-disk feedback log. Survives crashes, survives offline.
/// One JSON line per [`Feedback`] event. Drained by `sync` if delivery
/// is configured AND the network is reachable.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeedbackQueue {
    pub events: Vec<Feedback>,
}

impl FeedbackQueue {
    pub fn append(&mut self, fb: Feedback) {
        self.events.push(fb);
    }
    pub fn len(&self) -> usize {
        self.events.len()
    }
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
    /// Serialize as JSONL — one event per line, append-friendly format.
    pub fn to_jsonl(&self) -> Result<String, serde_json::Error> {
        let mut out = String::new();
        for ev in &self.events {
            out.push_str(&serde_json::to_string(ev)?);
            out.push('\n');
        }
        Ok(out)
    }
    /// Parse from JSONL. Empty lines skipped.
    pub fn from_jsonl(text: &str) -> Result<Self, serde_json::Error> {
        let events = text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(serde_json::from_str)
            .collect::<Result<Vec<Feedback>, _>>()?;
        Ok(Self { events })
    }
}

/// Parse the embedded seed corpus into structured training pairs. This
/// runs once at `init` time. Failures are fatal — a corrupt seed corpus
/// is a build-time bug, not a runtime fallback.
pub fn parse_seed_corpus() -> Result<Vec<TrainingPair>, serde_json::Error> {
    SEED_CORPUS_JSONL
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(serde_json::from_str::<TrainingPair>)
        .collect()
}

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn seed_corpus_size() -> usize {
    SEED_CORPUS_JSONL.len()
}

/// Deterministic FNV-1a fingerprint of the embedded seed corpus. Used by
/// the exopack TRIPLE SIMS gate as a single comparable value across runs
/// and across cross-compiled targets. Identical bytes → identical hash.
pub fn seed_corpus_fingerprint() -> u32 {
    let mut hash: u32 = 0x811c9dc5;
    for &b in SEED_CORPUS_JSONL.as_bytes() {
        hash ^= b as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    hash
}

// ─── tests ─────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_corpus_is_nonempty() {
        assert!(seed_corpus_size() > 0);
    }

    #[test]
    fn seed_corpus_parses() {
        let pairs = parse_seed_corpus().expect("seed corpus must parse");
        assert!(pairs.len() >= 20, "seed corpus too small: {}", pairs.len());
        // Every pair must have a non-empty expected key (or the literal
        // sentinels "freetext" / "unknown").
        for p in &pairs {
            assert!(!p.expected.is_empty());
        }
    }

    #[test]
    fn seed_corpus_fingerprint_is_deterministic() {
        let h1 = seed_corpus_fingerprint();
        let h2 = seed_corpus_fingerprint();
        let h3 = seed_corpus_fingerprint();
        assert_eq!(h1, h2);
        assert_eq!(h2, h3);
    }

    #[test]
    fn profile_round_trip() {
        let p = Profile {
            full_name: "Jane Doe".into(),
            email: "jane@example.com".into(),
            phone: "+1-555-0100".into(),
            years_experience: 7,
            skills: vec!["rust".into(), "ml".into()],
            ..Default::default()
        };
        let toml_text = toml::to_string(&p).unwrap();
        let back: Profile = toml::from_str(&toml_text).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn field_descriptor_round_trip() {
        let f = FieldDescriptor {
            label: "Email".into(),
            placeholder: "you@example.com".into(),
            aria_label: "Email address".into(),
            name: "email".into(),
            id: "input-email".into(),
            kind: "email".into(),
        };
        let json = serde_json::to_string(&f).unwrap();
        let back: FieldDescriptor = serde_json::from_str(&json).unwrap();
        assert_eq!(f, back);
    }

    #[test]
    fn mode_default_is_training_wheels() {
        assert_eq!(Mode::default(), Mode::TrainingWheels);
    }

    #[test]
    fn feedback_delivery_default_is_local_only() {
        assert_eq!(FeedbackDelivery::default(), FeedbackDelivery::LocalOnly);
    }

    #[test]
    fn feedback_queue_jsonl_round_trip() {
        let mut q = FeedbackQueue::default();
        q.append(Feedback {
            field: FieldDescriptor {
                label: "Email".into(),
                placeholder: "".into(),
                aria_label: "".into(),
                name: "email".into(),
                id: "".into(),
                kind: "email".into(),
            },
            predicted: "email".into(),
            actual: "email".into(),
            accepted: true,
        });
        q.append(Feedback {
            field: FieldDescriptor {
                label: "Why this role?".into(),
                placeholder: "".into(),
                aria_label: "".into(),
                name: "why".into(),
                id: "".into(),
                kind: "textarea".into(),
            },
            predicted: "freetext".into(),
            actual: "skip".into(),
            accepted: false,
        });
        let jsonl = q.to_jsonl().unwrap();
        let back = FeedbackQueue::from_jsonl(&jsonl).unwrap();
        assert_eq!(q, back);
        assert_eq!(back.len(), 2);
    }

    #[test]
    fn feedback_round_trip() {
        let fb = Feedback {
            field: FieldDescriptor {
                label: "Email".into(),
                placeholder: "".into(),
                aria_label: "".into(),
                name: "email".into(),
                id: "".into(),
                kind: "email".into(),
            },
            predicted: "email".into(),
            actual: "email".into(),
            accepted: true,
        };
        let json = serde_json::to_string(&fb).unwrap();
        let back: Feedback = serde_json::from_str(&json).unwrap();
        assert_eq!(fb, back);
    }

    /// In-process TRIPLE SIMS — the same determinism contract the external
    /// exopack gate enforces, but as a single `cargo test` failure mode.
    #[test]
    fn triple_sims_determinism() {
        let run = || -> (usize, u32, usize, String) {
            let pairs = parse_seed_corpus().unwrap();
            let p = Profile {
                full_name: "Jane Doe".into(),
                email: "jane@example.com".into(),
                ..Default::default()
            };
            (
                seed_corpus_size(),
                seed_corpus_fingerprint(),
                pairs.len(),
                serde_json::to_string(&p).unwrap(),
            )
        };
        let s1 = run();
        let s2 = run();
        let s3 = run();
        assert_eq!(s1, s2);
        assert_eq!(s2, s3);
    }
}
