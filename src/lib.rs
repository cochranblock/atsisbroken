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
pub mod browser_detect;
pub mod cdp;
pub mod config;
pub mod run_loop;

/// Re-export of kova's ats_fixtures exopack capability. (The `kova-engine`
/// package exposes its lib as `kova`, hence the import name.) Used by
/// `tests/ats_e2e.rs` and (future) the TUI's "diagnose against fixtures"
/// surface. Sources documented in `docs/ATS_FIXTURE_SOURCES.md`.
pub use kova::exopack::ats_fixtures;
pub mod paths;
pub mod resume;
pub mod strategy;
#[cfg(feature = "tui")]
pub mod tui;

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

impl Profile {
    /// True if at least two of the core identity fields are non-empty.
    /// Distinguishes "init was run but the parser found nothing" from
    /// "user has a real profile". Drives `status`'s first-run hint.
    pub fn is_meaningfully_populated(&self) -> bool {
        let signals = [
            !self.full_name.is_empty(),
            !self.email.is_empty(),
            !self.phone.is_empty(),
            !self.linkedin.is_empty(),
            !self.github.is_empty(),
        ];
        signals.iter().filter(|x| **x).count() >= 2
    }
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

impl Mode {
    /// Accept the human-typed CLI spellings. Kept in lib.rs so the
    /// vocabulary is unit-testable without spinning up clap.
    pub fn from_cli_str(s: &str) -> Option<Mode> {
        match s {
            "training-wheels" | "training_wheels" | "training" => Some(Mode::TrainingWheels),
            "shadow" => Some(Mode::Shadow),
            "chaos" => Some(Mode::Chaos),
            _ => None,
        }
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

    /// Read the queue from `path`. A missing file yields an empty queue
    /// (first-run is not an error). A corrupt file is fatal — the user's
    /// training history must not be silently dropped.
    pub fn load_from(path: &std::path::Path) -> std::io::Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::from_jsonl(&text)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e),
        }
    }

    /// Atomically write the queue to `path`. Writes to `<path>.tmp` then
    /// renames; a crash mid-write leaves the previous file intact.
    pub fn save_to(&self, path: &std::path::Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("jsonl.tmp");
        let body = self.to_jsonl().map_err(std::io::Error::other)?;
        std::fs::write(&tmp, body)?;
        std::fs::rename(&tmp, path)
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

/// Cheap keyword classifier — keyword union over label + placeholder +
/// aria_label + name + id. Returns one of the 11 vocabulary entries
/// (the seed corpus's `expected` set) or empty string for unknown.
///
/// This is the Rust mirror of `extension/content.js::predictKey`. Both
/// must agree byte-for-byte on the same input. Tested below.
pub fn predict_field_key(f: &FieldDescriptor) -> &'static str {
    let hay = format!(
        "{} {} {} {} {}",
        f.label, f.placeholder, f.aria_label, f.name, f.id
    )
    .to_lowercase();
    let has = |needle: &str| hay.contains(needle);

    // Strong, vendor-stable signals first (HTML5 input types).
    // We deliberately do NOT use substring "tel" — it appears inside
    // benign words like "websiteLinkedIn", "telegraph", "stelar".
    let kind = f.kind.as_str();
    if kind == "email" || has("email") {
        return "email";
    }
    if kind == "tel" || has("phone") || has("mobile") {
        return "phone";
    }
    // URL fields: route by the most specific platform signal in hay.
    if has("linkedin") {
        return "linkedin";
    }
    if has("github") {
        return "github";
    }
    if has("website") || has("portfolio") {
        return "website";
    }
    if has("address") || has("street") || has("city") || has("zip") {
        return "address";
    }
    if has("authoriz") || has("visa") || has("sponsor") {
        return "work_authorization";
    }
    if (has("year") || has("yrs")) && has("exp") {
        return "years_experience";
    }
    if has("name") {
        return "full_name";
    }
    if kind == "textarea" {
        return "freetext";
    }
    ""
}

