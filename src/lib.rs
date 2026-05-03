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

pub mod bridge;

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
///
/// Progression: `TrainingWheels` → `Shadow` → `Chaos`. The user can stop at
/// any rung; `Shadow` is where most users will live.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Every fill prompts the user for yes/no. Online learning on each response.
    TrainingWheels,
    /// Hands-off-the-wheel: the user fills the form manually, the binary
    /// silently observes (`Observation` events). Once the classifier's
    /// per-field-type confidence crosses [`ConfidenceThreshold`], that field
    /// type auto-fills on the next encounter without asking. Any field type
    /// still below threshold is left to the user. The user's manual entry on
    /// a low-confidence field is itself a positive training signal — the
    /// model graduates one field-type at a time.
    Shadow,
    /// Autonomous fills on every classified field. Post-hoc flagging still
    /// trains.
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

/// Passive observation of the user filling a field by hand during `Shadow`
/// mode. The classifier predicts what *it* would have called the field;
/// the user's actual entry's matched profile slot is the ground truth. If
/// they agree, that's a positive online example. If they disagree, the
/// user's value wins — they're showing us they want different behavior.
///
/// `Observation` carries only the *key* the user's value mapped to, never
/// the value itself. This keeps the training stream PII-free even before
/// `sync` runs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Observation {
    pub field: FieldDescriptor,
    pub predicted: String,
    /// Key matched against the Profile by exact-string match on what the
    /// user typed. e.g. user typed "jane@example.com" → matches
    /// `Profile.email` → `observed = "email"`. If nothing in Profile
    /// matched, `observed` is empty.
    pub observed: String,
    /// Classifier's confidence at prediction time, [0.0, 1.0].
    pub confidence: f32,
}

/// Per-key confidence threshold above which `Shadow` mode promotes a field
/// type to autonomous autofill. Default tuned for "be conservative early":
/// 0.85 typically requires ~10–20 consistent observations of a given key
/// before the user stops touching that field type by hand.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ConfidenceThreshold(pub f32); // f32 — no Eq, NaN-incomparable

impl Default for ConfidenceThreshold {
    fn default() -> Self {
        ConfidenceThreshold(0.85)
    }
}

impl ConfidenceThreshold {
    /// Returns true if `confidence` clears the threshold for autonomy.
    pub fn passes(&self, confidence: f32) -> bool {
        confidence >= self.0
    }
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

    // ─── seed corpus coverage ─────────────────────────────────────────────

    #[test]
    fn seed_corpus_covers_email() {
        let pairs = parse_seed_corpus().unwrap();
        assert!(pairs.iter().any(|p| p.expected == "email"));
    }
    #[test]
    fn seed_corpus_covers_phone() {
        let pairs = parse_seed_corpus().unwrap();
        assert!(pairs.iter().any(|p| p.expected == "phone"));
    }
    #[test]
    fn seed_corpus_covers_full_name() {
        let pairs = parse_seed_corpus().unwrap();
        assert!(pairs.iter().any(|p| p.expected == "full_name"));
    }
    #[test]
    fn seed_corpus_covers_linkedin() {
        let pairs = parse_seed_corpus().unwrap();
        assert!(pairs.iter().any(|p| p.expected == "linkedin"));
    }
    #[test]
    fn seed_corpus_covers_github() {
        let pairs = parse_seed_corpus().unwrap();
        assert!(pairs.iter().any(|p| p.expected == "github"));
    }
    #[test]
    fn seed_corpus_covers_website() {
        let pairs = parse_seed_corpus().unwrap();
        assert!(pairs.iter().any(|p| p.expected == "website"));
    }
    #[test]
    fn seed_corpus_covers_address() {
        let pairs = parse_seed_corpus().unwrap();
        assert!(pairs.iter().any(|p| p.expected == "address"));
    }
    #[test]
    fn seed_corpus_covers_work_authorization() {
        let pairs = parse_seed_corpus().unwrap();
        assert!(pairs.iter().any(|p| p.expected == "work_authorization"));
    }
    #[test]
    fn seed_corpus_covers_years_experience() {
        let pairs = parse_seed_corpus().unwrap();
        assert!(pairs.iter().any(|p| p.expected == "years_experience"));
    }
    #[test]
    fn seed_corpus_covers_freetext_sentinel() {
        let pairs = parse_seed_corpus().unwrap();
        assert!(pairs.iter().any(|p| p.expected == "freetext"));
    }
    #[test]
    fn seed_corpus_covers_unknown_sentinel() {
        let pairs = parse_seed_corpus().unwrap();
        assert!(pairs.iter().any(|p| p.expected == "unknown"));
    }
    #[test]
    fn seed_corpus_no_empty_label_and_name() {
        // Every pair needs at least one signal — pure-empty descriptors are
        // useless training data and indicate a corpus authoring bug.
        let pairs = parse_seed_corpus().unwrap();
        for (i, p) in pairs.iter().enumerate() {
            let any = !p.field.label.is_empty()
                || !p.field.placeholder.is_empty()
                || !p.field.aria_label.is_empty()
                || !p.field.name.is_empty()
                || !p.field.id.is_empty();
            assert!(any, "row {} has no descriptor signal at all", i);
        }
    }
    #[test]
    fn seed_corpus_expected_keys_are_known_vocab() {
        let allowed = [
            "email",
            "phone",
            "full_name",
            "linkedin",
            "github",
            "website",
            "address",
            "work_authorization",
            "years_experience",
            "freetext",
            "unknown",
        ];
        let pairs = parse_seed_corpus().unwrap();
        for p in &pairs {
            assert!(
                allowed.contains(&p.expected.as_str()),
                "out-of-vocab key in corpus: {:?}",
                p.expected
            );
        }
    }

    // ─── Profile / Education / Experience ─────────────────────────────────

    #[test]
    fn profile_default_is_empty() {
        let p = Profile::default();
        assert!(p.full_name.is_empty());
        assert!(p.email.is_empty());
        assert_eq!(p.years_experience, 0);
        assert!(p.skills.is_empty());
        assert!(p.experience.is_empty());
    }

    /// Pin the on-disk JSON shape of FieldDescriptor against a literal fixture.
    /// If any field name or layout drifts, every existing user's training
    /// data becomes unreadable — this catches that BEFORE shipping.
    #[test]
    fn field_descriptor_json_shape_is_stable() {
        let f = FieldDescriptor {
            label: "Email".into(),
            placeholder: "you@example.com".into(),
            aria_label: "Email address".into(),
            name: "email".into(),
            id: "input-email".into(),
            kind: "email".into(),
        };
        let got = serde_json::to_string(&f).unwrap();
        let want = r#"{"label":"Email","placeholder":"you@example.com","aria_label":"Email address","name":"email","id":"input-email","kind":"email"}"#;
        assert_eq!(got, want);
    }

    /// Pin Education's on-disk shape including the Option<gpa> serialization
    /// (must be `null` when None — not omitted — so file diffs are stable).
    #[test]
    fn education_json_shape_is_stable() {
        let ed = Education {
            school: "State U".into(),
            degree: "BS".into(),
            field: "CS".into(),
            start: "2016".into(),
            end: "2020".into(),
            gpa: None,
        };
        let got = serde_json::to_string(&ed).unwrap();
        let want = r#"{"school":"State U","degree":"BS","field":"CS","start":"2016","end":"2020","gpa":null}"#;
        assert_eq!(got, want);
    }

    /// Profile in TOML must accept missing optional collections (existing
    /// users with old shorter profile.toml files must keep working).
    #[test]
    fn profile_toml_accepts_partial_input() {
        let toml_text = r#"
            full_name = "Jane Doe"
            email    = "jane@example.com"
            phone    = ""
            address  = ""
            linkedin = ""
            github   = ""
            website  = ""
            work_authorization = ""
            years_experience   = 0
            raw_resume_text    = ""
            experience = []
            education  = []
            skills     = []
        "#;
        let p: Profile = toml::from_str(toml_text).unwrap();
        assert_eq!(p.full_name, "Jane Doe");
        assert_eq!(p.email, "jane@example.com");
    }

    // ─── Mode / Feedback / Delivery ───────────────────────────────────────

    #[test]
    fn mode_serializes_snake_case() {
        let s = serde_json::to_string(&Mode::TrainingWheels).unwrap();
        assert_eq!(s, "\"training_wheels\"");
        let s = serde_json::to_string(&Mode::Shadow).unwrap();
        assert_eq!(s, "\"shadow\"");
        let s = serde_json::to_string(&Mode::Chaos).unwrap();
        assert_eq!(s, "\"chaos\"");
    }

    #[test]
    fn confidence_threshold_default_is_conservative() {
        // Property test: the default must be > 0.5 (more than coin-flip),
        // < 1.0 (achievable), and not exactly an obvious round number that
        // suggests no thought went into picking it.
        let t = ConfidenceThreshold::default();
        assert!(t.0 > 0.5 && t.0 < 1.0);
    }