/// Resolve a classified key to the value the user has in their profile.
/// Returns `None` for unknown / freetext / fields not in the schema.
pub fn profile_value_for_key<'a>(profile: &'a Profile, key: &str) -> Option<&'a str> {
    let v: &str = match key {
        "full_name" => &profile.full_name,
        "email" => &profile.email,
        "phone" => &profile.phone,
        "address" => &profile.address,
        "linkedin" => &profile.linkedin,
        "github" => &profile.github,
        "website" => &profile.website,
        "work_authorization" => &profile.work_authorization,
        _ => return None,
    };
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
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
    fn seed_corpus_has_no_duplicate_descriptors() {
        // Duplicate (label, placeholder, aria, name, id) tuples bias the
        // model's prior toward whatever key the duplicate happens to
        // declare. Catch it at corpus-edit time.
        use std::collections::HashSet;
        let pairs = parse_seed_corpus().unwrap();
        let mut seen: HashSet<String> = HashSet::new();
        for p in &pairs {
            let key = format!(
                "{}|{}|{}|{}|{}",
                p.field.label, p.field.placeholder, p.field.aria_label, p.field.name, p.field.id
            );
            assert!(
                seen.insert(key.clone()),
                "duplicate descriptor in seed corpus: {key}"
            );
        }
    }

    #[test]
    fn seed_corpus_jsonl_has_no_trailing_or_leading_whitespace_per_line() {
        // JSONL parsers tolerate it but human reviewers don't catch it
        // visually — nail the format.
        for line in SEED_CORPUS_JSONL.lines() {
            if line.is_empty() {
                continue;
            }
            assert_eq!(
                line,
                line.trim(),
                "seed corpus line has whitespace edges: {line:?}"
            );
        }
    }

    #[test]
    fn parse_seed_corpus_propagates_malformed_line_error() {
        // If a seed corpus row ever drifts to invalid JSON, the loader
        // must return Err — silently dropping rows poisons training.
        let original = SEED_CORPUS_JSONL;
        let injected = format!("{original}\n{{this is not json}}\n");
        let result: Result<Vec<TrainingPair>, _> = injected
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(serde_json::from_str)
            .collect();
        assert!(result.is_err());
    }

    #[test]
    fn seed_corpus_covers_at_least_two_kinds() {
        // Quick sanity: corpus exercises more than one HTML input kind.
        let pairs = parse_seed_corpus().unwrap();
        let mut kinds: std::collections::HashSet<String> = Default::default();
        for p in &pairs {
            kinds.insert(p.field.kind.clone());
        }
        assert!(
            kinds.len() >= 2,
            "seed corpus is too monocultural in `kind`: {kinds:?}"
        );
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

    #[test]
    fn is_meaningfully_populated_returns_false_for_empty() {
        assert!(!Profile::default().is_meaningfully_populated());
    }

    #[test]
    fn is_meaningfully_populated_false_with_only_one_field() {
        // E6 finding: "init ran but parser found nothing" should be
        // distinguishable from "real profile". One signal isn't enough.
        let p = Profile {
            full_name: "Jane".into(),
            ..Default::default()
        };
        assert!(!p.is_meaningfully_populated());
    }

    #[test]
    fn is_meaningfully_populated_true_with_two_fields() {
        let p = Profile {
            full_name: "Jane Doe".into(),
            email: "jane@example.com".into(),
            ..Default::default()
        };
        assert!(p.is_meaningfully_populated());
    }

    // ─── predict_field_key ────────────────────────────────────────────────

    fn fd(label: &str, placeholder: &str, aria: &str, name: &str, id: &str, kind: &str) -> FieldDescriptor {
        FieldDescriptor {
            label: label.into(),
            placeholder: placeholder.into(),
            aria_label: aria.into(),
            name: name.into(),
            id: id.into(),
            kind: kind.into(),
        }
    }

    #[test]
    fn predict_email_via_label() {
        assert_eq!(predict_field_key(&fd("Email", "", "", "", "", "email")), "email");
    }
    #[test]
    fn predict_email_via_placeholder() {
        assert_eq!(
            predict_field_key(&fd("", "your.email@example.com", "", "", "", "text")),
            "email"
        );
    }
    #[test]
    fn predict_phone_variants() {
        assert_eq!(predict_field_key(&fd("Mobile phone", "", "", "", "", "tel")), "phone");
        assert_eq!(predict_field_key(&fd("", "", "", "user_phone", "", "tel")), "phone");
        assert_eq!(predict_field_key(&fd("Mobile #", "", "", "", "", "tel")), "phone");
    }
    #[test]
    fn predict_linkedin_github_website() {
        assert_eq!(predict_field_key(&fd("LinkedIn URL", "", "", "", "", "url")), "linkedin");
        assert_eq!(predict_field_key(&fd("GitHub", "", "", "", "", "url")), "github");
        assert_eq!(predict_field_key(&fd("Personal website", "", "", "", "", "url")), "website");
        assert_eq!(predict_field_key(&fd("Portfolio", "", "", "", "", "url")), "website");
    }
    #[test]
    fn predict_address_variants() {
        assert_eq!(predict_field_key(&fd("Street address", "", "", "", "", "text")), "address");
        assert_eq!(predict_field_key(&fd("City", "", "", "", "", "text")), "address");
        assert_eq!(predict_field_key(&fd("Zip code", "", "", "", "", "text")), "address");
    }
    #[test]
    fn predict_work_authorization_variants() {
        assert_eq!(
            predict_field_key(&fd("Are you authorized to work?", "", "", "", "", "select")),
            "work_authorization"
        );
        assert_eq!(
            predict_field_key(&fd("Visa sponsorship required?", "", "", "", "", "select")),
            "work_authorization"
        );
    }
    #[test]
    fn predict_years_experience() {
        assert_eq!(
            predict_field_key(&fd("Years of relevant experience", "", "", "", "", "number")),
            "years_experience"
        );
    }
    #[test]
    fn predict_full_name() {
        assert_eq!(predict_field_key(&fd("First name", "", "", "fname", "", "text")), "full_name");
        assert_eq!(predict_field_key(&fd("Last name", "", "", "", "", "text")), "full_name");
        assert_eq!(predict_field_key(&fd("Full name", "", "", "", "", "text")), "full_name");
    }
    #[test]
    fn predict_textarea_routes_to_freetext() {
        assert_eq!(
            predict_field_key(&fd("Why do you want this role?", "", "", "", "", "textarea")),
            "freetext"
        );
    }
    #[test]
    fn predict_unknown_returns_empty_not_a_guess() {
        assert_eq!(predict_field_key(&fd("Salary expectation", "", "", "", "", "number")), "");
        assert_eq!(predict_field_key(&fd("", "", "", "", "", "text")), "");
    }
    #[test]
    fn predict_does_not_match_tel_inside_unrelated_words() {
        // Regression: ats_e2e.rs caught this — substring "tel" matched
        // inside "websitelinkedin" / Workday's name="websiteLinkedIn".
        // Must classify as linkedin, not phone.
        assert_eq!(
            predict_field_key(&fd(
                "LinkedIn (Optional)",
                "",
                "LinkedIn URL",
                "websiteLinkedIn",
                "wd_lk",
                "url"
            )),
            "linkedin"
        );
    }

    #[test]
    fn predict_phone_via_html_kind_tel() {
        // The semantic input type="tel" is a strong, unambiguous signal.
        let p = fd("", "", "", "", "ph", "tel");
        assert_eq!(predict_field_key(&p), "phone");
    }

    #[test]
    fn predict_email_via_html_kind_email() {
        let e = fd("", "", "", "", "x", "email");
        assert_eq!(predict_field_key(&e), "email");
    }

    #[test]
    fn predict_email_outranks_name_when_both_present() {
        // "name@email.com" placeholder must classify as email even though
        // the label says "Name". User-trust property: email evidence wins.
        assert_eq!(
            predict_field_key(&fd("Contact name", "name@email.com", "", "", "", "text")),
            "email"
        );
    }

    #[test]
    fn profile_value_for_key_empty_field_returns_none() {
        // is_meaningfully_populated guards the empty case at the profile
        // level; profile_value_for_key guards at the field level.
        let p = Profile::default();
        assert!(profile_value_for_key(&p, "email").is_none());
    }

    #[test]
    fn profile_value_for_key_known_keys() {
        let p = Profile {
            full_name: "Jane".into(),
            email: "j@e.com".into(),
            phone: "+1".into(),
            ..Default::default()
        };
        assert_eq!(profile_value_for_key(&p, "full_name"), Some("Jane"));
        assert_eq!(profile_value_for_key(&p, "email"), Some("j@e.com"));
        assert_eq!(profile_value_for_key(&p, "phone"), Some("+1"));
    }

    #[test]
    fn profile_value_for_key_unknown_keys_return_none() {
        let p = Profile { full_name: "Jane".into(), ..Default::default() };
        assert!(profile_value_for_key(&p, "freetext").is_none());
        assert!(profile_value_for_key(&p, "unknown").is_none());
        assert!(profile_value_for_key(&p, "garbage").is_none());
    }

    #[test]
    fn is_meaningfully_populated_counts_only_identity_signals() {
        // Address / years_experience / skills should NOT count toward
        // the threshold — those don't disambiguate first-run from
        // real-profile (a user might have only filled identity).
        let p = Profile {
            address: "1 Main St".into(),
            years_experience: 7,
            skills: vec!["rust".into(), "ml".into()],
            ..Default::default()
        };
        assert!(!p.is_meaningfully_populated());
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

    /// Experience is the deepest nested struct in Profile — pin it.
    #[test]
    fn experience_json_shape_is_stable() {
        let e = Experience {
            company: "Acme Co".into(),
            title: "Engineer III".into(),
            start: "2020-01".into(),
            end: "2024-06".into(),
            bullets: vec!["shipped X".into(), "led Y".into()],
        };
        let got = serde_json::to_string(&e).unwrap();
        let want = r#"{"company":"Acme Co","title":"Engineer III","start":"2020-01","end":"2024-06","bullets":["shipped X","led Y"]}"#;
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
    fn mode_from_cli_str_accepts_all_documented_spellings() {
        // Every spelling that ships in --help has to keep working;
        // accidentally renaming one breaks every script in the wild.
        assert_eq!(Mode::from_cli_str("training-wheels"), Some(Mode::TrainingWheels));
        assert_eq!(Mode::from_cli_str("training_wheels"), Some(Mode::TrainingWheels));
        assert_eq!(Mode::from_cli_str("training"), Some(Mode::TrainingWheels));
        assert_eq!(Mode::from_cli_str("shadow"), Some(Mode::Shadow));
        assert_eq!(Mode::from_cli_str("chaos"), Some(Mode::Chaos));
    }

    #[test]
    fn mode_from_cli_str_rejects_unknown_and_typos() {
        assert_eq!(Mode::from_cli_str(""), None);
        assert_eq!(Mode::from_cli_str("Chaos"), None); // case-sensitive
        assert_eq!(Mode::from_cli_str("training wheels"), None); // space, not dash
        assert_eq!(Mode::from_cli_str("shadows"), None);
        assert_eq!(Mode::from_cli_str("yolo"), None);
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
    fn feedback_queue_load_missing_file_is_empty() {
        let dir = std::env::temp_dir().join(format!(
            "atsisbroken_load_missing_{}",
            std::process::id()
        ));
        let path = dir.join("nope.jsonl");
        let q = FeedbackQueue::load_from(&path).unwrap();
        assert!(q.is_empty());
    }

    #[test]
    fn feedback_queue_save_then_load_round_trip() {
        let dir = std::env::temp_dir().join(format!(
            "atsisbroken_save_load_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("feedback.jsonl");
        let mut q = FeedbackQueue::default();
        q.append(sample_fb("Email", "email", "email", true));
        q.append(sample_fb("Phone", "phone", "phone", true));
        q.save_to(&path).unwrap();
        let back = FeedbackQueue::load_from(&path).unwrap();
        assert_eq!(q, back);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn feedback_queue_preserves_event_order_across_save_load() {
        // Order matters — Feedback events are a temporal stream. The
        // online updater applies them in order; a shuffle inverts the
        // training trajectory.
        let dir = std::env::temp_dir().join(format!(
            "atsisbroken_order_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("feedback.jsonl");
        let mut q = FeedbackQueue::default();
        q.append(sample_fb("first", "email", "email", true));
        q.append(sample_fb("second", "phone", "phone", true));
        q.append(sample_fb("third", "freetext", "skip", false));
        q.save_to(&path).unwrap();
        let back = FeedbackQueue::load_from(&path).unwrap();
        assert_eq!(back.events[0].field.label, "first");
        assert_eq!(back.events[1].field.label, "second");
        assert_eq!(back.events[2].field.label, "third");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn feedback_queue_save_idempotent_overwrites_prior_content() {
        // Saving twice must produce the second state, not append.
        let dir = std::env::temp_dir().join(format!(
            "atsisbroken_overwrite_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("feedback.jsonl");
        let mut q = FeedbackQueue::default();
        q.append(sample_fb("one", "email", "email", true));
        q.save_to(&path).unwrap();
        let mut q2 = FeedbackQueue::default();
        q2.append(sample_fb("two", "phone", "phone", true));
        q2.save_to(&path).unwrap();
        let back = FeedbackQueue::load_from(&path).unwrap();
        assert_eq!(back.len(), 1);
        assert_eq!(back.events[0].field.label, "two");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn feedback_queue_save_atomically_via_tmp() {
        // After save, the .tmp file must not exist (it was renamed).
        let dir = std::env::temp_dir().join(format!(
            "atsisbroken_atomic_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("feedback.jsonl");
        FeedbackQueue::default().save_to(&path).unwrap();
        let tmp = path.with_extension("jsonl.tmp");
        assert!(!tmp.exists(), ".tmp must be renamed away");
        let _ = std::fs::remove_dir_all(&dir);
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