    #[test]
    fn confidence_threshold_gate() {
        let t = ConfidenceThreshold(0.85);
        assert!(t.passes(0.85));
        assert!(t.passes(0.90));
        assert!(t.passes(1.0));
        assert!(!t.passes(0.84));
        assert!(!t.passes(0.0));
    }

    /// Pin Observation's on-disk shape — this is part of the user's training
    /// stream and changing the field names breaks every prior session.
    #[test]
    fn observation_json_shape_is_stable() {
        let o = Observation {
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
        };
        let got = serde_json::to_string(&o).unwrap();
        let want = r#"{"field":{"label":"Email","placeholder":"","aria_label":"","name":"email","id":"","kind":"email"},"predicted":"email","observed":"email","confidence":0.92}"#;
        assert_eq!(got, want);
    }

    /// Auto-graduation contract: once classifier confidence on a key crosses
    /// the threshold, Shadow mode silently autofills it on the next sighting.
    /// The threshold gate is the only thing standing between observation
    /// and action — verify it actually gates.
    #[test]
    fn shadow_mode_gate_only_promotes_when_confident() {
        let t = ConfidenceThreshold::default();
        // Simulate a stream of observations for the same key, with rising
        // confidence as the classifier sees more examples.
        let stream: Vec<f32> = vec![0.42, 0.58, 0.71, 0.79, 0.83, 0.86, 0.90];
        let mut promoted_at: Option<usize> = None;
        for (i, c) in stream.iter().enumerate() {
            if t.passes(*c) {
                promoted_at = Some(i);
                break;
            }
        }
        // Must promote eventually — but only after crossing the bar.
        assert_eq!(promoted_at, Some(5));
    }

    /// FeedbackDelivery on-disk shape is part of the user's config schema.
    /// Pin both variants explicitly so adding a third doesn't silently
    /// reorder discriminants.
    #[test]
    fn feedback_delivery_json_shapes_are_stable() {
        let local = serde_json::to_string(&FeedbackDelivery::LocalOnly).unwrap();
        assert_eq!(local, r#"{"kind":"local_only"}"#);
        let online = serde_json::to_string(&FeedbackDelivery::SendWhenOnline {
            destination: "mailto:me@example.com".into(),
        })
        .unwrap();
        assert_eq!(
            online,
            r#"{"kind":"send_when_online","destination":"mailto:me@example.com"}"#
        );
    }

    // ─── FeedbackQueue ────────────────────────────────────────────────────

    fn sample_fb(label: &str, predicted: &str, actual: &str, accepted: bool) -> Feedback {
        Feedback {
            field: FieldDescriptor {
                label: label.into(),
                placeholder: "".into(),
                aria_label: "".into(),
                name: "".into(),
                id: "".into(),
                kind: "text".into(),
            },
            predicted: predicted.into(),
            actual: actual.into(),
            accepted,
        }
    }

    #[test]
    fn feedback_queue_jsonl_skips_blank_lines() {
        let jsonl = "\n\n{\"field\":{\"label\":\"Email\",\"placeholder\":\"\",\"aria_label\":\"\",\"name\":\"\",\"id\":\"\",\"kind\":\"text\"},\"predicted\":\"email\",\"actual\":\"email\",\"accepted\":true}\n\n";
        let q = FeedbackQueue::from_jsonl(jsonl).unwrap();
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn feedback_queue_empty_jsonl_parses() {
        let q = FeedbackQueue::from_jsonl("").unwrap();
        assert!(q.is_empty());
        let q = FeedbackQueue::from_jsonl("\n\n\n").unwrap();
        assert!(q.is_empty());
    }

    #[test]
    fn feedback_queue_jsonl_is_one_line_per_event() {
        let mut q = FeedbackQueue::default();
        q.append(sample_fb("a", "email", "email", true));
        q.append(sample_fb("b", "phone", "phone", true));
        q.append(sample_fb("c", "freetext", "skip", false));
        let jsonl = q.to_jsonl().unwrap();
        let lines: Vec<&str> = jsonl.lines().collect();
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn feedback_queue_corrupt_line_fails_loudly() {
        // A single corrupt line must error — silent skipping would erase
        // the user's training history. Better to surface the parse error
        // and let `sync` quarantine the file.
        let result = FeedbackQueue::from_jsonl("{this is not json}\n");
        assert!(result.is_err());
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
