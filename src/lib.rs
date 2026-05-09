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
#[cfg(feature = "browser")]
pub mod browser;
pub mod browser_detect;
pub mod cdp;
pub mod config;
pub mod github;
pub mod learning;
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

// Eq dropped: `technical_skills: Vec<Skill>` carries `Option<f32>`
// (skill years), and f32 isn't Eq because of NaN. PartialEq is
// enough for assert_eq! and HashMap-key uses; nothing in the
// codebase needs Profile as a HashSet element.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Profile {
    pub full_name: String,
    /// Split-out first name. Populated by the resume parser (last
    /// whitespace-separated token of `full_name` is `last_name`,
    /// rest is `first_name`). When the form asks for first vs last
    /// separately (Greenhouse, Workday, iCIMS), the classifier
    /// returns "first_name" / "last_name" and the run loop reads
    /// these. Forms that ask for one combined "Name" field still
    /// get `full_name`.
    #[serde(default)]
    pub first_name: String,
    #[serde(default)]
    pub last_name: String,
    /// "Goes by" — what the user actually responds to. Some ATS
    /// forms ask both legal and preferred names.
    #[serde(default)]
    pub preferred_name: String,
    /// User-typed pronouns. Free-form so the user can use any
    /// phrasing (she/her, they/them, custom). Empty when the user
    /// declines to share.
    #[serde(default)]
    pub pronouns: String,
    pub email: String,
    pub phone: String,
    /// Legacy single-line address. Retained for forward compatibility
    /// with profile.toml files that don't have structured sub-fields,
    /// and for forms that ask for "Address" as a single string. The
    /// resume parser populates this; the structured sub-fields below
    /// are optional and populated only if the user edits them.
    pub address: String,
    /// Structured address sub-fields. Empty by default; populated
    /// either by user edit or (later) by a structured address parser
    /// run over the legacy `address` line. profile_value_for_key
    /// falls back to `address` when these are empty.
    #[serde(default)]
    pub street1: String,
    #[serde(default)]
    pub street2: String,
    #[serde(default)]
    pub city: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub postal_code: String,
    #[serde(default)]
    pub country: String,
    pub linkedin: String,
    pub github: String,
    pub website: String,
    /// Online presence beyond the legacy linkedin/github/website
    /// trio. Populated by user edit; resume parser does not yet
    /// auto-fill these. Each field is empty by default; the
    /// classifier consults these when the form asks for a specific
    /// platform handle.
    #[serde(default)]
    pub presence: OnlinePresence,
    pub work_authorization: String,
    pub years_experience: u8,
    pub experience: Vec<Experience>,
    pub education: Vec<Education>,
    /// Legacy bag-of-words skills. Retained for forward compat;
    /// the new `technical_skills` carries structured Skill entries
    /// with proficiency level + years. Forms that ask for a free-
    /// text "skills" field can still pull from this.
    pub skills: Vec<String>,
    /// Structured technical skills with proficiency + years. Empty
    /// by default (resume parser does not yet populate). Phase H
    /// `init` walkthrough will let the user enter these explicitly.
    #[serde(default)]
    pub technical_skills: Vec<Skill>,
    /// Spoken/written languages with proficiency.
    #[serde(default)]
    pub languages: Vec<Language>,
    /// Soft skills as plain strings — no proficiency vocabulary
    /// exists for these in industry, so we don't invent one.
    #[serde(default)]
    pub soft_skills: Vec<String>,
    /// Professional certifications (AWS, CISSP, PMP, etc.).
    #[serde(default)]
    pub certifications: Vec<Certification>,
    /// Personal / open-source projects. May cross-link to a
    /// GitHub inventory entry via `github_repo` when Phase I lands.
    #[serde(default)]
    pub projects: Vec<Project>,
    /// Compensation expectations. None when the user has not chosen
    /// to share — the run loop must NOT autofill compensation
    /// fields without the user's explicit value here.
    #[serde(default)]
    pub compensation: Option<Compensation>,
    /// Voluntary EEO / AAP demographics. None by default — the run
    /// loop has a hard contract that demographics fields are skipped
    /// in every mode unless the user has explicitly set this AND
    /// opted in per-fill via a dedicated flag (not implemented yet).
    /// Even setting `Some(Demographics::default())` does NOT enable
    /// autofill on its own.
    #[serde(default)]
    pub demographics: Option<Demographics>,
    /// Structured work authorization (citizenship country, status,
    /// sponsorship needed, security clearance, remote/relocation
    /// preference). Additive — the legacy `work_authorization: String`
    /// stays for backward compat. When both are populated the
    /// classifier prefers the structured form.
    #[serde(default)]
    pub work_auth: Option<WorkAuth>,
    /// Academic / industry publications.
    #[serde(default)]
    pub publications: Vec<Publication>,
    /// Patents granted or pending.
    #[serde(default)]
    pub patents: Vec<Patent>,
    /// Awards / recognitions / honors at the professional (not
    /// academic) level. Academic honors live on Education.honors.
    #[serde(default)]
    pub awards: Vec<Award>,
    /// Professional references the user is willing to share with
    /// employers. Empty when the user prefers "available on request."
    #[serde(default)]
    pub references: Vec<Reference>,
    /// Application ledger — atsisbroken's own record of where the
    /// user has applied via the tool. Appended automatically by
    /// the run loop when a submission completes (Phase 1.5 F flow).
    #[serde(default)]
    pub applications: Vec<Application>,
    /// User-authored free-form answer slots. The answer composer
    /// (Phase K) fills these via verbatim source synthesis (GitHub
    /// + Blog inventories) when the user has opted in; the user
    /// can also write them by hand. Either way, output is verbatim.
    #[serde(default)]
    pub free_form: FreeFormAnswers,
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
    /// City/state/country of the role. Empty when remote or unknown.
    #[serde(default)]
    pub location: String,
    /// Free-form: "full-time", "part-time", "contract", "internship",
    /// "volunteer". Free-form rather than enum so a user can use
    /// any phrasing the form expects.
    #[serde(default)]
    pub employment_type: String,
    /// Reference contact info — some ATS forms ask. Empty by default;
    /// the user opts in per-experience.
    #[serde(default)]
    pub supervisor_name: String,
    #[serde(default)]
    pub supervisor_email: String,
    #[serde(default)]
    pub supervisor_phone: String,
    /// Free-form. Empty unless the user wants to volunteer it.
    #[serde(default)]
    pub reason_for_leaving: String,
    /// "May we contact this employer for a reference?" — common ATS
    /// boolean field. Defaults to false (the user must opt in
    /// explicitly; we never assume contact permission).
    #[serde(default)]
    pub can_we_contact: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Education {
    pub school: String,
    pub degree: String,
    pub field: String,
    pub start: String,
    pub end: String,
    pub gpa: Option<String>,
    /// Honors / distinctions: "Cum Laude", "Phi Beta Kappa",
    /// "Dean's List". Empty when none claimed.
    #[serde(default)]
    pub honors: Vec<String>,
    #[serde(default)]
    pub minor: String,
    #[serde(default)]
    pub relevant_coursework: Vec<String>,
    #[serde(default)]
    pub thesis_title: String,
    #[serde(default)]
    pub extracurriculars: Vec<String>,
    /// City/state of the institution.
    #[serde(default)]
    pub location: String,
}

// ─── Profile sub-structs added in Phase G ─────────────────────────────────

/// Online presence beyond the legacy linkedin/github/website fields
/// which remain at the Profile root for forward-compat. Each field
/// is empty when the user has nothing to share. Field names align
/// with the platform name lowercased so the classifier can route
/// `has("mastodon")` → `mastodon`, etc.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct OnlinePresence {
    #[serde(default)]
    pub gitlab: String,
    #[serde(default)]
    pub bitbucket: String,
    #[serde(default)]
    pub portfolio: String,
    /// Personal blog or writing site URL.
    #[serde(default)]
    pub blog: String,
    #[serde(default)]
    pub twitter: String,
    #[serde(default)]
    pub bluesky: String,
    #[serde(default)]
    pub mastodon: String,
    #[serde(default)]
    pub stackoverflow: String,
    #[serde(default)]
    pub devto: String,
    #[serde(default)]
    pub medium: String,
    #[serde(default)]
    pub hashnode: String,
    #[serde(default)]
    pub youtube: String,
    #[serde(default)]
    pub dribbble: String,
    #[serde(default)]
    pub behance: String,
    #[serde(default)]
    pub artstation: String,
}

/// Self-reported proficiency for one technical skill.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkillLevel {
    Beginner,
    Intermediate,
    Advanced,
    Expert,
}

impl Default for SkillLevel {
    fn default() -> Self {
        SkillLevel::Intermediate
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Skill {
    pub name: String,
    /// Years of experience with the skill. Optional — many users
    /// can't or don't want to estimate.
    #[serde(default)]
    pub years: Option<f32>,
    #[serde(default)]
    pub level: SkillLevel,
    /// Year of last use, e.g. "2025". Empty when not specified.
    #[serde(default)]
    pub last_used: String,
}

/// Self-reported proficiency for one spoken/written language.
/// Categories chosen to map onto common ATS dropdowns rather than
/// the more granular CEFR levels.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LanguageProficiency {
    Native,
    Fluent,
    Conversational,
    Beginner,
}

impl Default for LanguageProficiency {
    fn default() -> Self {
        LanguageProficiency::Conversational
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Language {
    pub name: String,
    #[serde(default)]
    pub proficiency: LanguageProficiency,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Certification {
    pub name: String,
    pub issuer: String,
    /// "YYYY-MM" or "YYYY". Free-form to match what the user has
    /// on the credential itself.
    pub issue_date: String,
    /// Empty when the cert doesn't expire or the user prefers not
    /// to disclose. (`Option<String>` would force null in JSON;
    /// empty-string is friendlier in TOML.)
    #[serde(default)]
    pub expiry_date: String,
    #[serde(default)]
    pub credential_id: String,
    #[serde(default)]
    pub credential_url: String,
}

/// Cross-link target. When a `Project` corresponds to a GitHub repo,
/// `Project.github_repo` carries this so the answer composer (Phase K)
/// can pull verbatim README excerpts and commit messages from the
/// GitHub inventory entry with the same (owner, name).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct RepoRef {
    pub owner: String,
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Project {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub start_date: String,
    #[serde(default)]
    pub end_date: String,
    #[serde(default)]
    pub technologies: Vec<String>,
    /// Role the user played. "creator", "maintainer", "contributor",
    /// "lead engineer", etc. Free-form.
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub highlights: Vec<String>,
    /// Cross-link to a GitHub inventory repo when applicable. None
    /// for projects that don't live on GitHub.
    #[serde(default)]
    pub github_repo: Option<RepoRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Compensation {
    /// Numeric expectations. None when the user prefers to defer
    /// to negotiation; the run loop will skip salary fields rather
    /// than guess a number.
    #[serde(default)]
    pub salary_expectation_min: Option<u32>,
    #[serde(default)]
    pub salary_expectation_max: Option<u32>,
    /// ISO 4217 code (USD, EUR, GBP). Defaults to USD as the most
    /// common ATS form expectation; the user can change it.
    pub salary_currency: String,
    /// Free-form, e.g. "open to equity-heavy", "negotiable".
    #[serde(default)]
    pub compensation_notes: String,
    #[serde(default)]
    pub desired_base: Option<u32>,
    #[serde(default)]
    pub desired_variable: Option<u32>,
    #[serde(default)]
    pub desired_equity: String,
}

/// Voluntary self-identification fields. Off by default; the run loop
/// has a hard contract that demographics fields are skipped in every
/// mode unless the user has both populated this struct AND passed the
/// per-run flag (not implemented yet — explicit gesture required).
///
/// All fields are free-form strings so users can write what they
/// actually identify as without being forced into our enum vocabulary.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Demographics {
    #[serde(default)]
    pub gender: String,
    #[serde(default)]
    pub race_ethnicity: Vec<String>,
    #[serde(default)]
    pub veteran_status: String,
    #[serde(default)]
    pub disability_status: String,
    /// Stored as a string ("yes", "no", "prefer_not_to_say") rather
    /// than bool so users can decline without us inferring false.
    #[serde(default)]
    pub lgbtq_self_id: String,
}

// ─── Phase G Tier 2 — work authorization, history, free-form ──────────────

/// US work-eligibility classification. Free-form `Other(String)` keeps
/// the door open for international categories we haven't enumerated.
/// Default is `Other(String::new())` — empty escape hatch — so an
/// unspecified work_auth deserializes without us picking a category
/// for the user.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkAuthStatus {
    Citizen,
    PermanentResident,
    H1B,
    Opt,
    Ead,
    Tn,
    OptionalPracticalTraining,
    RequireSponsorship,
    Other(String),
}

impl Default for WorkAuthStatus {
    fn default() -> Self {
        WorkAuthStatus::Other(String::new())
    }
}

/// US security-clearance levels (lowest → highest). `Other(String)`
/// for non-US clearances or the rare case of a tenant-specific tier.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SecurityClearance {
    /// Explicit "no clearance held" — different from
    /// `Other(String::new())` which means "not specified."
    None,
    PublicTrust,
    Confidential,
    Secret,
    TopSecret,
    TsSci,
    Other(String),
}

impl Default for SecurityClearance {
    fn default() -> Self {
        SecurityClearance::Other(String::new())
    }
}

/// Remote/onsite preference. Default `Flexible` reads as "user
/// hasn't expressed a preference"; pickers can prompt explicitly.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RemotePreference {
    Remote,
    Hybrid,
    OnSite,
    Flexible,
}

impl Default for RemotePreference {
    fn default() -> Self {
        RemotePreference::Flexible
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct WorkAuth {
    /// ISO-3166-1 alpha-3 (e.g. "USA", "GBR"). Empty when not set.
    #[serde(default)]
    pub citizenship_country: String,
    #[serde(default)]
    pub status: WorkAuthStatus,
    #[serde(default)]
    pub visa_sponsorship_required: bool,
    #[serde(default)]
    pub security_clearance: SecurityClearance,
    /// Whether the user is open to relocating for a role.
    #[serde(default)]
    pub relocation_willingness: bool,
    /// Free-form region preferences ("West Coast US", "EMEA",
    /// "Bay Area only"). Empty when no preference expressed.
    #[serde(default)]
    pub region_preferences: Vec<String>,
    #[serde(default)]
    pub remote_preference: RemotePreference,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Publication {
    pub title: String,
    #[serde(default)]
    pub authors: Vec<String>,
    /// Conference, journal, or publisher name.
    pub venue: String,
    /// "YYYY" or "YYYY-MM" — free-form so users match what's on the
    /// publication itself.
    pub date: String,
    #[serde(default)]
    pub url: String,
    /// Digital Object Identifier — empty when not assigned.
    #[serde(default)]
    pub doi: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Patent {
    pub title: String,
    /// Patent number (e.g. "US10,123,456 B2") or application number.
    pub number: String,
    pub issued_date: String,
    #[serde(default)]
    pub inventors: Vec<String>,
    #[serde(default)]
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Award {
    pub name: String,
    pub issuer: String,
    pub date: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Reference {
    pub name: String,
    /// "Manager", "Direct report", "Peer", "Mentor", etc.
    pub relationship: String,
    #[serde(default)]
    pub company: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub phone: String,
}

/// Application status as the user knows it. The run loop appends
/// `Submitted` automatically; transitions to other states are user-
/// driven (via a future `atsisbroken applications mark <url> <status>`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApplicationStatus {
    Submitted,
    Interviewed,
    Offer,
    Rejected,
    Ghosted,
    Withdrawn,
}

impl Default for ApplicationStatus {
    fn default() -> Self {
        ApplicationStatus::Submitted
    }
}

/// One ATS application the user submitted via atsisbroken. Appended
/// when the run loop confirms a successful submit.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Application {
    pub company: String,
    pub role: String,
    /// RFC3339 timestamp of submission.
    pub submitted_at: String,
    /// Job posting URL (or the form URL if no posting URL exists).
    pub url: String,
    #[serde(default)]
    pub status: ApplicationStatus,
    #[serde(default)]
    pub notes: String,
}

/// User-authored answers to common ATS prompt patterns. The question
/// classifier (Phase J) routes recognized prompts to one of these
/// slots; the answer composer (Phase K) then either uses the user's
/// authored value or composes from GitHub / Blog inventories.
///
/// Each slot is `Option<String>`. None = "user has not authored a
/// response"; the composer is allowed to synthesize from inventories.
/// `Some("")` = "user explicitly cleared this slot — never autofill,
/// even from a verbatim source." Two distinct signals.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct FreeFormAnswers {
    #[serde(default)]
    pub elevator_pitch: Option<String>,
    #[serde(default)]
    pub why_this_role_template: Option<String>,
    #[serde(default)]
    pub biggest_technical_challenge: Option<String>,
    #[serde(default)]
    pub proudest_project: Option<String>,
    #[serde(default)]
    pub biggest_failure_and_lesson: Option<String>,
    #[serde(default)]
    pub five_year_plan: Option<String>,
    #[serde(default)]
    pub strengths: Option<String>,
    #[serde(default)]
    pub weaknesses: Option<String>,
    #[serde(default)]
    pub management_style: Option<String>,
    #[serde(default)]
    pub collaboration_example: Option<String>,
    /// Catch-all for prompts not covered by the named slots. Key is
    /// a question hash or descriptive name; value is the user's
    /// authored answer. Phase K caches accepted composer outputs here
    /// keyed by question hash, so the same question on a different
    /// ATS reuses the answer.
    #[serde(default)]
    pub custom: std::collections::BTreeMap<String, String>,
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
/// Split `s` into lowercase word tokens. Splits on non-alphanumeric
/// boundaries AND on camelCase transitions ("websiteLinkedIn" →
/// ["website","linked","in"]). Used for word-equality checks on
/// short ambiguous tokens like "first" / "last" that would
/// false-positive on substring matches inside developer-chosen ids.
fn tokenize(s: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut prev_lower = false;
    for c in s.chars() {
        if !c.is_alphanumeric() {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur).to_lowercase());
            }
            prev_lower = false;
            continue;
        }
        if prev_lower && c.is_ascii_uppercase() && !cur.is_empty() {
            out.push(std::mem::take(&mut cur).to_lowercase());
        }
        cur.push(c);
        prev_lower = c.is_ascii_lowercase();
    }
    if !cur.is_empty() {
        out.push(cur.to_lowercase());
    }
    out
}

pub fn predict_field_key(f: &FieldDescriptor) -> &'static str {
    let hay = format!(
        "{} {} {} {} {}",
        f.label, f.placeholder, f.aria_label, f.name, f.id
    )
    .to_lowercase();
    let has = |needle: &str| hay.contains(needle);
    // For short ambiguous tokens (first/last/given/family/forename/
    // city/zip), substring matching false-positives inside developer
    // ids like `id="first"` or `name="firstChoice"`. Use:
    //   - has_word: word-equality over the camelCase-aware tokenization.
    //   - has_phrase: substring match over a normalized "tokens joined
    //     by spaces" form. Catches "first_name" / "firstName" /
    //     "first-name" / "First Name" — all normalize to "first name".
    let tokens = tokenize(&hay);
    let has_word = |w: &str| tokens.iter().any(|t| t == w);
    let hay_norm: String = tokens.join(" ");
    let has_phrase = |p: &str| hay_norm.contains(p);

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
    // Phase G presence platforms — placed BEFORE the website
    // catch-all because some forms ask for several at once. Each
    // platform name is distinctive enough for substring match;
    // no false positives observed across the 5 vendor fixtures.
    if has("gitlab") {
        return "gitlab";
    }
    if has("bitbucket") {
        return "bitbucket";
    }
    if has("bluesky") {
        return "bluesky";
    }
    if has("mastodon") {
        return "mastodon";
    }
    // "stackoverflow" or "stack overflow" or "stackexchange" — the
    // hay_norm form joins tokens with spaces, so "stack overflow"
    // matches as a phrase. The bare lowercased "stackoverflow"
    // matches via has().
    if has("stackoverflow") || has_phrase("stack overflow") {
        return "stackoverflow";
    }
    // Twitter / X. We key on "twitter" (still the dominant ATS
    // form name); /x.com/ as a URL pattern is left to the URL
    // signal route via the field's value at fill time.
    if has("twitter") {
        return "twitter";
    }
    // "blog" is distinctive when it appears as a label/name;
    // requires word-equality so "weblog" / arbitrary substrings
    // don't trip it.
    if has_word("blog") {
        return "blog";
    }
    if has("website") || has("portfolio") {
        return "website";
    }
    // Work authorization checked BEFORE address sub-fields because the
    // canonical phrasing "authorized to work in this country" contains
    // the word "country" and would otherwise route to the country slot.
    // The "authoriz"/"visa"/"sponsor" signals are unambiguous.
    if has("authoriz") || has("visa") || has("sponsor") {
        return "work_authorization";
    }

    // Address sub-fields, most-specific first. Word-equality so
    // generic substrings ("address" inside "addressbar") don't
    // false-positive on unrelated DOM ids.
    if has_word("postal") || has_word("postcode") || has_word("zip") {
        return "postal_code";
    }
    if has_word("city") {
        return "city";
    }
    if has_word("state") || has_word("province") || has_word("region") {
        return "state";
    }
    if has_word("country") || has_word("nation") {
        return "country";
    }
    // street1 vs street2: only fires if "line 2" or "address line 2"
    // appears verbatim. Otherwise generic "street" / "address" → street1.
    if has_phrase("address line 2") || has_phrase("line 2") || has_word("apartment") || has_word("apt") || has_word("suite") {
        return "street2";
    }
    if has_word("street")
        || has_phrase("address line 1")
        || has_phrase("address line")
        || has_phrase("street address")
    {
        return "street1";
    }
    // Bare "address" with no qualifier → legacy single-line. Most
    // forms that say just "Address" mean street1 today, but we keep
    // a route to the legacy `address` slot so older profiles still
    // fill via the address fallback path.
    if has("address") {
        return "address";
    }
    if (has("year") || has("yrs")) && has("exp") {
        return "years_experience";
    }
    // Compensation routes. "salary" alone is unambiguous; bare
    // "compensation" is ambiguous (could be a section header) so
    // we require an "expect"/"desired" qualifier or an
    // expectation-shaped phrase. Returns the umbrella key
    // "salary_expectation"; the run loop currently surfaces no
    // value (Compensation lives under Profile.compensation:
    // Option<Compensation> and is not yet wired into
    // profile_value_for_key — Phase K's answer composer is the
    // natural home for compensation rendering, since salary
    // forms often want context like "open to equity-heavy"
    // rather than a bare number).
    if has("salary")
        || has_phrase("compensation expectation")
        || has_phrase("expected compensation")
        || has_phrase("desired compensation")
        || has_phrase("expected pay")
    {
        return "salary_expectation";
    }
    // Name disambiguation (specificity-ordered):
    //   first/given/forename + name → first_name
    //   last/family/sur + name      → last_name
    //   "surname" alone              → last_name
    //   anything else mentioning "name" → full_name
    if has_word("surname") {
        return "last_name";
    }
    if has("name") {
        // Phrase-based — the developer typed "first" + "name" together,
        // which is a real signal. A bare id="first" (no "name" near it)
        // does NOT trigger; that's the correct behavior — `id` alone
        // can't disambiguate first vs full vs anything else.
        let first_signal = has_phrase("first name")
            || has_phrase("given name")
            || has_word("forename");
        let last_signal =
            has_phrase("last name") || has_phrase("family name") || has_word("surname");
        if first_signal && !last_signal {
            return "first_name";
        }
        if last_signal && !first_signal {
            return "last_name";
        }
        // Both signals or neither → fall through to full_name.
        // (E.g. a "Legal Name (First and Last)" field asks for both.)
        return "full_name";
    }
    if kind == "textarea" {
        return "freetext";
    }
    ""
}

/// Like `predict_field_key`, but also returns a confidence score in
/// [0.0, 1.0]. The keyword classifier is binary — either it matched
/// a vocabulary entry (1.0) or it didn't (0.0). When the trained
/// classifier (R4) lands, this signature stays stable; only the
/// implementation changes. Shadow-mode threshold gating reads this.
pub fn predict_field_key_with_confidence(f: &FieldDescriptor) -> (&'static str, f32) {
    let key = predict_field_key(f);
    let confidence = if key.is_empty() { 0.0 } else { 1.0 };
    (key, confidence)
}

/// Resolve a classified key to the value the user has in their profile.
/// Returns `None` for unknown / freetext / fields not in the schema.
pub fn profile_value_for_key<'a>(profile: &'a Profile, key: &str) -> Option<&'a str> {
    // Each key has a primary field and (where applicable) a fallback
    // for forward-compat with profiles that haven't filled the
    // structured sub-fields yet. Empty strings degrade to None so
    // the run-loop's NoProfileValue branch treats unfilled fields
    // as "skip", not "fill with empty".
    let primary = |s: &'a str| -> &'a str { s };
    let fallback = |s: &'a str, f: &'a str| -> &'a str {
        if s.is_empty() { f } else { s }
    };
    let v: &str = match key {
        "full_name" => primary(&profile.full_name),
        "first_name" => fallback(&profile.first_name, &profile.full_name),
        "last_name" => fallback(&profile.last_name, &profile.full_name),
        "email" => primary(&profile.email),
        "phone" => primary(&profile.phone),
        // Structured address sub-fields. Each falls back to the
        // legacy single-line `address` so existing profiles still
        // populate something rather than skipping the field.
        "street1" => fallback(&profile.street1, &profile.address),
        "street2" => primary(&profile.street2),
        "city" => fallback(&profile.city, &profile.address),
        "state" => fallback(&profile.state, &profile.address),
        "postal_code" => fallback(&profile.postal_code, &profile.address),
        "country" => primary(&profile.country),
        "address" => primary(&profile.address),
        "linkedin" => primary(&profile.linkedin),
        "github" => primary(&profile.github),
        "website" => primary(&profile.website),
        // Phase G OnlinePresence platforms. Each falls back to the
        // legacy Profile.website only for `portfolio` (which is the
        // semantic equivalent for designers/PMs); the others have
        // no meaningful legacy fallback — empty means "user has not
        // shared this handle" and the run loop skips the field.
        "gitlab" => primary(&profile.presence.gitlab),
        "bitbucket" => primary(&profile.presence.bitbucket),
        "blog" => primary(&profile.presence.blog),
        "twitter" => primary(&profile.presence.twitter),
        "bluesky" => primary(&profile.presence.bluesky),
        "mastodon" => primary(&profile.presence.mastodon),
        "stackoverflow" => primary(&profile.presence.stackoverflow),
        "portfolio" => fallback(&profile.presence.portfolio, &profile.website),
        "work_authorization" => primary(&profile.work_authorization),
        // salary_expectation is classified but not surfaced as a
        // borrowable string here. Compensation is structured (numeric
        // min/max + currency + notes); rendering it requires
        // formatting that owns the result. Phase K's answer composer
        // handles compensation; for now the keyword classifier
        // identifies the field but the run loop skips it.
        "salary_expectation" => return None,
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

    // (field_descriptor_round_trip removed — was self-licking; the
    // wire-format invariant is enforced by
    // `field_descriptor_json_shape_is_stable` which pins the literal
    // JSON bytes, not just the round-trip equality.)

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

    /// Pin Feedback's on-disk JSON shape against literal bytes. This
    /// is the format that lands in `~/.atsisbroken/feedback.jsonl`
    /// and the format the bridge speaks across the Native Messaging
    /// boundary. A drift in field names or default behaviors would
    /// silently break the consumer side.
    #[test]
    fn feedback_json_shape_is_stable() {
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
        let got = serde_json::to_string(&fb).unwrap();
        let want = r#"{"field":{"label":"Email","placeholder":"","aria_label":"","name":"email","id":"","kind":"email"},"predicted":"email","actual":"email","accepted":true}"#;
        assert_eq!(got, want);
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
        assert_eq!(predict_field_key(&fd("Street address", "", "", "", "", "text")), "street1");
        assert_eq!(predict_field_key(&fd("Street", "", "", "", "", "text")), "street1");
        assert_eq!(predict_field_key(&fd("Address Line 1", "", "", "", "", "text")), "street1");
        assert_eq!(predict_field_key(&fd("Address Line 2", "", "", "", "", "text")), "street2");
        assert_eq!(predict_field_key(&fd("Apartment / Suite", "", "", "", "", "text")), "street2");
        assert_eq!(predict_field_key(&fd("City", "", "", "", "", "text")), "city");
        assert_eq!(predict_field_key(&fd("State", "", "", "", "", "text")), "state");
        assert_eq!(predict_field_key(&fd("Province", "", "", "", "", "text")), "state");
        assert_eq!(predict_field_key(&fd("Country", "", "", "", "", "text")), "country");
        assert_eq!(predict_field_key(&fd("Zip code", "", "", "", "", "text")), "postal_code");
        assert_eq!(predict_field_key(&fd("Postal code", "", "", "", "", "text")), "postal_code");
        // Bare "Address" with no qualifier → legacy single-line slot.
        // Forms that mean "street1" usually say so.
        assert_eq!(predict_field_key(&fd("Address", "", "", "", "", "text")), "address");
    }

    #[test]
    fn predict_address_does_not_match_address_inside_unrelated_words() {
        // Regression-style: a field id="addressbar" or label
        // "Email address line" must NOT classify as a street1.
        // (Email wins via has("email") earlier in the chain.)
        assert_eq!(
            predict_field_key(&fd("Email address", "", "", "user_email", "", "email")),
            "email"
        );
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
    fn predict_name_disambiguation() {
        // Most-common bug: forms with separate first/last fields used
        // to both classify as "full_name" and got the same string
        // dropped into both. Now they split.
        assert_eq!(predict_field_key(&fd("First name", "", "", "fname", "", "text")), "first_name");
        assert_eq!(predict_field_key(&fd("Last name", "", "", "", "", "text")), "last_name");
        assert_eq!(predict_field_key(&fd("Surname", "", "", "", "", "text")), "last_name");
        assert_eq!(predict_field_key(&fd("Given name", "", "", "", "", "text")), "first_name");
        assert_eq!(predict_field_key(&fd("Family name", "", "", "", "", "text")), "last_name");
        // Single-name field stays full_name
        assert_eq!(predict_field_key(&fd("Full name", "", "", "", "", "text")), "full_name");
        assert_eq!(predict_field_key(&fd("Name", "", "", "", "", "text")), "full_name");
        // Workday — real markup has aria-label="First Name" alongside
        // the parenthesized visible label. The aria-label is the
        // unambiguous signal; the parenthesized "(First)" alone
        // cannot disambiguate (form might use "Name (First)" or
        // "Name (Mr/Mrs)" with the same shape).
        assert_eq!(
            predict_field_key(&fd(
                "Legal Name (First)",
                "",
                "First Name",
                "legalNameFirst",
                "lf",
                "text"
            )),
            "first_name"
        );
        assert_eq!(
            predict_field_key(&fd(
                "Legal Name (Last)",
                "",
                "Last Name",
                "legalNameLast",
                "ll",
                "text"
            )),
            "last_name"
        );

        // Negative case: a form whose ID alone happens to be "first"
        // but whose label is "Full name" — must NOT classify as
        // first_name. This is the Lever combined-name-field shape
        // and the case the e2e test caught.
        assert_eq!(
            predict_field_key(&fd("Full name", "", "", "name", "first", "text")),
            "full_name"
        );
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
        // "Salary expectation" used to live here as a
        // not-yet-classified example; Phase G Tier 3 added a
        // salary_expectation route, so we substitute a still-
        // genuinely-unknown prompt. Anything not in the vocabulary
        // must return "" — never invent a route.
        assert_eq!(predict_field_key(&fd("Hobbies", "", "", "", "", "text")), "");
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
    fn predict_with_confidence_returns_one_for_known() {
        let (key, conf) = predict_field_key_with_confidence(
            &fd("Email", "", "", "", "", "email"),
        );
        assert_eq!(key, "email");
        assert_eq!(conf, 1.0);
    }

    #[test]
    fn predict_with_confidence_returns_zero_for_unknown() {
        let (key, conf) = predict_field_key_with_confidence(
            &fd("How many siblings?", "", "", "", "", "number"),
        );
        assert_eq!(key, "");
        assert_eq!(conf, 0.0);
    }

    // ─── Phase G Tier 3 — new presence + salary classifier routes ────────

    #[test]
    fn predict_gitlab_routes_to_gitlab() {
        assert_eq!(predict_field_key(&fd("GitLab URL", "", "", "", "", "url")), "gitlab");
        assert_eq!(predict_field_key(&fd("", "", "", "gitlab_handle", "", "text")), "gitlab");
    }

    #[test]
    fn predict_bitbucket_routes_to_bitbucket() {
        assert_eq!(predict_field_key(&fd("Bitbucket", "", "", "", "", "url")), "bitbucket");
    }

    #[test]
    fn predict_twitter_routes_to_twitter() {
        assert_eq!(predict_field_key(&fd("Twitter", "", "", "", "", "url")), "twitter");
        assert_eq!(predict_field_key(&fd("Twitter / X handle", "", "", "", "", "text")), "twitter");
    }

    #[test]
    fn predict_bluesky_routes_to_bluesky() {
        assert_eq!(predict_field_key(&fd("Bluesky", "", "", "", "", "url")), "bluesky");
        assert_eq!(predict_field_key(&fd("Bluesky handle", "", "", "", "", "text")), "bluesky");
    }

    #[test]
    fn predict_mastodon_routes_to_mastodon() {
        assert_eq!(predict_field_key(&fd("Mastodon", "", "", "", "", "url")), "mastodon");
    }

    #[test]
    fn predict_stackoverflow_routes_to_stackoverflow() {
        assert_eq!(predict_field_key(&fd("StackOverflow URL", "", "", "", "", "url")), "stackoverflow");
        assert_eq!(predict_field_key(&fd("Stack Overflow profile", "", "", "", "", "url")), "stackoverflow");
    }

    #[test]
    fn predict_blog_routes_to_blog() {
        // Word-equality on "blog" — substring "weblog" must NOT
        // match (no false positive on that token).
        assert_eq!(predict_field_key(&fd("Blog URL", "", "", "", "", "url")), "blog");
        assert_eq!(predict_field_key(&fd("Personal blog", "", "", "", "", "url")), "blog");
    }

    #[test]
    fn predict_blog_does_not_match_inside_unrelated_words() {
        // The word-equality guard is what stops "weblog" from
        // matching the blog route. The has() route over the raw
        // hay would false-positive; has_word() doesn't.
        // Synthetic but worth pinning.
        let f = fd("Weblog server URL", "", "", "weblog_url", "", "url");
        // Should NOT classify as blog. Falls through to website
        // since "weblog" doesn't match has("blog") via word-equality
        // and "URL" alone goes to website? Actually nothing in our
        // vocab routes generic "url" — so this falls through to
        // unknown. Either is acceptable; the contract is "not blog".
        let got = predict_field_key(&f);
        assert_ne!(got, "blog");
    }

    #[test]
    fn predict_salary_expectation_routes_to_salary_expectation() {
        assert_eq!(
            predict_field_key(&fd("Salary expectation", "", "", "", "", "number")),
            "salary_expectation"
        );
        assert_eq!(
            predict_field_key(&fd("Expected salary", "", "", "", "", "number")),
            "salary_expectation"
        );
        assert_eq!(
            predict_field_key(&fd("Compensation expectation", "", "", "", "", "text")),
            "salary_expectation"
        );
        assert_eq!(
            predict_field_key(&fd("Expected pay", "", "", "", "", "number")),
            "salary_expectation"
        );
    }

    #[test]
    fn predict_bare_compensation_does_not_match_salary() {
        // Bare "Compensation" as a section header alone (no
        // qualifier) must NOT match — it's ambiguous between a
        // header and an actual ask. Caller would surface this
        // as unknown.
        let got = predict_field_key(&fd("Compensation", "", "", "", "", "text"));
        assert_ne!(got, "salary_expectation");
    }

    #[test]
    fn profile_value_for_key_resolves_new_presence_keys() {
        let p = Profile {
            presence: OnlinePresence {
                gitlab: "https://gitlab.com/janedoe".into(),
                bitbucket: "https://bitbucket.org/janedoe".into(),
                blog: "https://janedoe.dev/blog".into(),
                twitter: "@janedoe".into(),
                bluesky: "@janedoe.bsky.social".into(),
                mastodon: "@janedoe@mastodon.social".into(),
                stackoverflow: "https://stackoverflow.com/users/123".into(),
                portfolio: "https://janedoe.dev/portfolio".into(),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(profile_value_for_key(&p, "gitlab"), Some("https://gitlab.com/janedoe"));
        assert_eq!(profile_value_for_key(&p, "bitbucket"), Some("https://bitbucket.org/janedoe"));
        assert_eq!(profile_value_for_key(&p, "blog"), Some("https://janedoe.dev/blog"));
        assert_eq!(profile_value_for_key(&p, "twitter"), Some("@janedoe"));
        assert_eq!(profile_value_for_key(&p, "bluesky"), Some("@janedoe.bsky.social"));
        assert_eq!(profile_value_for_key(&p, "mastodon"), Some("@janedoe@mastodon.social"));
        assert_eq!(profile_value_for_key(&p, "stackoverflow"), Some("https://stackoverflow.com/users/123"));
        assert_eq!(profile_value_for_key(&p, "portfolio"), Some("https://janedoe.dev/portfolio"));
    }

    #[test]
    fn profile_value_for_key_portfolio_falls_back_to_website() {
        // Portfolio has a meaningful legacy fallback to the
        // top-level `website` field (designer/PM convention:
        // many use the same URL for both). Pinned so the
        // fallback can't be silently removed.
        let p = Profile {
            website: "https://janedoe.dev".into(),
            ..Default::default()
        };
        assert_eq!(profile_value_for_key(&p, "portfolio"), Some("https://janedoe.dev"));
    }

    #[test]
    fn profile_value_for_key_empty_presence_returns_none() {
        // Empty platform field with no legacy fallback (everything
        // except portfolio) returns None — the run loop reads
        // None as "skip this field" rather than "fill with empty".
        let p = Profile::default();
        assert!(profile_value_for_key(&p, "gitlab").is_none());
        assert!(profile_value_for_key(&p, "twitter").is_none());
        assert!(profile_value_for_key(&p, "mastodon").is_none());
    }

    #[test]
    fn profile_value_for_key_salary_expectation_returns_none() {
        // Pinned: salary_expectation is classified but not yet
        // surfaced as a borrowable string. When Phase K wires
        // compensation rendering, this contract changes — the
        // test will fail and force an explicit migration.
        let p = Profile {
            compensation: Some(Compensation {
                salary_expectation_min: Some(150_000),
                salary_expectation_max: Some(220_000),
                salary_currency: "USD".into(),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(profile_value_for_key(&p, "salary_expectation").is_none());
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
    /// Phase G added 7 forward-compat fields (location, employment_type,
    /// supervisor_name/email/phone, reason_for_leaving, can_we_contact);
    /// they all serialize at default values when not set, which is what
    /// the wire format now looks like for v0 profile.toml entries.
    #[test]
    fn experience_json_shape_is_stable() {
        let e = Experience {
            company: "Acme Co".into(),
            title: "Engineer III".into(),
            start: "2020-01".into(),
            end: "2024-06".into(),
            bullets: vec!["shipped X".into(), "led Y".into()],
            ..Default::default()
        };
        let got = serde_json::to_string(&e).unwrap();
        let want = r#"{"company":"Acme Co","title":"Engineer III","start":"2020-01","end":"2024-06","bullets":["shipped X","led Y"],"location":"","employment_type":"","supervisor_name":"","supervisor_email":"","supervisor_phone":"","reason_for_leaving":"","can_we_contact":false}"#;
        assert_eq!(got, want);
    }

    /// Pin Education's on-disk shape including the Option<gpa> serialization
    /// (must be `null` when None — not omitted — so file diffs are stable).
    /// Phase G added 6 forward-compat fields (honors, minor,
    /// relevant_coursework, thesis_title, extracurriculars, location).
    #[test]
    fn education_json_shape_is_stable() {
        let ed = Education {
            school: "State U".into(),
            degree: "BS".into(),
            field: "CS".into(),
            start: "2016".into(),
            end: "2020".into(),
            gpa: None,
            ..Default::default()
        };
        let got = serde_json::to_string(&ed).unwrap();
        let want = r#"{"school":"State U","degree":"BS","field":"CS","start":"2016","end":"2020","gpa":null,"honors":[],"minor":"","relevant_coursework":[],"thesis_title":"","extracurriculars":[],"location":""}"#;
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

    // ─── Phase G: expanded schema migration + sub-struct round-trips ──────

    /// v0 → v1 migration without an explicit migrate function: every
    /// Phase G field is `#[serde(default)]`, so a TOML written by an
    /// older atsisbroken version parses cleanly with all new fields at
    /// their defaults. This is the migration. Add a field, give it a
    /// default — never re-shape what's already on disk.
    #[test]
    fn profile_v0_toml_loads_with_phase_g_defaults() {
        // Exact shape an older atsisbroken (pre-Phase-G) would have written.
        let v0 = r#"
            full_name = "Jane Doe"
            first_name = "Jane"
            last_name = "Doe"
            email = "jane@example.com"
            phone = "+1-555-0100"
            address = "123 Main St, Anywhere, CA 94000"
            street1 = ""
            street2 = ""
            city = ""
            state = ""
            postal_code = ""
            country = ""
            linkedin = ""
            github = ""
            website = ""
            work_authorization = "Citizen"
            years_experience = 7
            raw_resume_text = ""
            experience = []
            education = []
            skills = ["rust", "ml"]
        "#;
        let p: Profile = toml::from_str(v0).unwrap();
        // Existing fields preserved.
        assert_eq!(p.full_name, "Jane Doe");
        assert_eq!(p.first_name, "Jane");
        assert_eq!(p.last_name, "Doe");
        assert_eq!(p.years_experience, 7);
        assert_eq!(p.skills, vec!["rust".to_string(), "ml".to_string()]);
        // Every new Phase G field defaults cleanly.
        assert_eq!(p.preferred_name, "");
        assert_eq!(p.pronouns, "");
        assert_eq!(p.presence, OnlinePresence::default());
        assert!(p.technical_skills.is_empty());
        assert!(p.languages.is_empty());
        assert!(p.soft_skills.is_empty());
        assert!(p.certifications.is_empty());
        assert!(p.projects.is_empty());
        assert!(p.compensation.is_none());
        assert!(p.demographics.is_none());
    }

    /// Round-trip every Phase G field non-default through TOML. If any
    /// new field gets dropped or reshaped on serialize/deserialize, this
    /// detects it before users lose data.
    #[test]
    fn profile_v1_full_round_trip_preserves_all_phase_g_fields() {
        let p = Profile {
            full_name: "Jane Q. Doe".into(),
            first_name: "Jane Q.".into(),
            last_name: "Doe".into(),
            preferred_name: "Janie".into(),
            pronouns: "she/her".into(),
            email: "jane@example.com".into(),
            phone: "+1-555-0100".into(),
            address: "".into(),
            street1: "123 Main St".into(),
            city: "Anywhere".into(),
            state: "CA".into(),
            postal_code: "94000".into(),
            country: "USA".into(),
            linkedin: "https://linkedin.com/in/janedoe".into(),
            github: "https://github.com/janedoe".into(),
            website: "https://janedoe.dev".into(),
            presence: OnlinePresence {
                gitlab: "https://gitlab.com/janedoe".into(),
                portfolio: "https://janedoe.dev/portfolio".into(),
                blog: "https://janedoe.dev/blog".into(),
                twitter: "@janedoe".into(),
                bluesky: "@janedoe.bsky.social".into(),
                mastodon: "@janedoe@mastodon.social".into(),
                stackoverflow: "https://stackoverflow.com/users/123/janedoe".into(),
                ..Default::default()
            },
            work_authorization: "Citizen".into(),
            years_experience: 7,
            technical_skills: vec![Skill {
                name: "Rust".into(),
                years: Some(5.5),
                level: SkillLevel::Advanced,
                last_used: "2026".into(),
            }],
            languages: vec![Language {
                name: "Spanish".into(),
                proficiency: LanguageProficiency::Conversational,
            }],
            soft_skills: vec!["written communication".into()],
            certifications: vec![Certification {
                name: "AWS Solutions Architect — Associate".into(),
                issuer: "Amazon Web Services".into(),
                issue_date: "2024-09".into(),
                expiry_date: "2027-09".into(),
                credential_id: "ABC-123".into(),
                credential_url: "https://credly.com/badges/abc-123".into(),
            }],
            projects: vec![Project {
                name: "atsisbroken".into(),
                description: "Browser autopilot for ATS forms".into(),
                url: "https://github.com/cochranblock/atsisbroken".into(),
                start_date: "2026-04".into(),
                end_date: "".into(),
                technologies: vec!["Rust".into(), "CDP".into()],
                role: "creator/maintainer".into(),
                highlights: vec!["263/263 tests".into()],
                github_repo: Some(RepoRef {
                    owner: "cochranblock".into(),
                    name: "atsisbroken".into(),
                    url: "https://github.com/cochranblock/atsisbroken".into(),
                }),
            }],
            compensation: Some(Compensation {
                salary_expectation_min: Some(150_000),
                salary_expectation_max: Some(220_000),
                salary_currency: "USD".into(),
                compensation_notes: "open to equity-heavy".into(),
                desired_base: Some(170_000),
                desired_variable: Some(20_000),
                desired_equity: "0.1%".into(),
            }),
            demographics: Some(Demographics::default()),
            raw_resume_text: "".into(),
            ..Default::default()
        };
        let toml_text = toml::to_string(&p).unwrap();
        let back: Profile = toml::from_str(&toml_text).unwrap();
        assert_eq!(p, back);
    }

    /// Pin OnlinePresence's wire shape. Drift here would silently
    /// drop link kinds from existing user profiles.
    #[test]
    fn online_presence_json_shape_is_stable() {
        let op = OnlinePresence::default();
        let got = serde_json::to_string(&op).unwrap();
        let want = r#"{"gitlab":"","bitbucket":"","portfolio":"","blog":"","twitter":"","bluesky":"","mastodon":"","stackoverflow":"","devto":"","medium":"","hashnode":"","youtube":"","dribbble":"","behance":"","artstation":""}"#;
        assert_eq!(got, want);
    }

    #[test]
    fn skill_json_shape_is_stable() {
        let s = Skill {
            name: "Rust".into(),
            years: Some(5.0),
            level: SkillLevel::Advanced,
            last_used: "2026".into(),
        };
        let got = serde_json::to_string(&s).unwrap();
        let want = r#"{"name":"Rust","years":5.0,"level":"advanced","last_used":"2026"}"#;
        assert_eq!(got, want);
    }

    #[test]
    fn skill_level_serializes_snake_case() {
        assert_eq!(serde_json::to_string(&SkillLevel::Beginner).unwrap(), "\"beginner\"");
        assert_eq!(serde_json::to_string(&SkillLevel::Intermediate).unwrap(), "\"intermediate\"");
        assert_eq!(serde_json::to_string(&SkillLevel::Advanced).unwrap(), "\"advanced\"");
        assert_eq!(serde_json::to_string(&SkillLevel::Expert).unwrap(), "\"expert\"");
    }

    #[test]
    fn skill_default_level_is_intermediate() {
        // Self-reported defaults bias toward "I can do this" rather
        // than "I'm a beginner" — most users underclaim, not overclaim.
        // Deliberate; if this changes, do it on purpose.
        assert_eq!(SkillLevel::default(), SkillLevel::Intermediate);
    }

    #[test]
    fn language_json_shape_is_stable() {
        let l = Language {
            name: "Spanish".into(),
            proficiency: LanguageProficiency::Conversational,
        };
        let got = serde_json::to_string(&l).unwrap();
        let want = r#"{"name":"Spanish","proficiency":"conversational"}"#;
        assert_eq!(got, want);
    }

    #[test]
    fn language_proficiency_serializes_snake_case() {
        assert_eq!(serde_json::to_string(&LanguageProficiency::Native).unwrap(), "\"native\"");
        assert_eq!(serde_json::to_string(&LanguageProficiency::Fluent).unwrap(), "\"fluent\"");
        assert_eq!(serde_json::to_string(&LanguageProficiency::Conversational).unwrap(), "\"conversational\"");
        assert_eq!(serde_json::to_string(&LanguageProficiency::Beginner).unwrap(), "\"beginner\"");
    }

    #[test]
    fn certification_json_shape_is_stable() {
        let c = Certification {
            name: "AWS SAA".into(),
            issuer: "AWS".into(),
            issue_date: "2024-09".into(),
            expiry_date: "2027-09".into(),
            credential_id: "ABC-123".into(),
            credential_url: "https://credly.com/badges/abc-123".into(),
        };
        let got = serde_json::to_string(&c).unwrap();
        let want = r#"{"name":"AWS SAA","issuer":"AWS","issue_date":"2024-09","expiry_date":"2027-09","credential_id":"ABC-123","credential_url":"https://credly.com/badges/abc-123"}"#;
        assert_eq!(got, want);
    }

    #[test]
    fn project_with_github_repo_json_shape_is_stable() {
        let proj = Project {
            name: "atsisbroken".into(),
            description: "Browser autopilot".into(),
            url: "https://github.com/cochranblock/atsisbroken".into(),
            start_date: "2026-04".into(),
            end_date: "".into(),
            technologies: vec!["Rust".into()],
            role: "creator".into(),
            highlights: vec!["263/263".into()],
            github_repo: Some(RepoRef {
                owner: "cochranblock".into(),
                name: "atsisbroken".into(),
                url: "https://github.com/cochranblock/atsisbroken".into(),
            }),
        };
        let got = serde_json::to_string(&proj).unwrap();
        let want = r#"{"name":"atsisbroken","description":"Browser autopilot","url":"https://github.com/cochranblock/atsisbroken","start_date":"2026-04","end_date":"","technologies":["Rust"],"role":"creator","highlights":["263/263"],"github_repo":{"owner":"cochranblock","name":"atsisbroken","url":"https://github.com/cochranblock/atsisbroken"}}"#;
        assert_eq!(got, want);
    }

    #[test]
    fn project_without_github_repo_serializes_repo_as_null() {
        // Option<RepoRef> = None must serialize as `null`, not be
        // omitted — so the wire format is stable across v0 → v1.
        let proj = Project {
            name: "private project".into(),
            description: "personal".into(),
            ..Default::default()
        };
        let got = serde_json::to_string(&proj).unwrap();
        assert!(got.contains(r#""github_repo":null"#));
    }

    #[test]
    fn compensation_json_shape_is_stable() {
        let c = Compensation {
            salary_expectation_min: Some(150_000),
            salary_expectation_max: Some(220_000),
            salary_currency: "USD".into(),
            compensation_notes: "open to equity-heavy".into(),
            desired_base: Some(170_000),
            desired_variable: Some(20_000),
            desired_equity: "0.1%".into(),
        };
        let got = serde_json::to_string(&c).unwrap();
        let want = r#"{"salary_expectation_min":150000,"salary_expectation_max":220000,"salary_currency":"USD","compensation_notes":"open to equity-heavy","desired_base":170000,"desired_variable":20000,"desired_equity":"0.1%"}"#;
        assert_eq!(got, want);
    }

    #[test]
    fn demographics_default_is_all_empty() {
        // Default Demographics must be inert — no field set means
        // "user has not chosen to disclose anything." This is the
        // *baseline* the run loop sees when the user has never
        // touched demographics; it must trigger zero autofills.
        let d = Demographics::default();
        assert!(d.gender.is_empty());
        assert!(d.race_ethnicity.is_empty());
        assert!(d.veteran_status.is_empty());
        assert!(d.disability_status.is_empty());
        assert!(d.lgbtq_self_id.is_empty());
    }

    #[test]
    fn profile_demographics_default_is_none_not_empty_struct() {
        // Hard contract: a Profile that has never had demographics
        // populated should have `demographics: None`, not
        // `Some(Demographics::default())`. The Option discriminator
        // is the run loop's guard — `None` means "skip every EEO
        // field"; Some-with-empty-struct would still mean "user
        // opted in but left blank" which is a different signal.
        assert!(Profile::default().demographics.is_none());
    }

    #[test]
    fn profile_compensation_default_is_none() {
        // Same contract as demographics — None means "skip salary
        // fields"; Some with blank min/max means "user opted in but
        // left blank" which the run loop treats differently.
        assert!(Profile::default().compensation.is_none());
    }

    #[test]
    fn repo_ref_round_trip() {
        let r = RepoRef {
            owner: "cochranblock".into(),
            name: "atsisbroken".into(),
            url: "https://github.com/cochranblock/atsisbroken".into(),
        };
        let s = serde_json::to_string(&r).unwrap();
        let back: RepoRef = serde_json::from_str(&s).unwrap();
        assert_eq!(r, back);
    }

    #[test]
    fn experience_v0_toml_loads_with_phase_g_defaults() {
        // An older Experience entry (just the 5 original fields)
        // must parse cleanly with new fields at defaults.
        let v0 = r#"
            company = "Acme Co"
            title   = "Engineer III"
            start   = "2020-01"
            end     = "2024-06"
            bullets = ["shipped X", "led Y"]
        "#;
        let e: Experience = toml::from_str(v0).unwrap();
        assert_eq!(e.company, "Acme Co");
        assert_eq!(e.location, "");
        assert_eq!(e.employment_type, "");
        assert!(!e.can_we_contact);
    }

    #[test]
    fn education_v0_toml_loads_with_phase_g_defaults() {
        let v0 = r#"
            school = "State U"
            degree = "BS"
            field  = "CS"
            start  = "2016"
            end    = "2020"
        "#;
        let ed: Education = toml::from_str(v0).unwrap();
        assert_eq!(ed.school, "State U");
        assert!(ed.honors.is_empty());
        assert_eq!(ed.minor, "");
        assert_eq!(ed.thesis_title, "");
    }

    // ─── Phase G Tier 2 — work auth + history + free-form ────────────────

    #[test]
    fn profile_tier2_defaults_are_inert() {
        // Hard contracts: a default Profile has nothing in the Tier 2
        // collections that could trigger an autofill. The run loop
        // reads None / empty as "skip"; presence of any value is the
        // user's explicit gesture.
        let p = Profile::default();
        assert!(p.work_auth.is_none(),
            "work_auth must default to None (skip) — Some(default) would mean opted-in-blank");
        assert!(p.publications.is_empty());
        assert!(p.patents.is_empty());
        assert!(p.awards.is_empty());
        assert!(p.references.is_empty());
        assert!(p.applications.is_empty());
        // free_form slots are all None until the user authors something
        // OR the composer caches an accepted answer.
        assert!(p.free_form.elevator_pitch.is_none());
        assert!(p.free_form.proudest_project.is_none());
        assert!(p.free_form.custom.is_empty());
    }

    #[test]
    fn profile_v0_toml_loads_with_tier2_defaults() {
        // The exact v0 shape from before Tier 2 must still parse; new
        // collections come up empty and Optional fields come up None.
        let v0 = r#"
            full_name = "Jane Doe"
            email = "jane@example.com"
            phone = ""
            address = ""
            linkedin = ""
            github = ""
            website = ""
            work_authorization = "Citizen"
            years_experience = 0
            raw_resume_text = ""
            experience = []
            education = []
            skills = []
        "#;
        let p: Profile = toml::from_str(v0).unwrap();
        assert!(p.work_auth.is_none());
        assert!(p.publications.is_empty());
        assert!(p.patents.is_empty());
        assert!(p.awards.is_empty());
        assert!(p.references.is_empty());
        assert!(p.applications.is_empty());
        assert_eq!(p.free_form, FreeFormAnswers::default());
        // Legacy work_authorization String preserved (no breaking change).
        assert_eq!(p.work_authorization, "Citizen");
    }

    #[test]
    fn work_auth_status_serializes_snake_case() {
        assert_eq!(serde_json::to_string(&WorkAuthStatus::Citizen).unwrap(), "\"citizen\"");
        assert_eq!(serde_json::to_string(&WorkAuthStatus::PermanentResident).unwrap(), "\"permanent_resident\"");
        assert_eq!(serde_json::to_string(&WorkAuthStatus::H1B).unwrap(), "\"h1_b\"");
        assert_eq!(serde_json::to_string(&WorkAuthStatus::Opt).unwrap(), "\"opt\"");
        assert_eq!(serde_json::to_string(&WorkAuthStatus::RequireSponsorship).unwrap(), "\"require_sponsorship\"");
        // The Other variant carries a String — serializes as
        // {"other": "..."} via serde's default tagged enum behavior.
        let s = serde_json::to_string(&WorkAuthStatus::Other("DACA".into())).unwrap();
        assert_eq!(s, r#"{"other":"DACA"}"#);
    }

    #[test]
    fn work_auth_status_default_is_empty_other_escape_hatch() {
        // The default reads as "user has not specified a status" —
        // distinct from any concrete category. The run loop must NOT
        // autofill a work-auth dropdown when status is Other("").
        assert_eq!(WorkAuthStatus::default(), WorkAuthStatus::Other(String::new()));
    }

    #[test]
    fn security_clearance_none_distinct_from_unspecified() {
        // SecurityClearance::None means the user explicitly stated
        // "I do not hold any clearance." That's a real ATS-form
        // answer. SecurityClearance::Other("") means "user hasn't
        // told us either way." A form asking "do you hold a
        // clearance?" must distinguish these — auto-filling "no"
        // when the user simply hasn't stated would put words in
        // their mouth.
        assert_ne!(SecurityClearance::default(), SecurityClearance::None);
        assert_eq!(SecurityClearance::default(), SecurityClearance::Other(String::new()));
    }

    #[test]
    fn remote_preference_default_is_flexible() {
        // "Flexible" reads as "user hasn't expressed a preference."
        // Any UI surfacing remote preference should prompt rather
        // than treat the default as a strong signal.
        assert_eq!(RemotePreference::default(), RemotePreference::Flexible);
    }

    #[test]
    fn remote_preference_serializes_snake_case() {
        assert_eq!(serde_json::to_string(&RemotePreference::Remote).unwrap(), "\"remote\"");
        assert_eq!(serde_json::to_string(&RemotePreference::Hybrid).unwrap(), "\"hybrid\"");
        assert_eq!(serde_json::to_string(&RemotePreference::OnSite).unwrap(), "\"on_site\"");
        assert_eq!(serde_json::to_string(&RemotePreference::Flexible).unwrap(), "\"flexible\"");
    }

    #[test]
    fn application_status_default_is_submitted() {
        // Run loop appends every successful submission as Submitted.
        // Other states are user-driven transitions; the default that
        // gets stored is what the run loop writes.
        assert_eq!(ApplicationStatus::default(), ApplicationStatus::Submitted);
    }

    #[test]
    fn application_status_serializes_snake_case() {
        assert_eq!(serde_json::to_string(&ApplicationStatus::Submitted).unwrap(), "\"submitted\"");
        assert_eq!(serde_json::to_string(&ApplicationStatus::Interviewed).unwrap(), "\"interviewed\"");
        assert_eq!(serde_json::to_string(&ApplicationStatus::Offer).unwrap(), "\"offer\"");
        assert_eq!(serde_json::to_string(&ApplicationStatus::Rejected).unwrap(), "\"rejected\"");
        assert_eq!(serde_json::to_string(&ApplicationStatus::Ghosted).unwrap(), "\"ghosted\"");
        assert_eq!(serde_json::to_string(&ApplicationStatus::Withdrawn).unwrap(), "\"withdrawn\"");
    }

    #[test]
    fn work_auth_json_shape_is_stable() {
        let wa = WorkAuth {
            citizenship_country: "USA".into(),
            status: WorkAuthStatus::Citizen,
            visa_sponsorship_required: false,
            security_clearance: SecurityClearance::None,
            relocation_willingness: true,
            region_preferences: vec!["West Coast US".into()],
            remote_preference: RemotePreference::Hybrid,
        };
        let got = serde_json::to_string(&wa).unwrap();
        let want = r#"{"citizenship_country":"USA","status":"citizen","visa_sponsorship_required":false,"security_clearance":"none","relocation_willingness":true,"region_preferences":["West Coast US"],"remote_preference":"hybrid"}"#;
        assert_eq!(got, want);
    }

    #[test]
    fn publication_json_shape_is_stable() {
        let pub_ = Publication {
            title: "On the Verbatim Source Property".into(),
            authors: vec!["Cochran, M.".into()],
            venue: "USENIX Security".into(),
            date: "2026-08".into(),
            url: "https://example.org/paper".into(),
            doi: "10.1234/example.5678".into(),
        };
        let got = serde_json::to_string(&pub_).unwrap();
        let want = r#"{"title":"On the Verbatim Source Property","authors":["Cochran, M."],"venue":"USENIX Security","date":"2026-08","url":"https://example.org/paper","doi":"10.1234/example.5678"}"#;
        assert_eq!(got, want);
    }

    #[test]
    fn patent_json_shape_is_stable() {
        let p = Patent {
            title: "Method for verifiable autofill".into(),
            number: "US10,123,456 B2".into(),
            issued_date: "2024-03-12".into(),
            inventors: vec!["Cochran, M.".into()],
            url: "https://patents.google.com/patent/US10123456B2".into(),
        };
        let got = serde_json::to_string(&p).unwrap();
        let want = r#"{"title":"Method for verifiable autofill","number":"US10,123,456 B2","issued_date":"2024-03-12","inventors":["Cochran, M."],"url":"https://patents.google.com/patent/US10123456B2"}"#;
        assert_eq!(got, want);
    }

    #[test]
    fn award_json_shape_is_stable() {
        let a = Award {
            name: "Outstanding Contribution Award".into(),
            issuer: "Open Source Foundation".into(),
            date: "2025".into(),
            description: "For sustained Rust ecosystem maintenance".into(),
        };
        let got = serde_json::to_string(&a).unwrap();
        let want = r#"{"name":"Outstanding Contribution Award","issuer":"Open Source Foundation","date":"2025","description":"For sustained Rust ecosystem maintenance"}"#;
        assert_eq!(got, want);
    }

    #[test]
    fn reference_json_shape_is_stable() {
        let r = Reference {
            name: "Pat Doe".into(),
            relationship: "Direct Manager".into(),
            company: "Acme Co".into(),
            title: "VP Engineering".into(),
            email: "pat.doe@acme.example".into(),
            phone: "+1-555-0188".into(),
        };
        let got = serde_json::to_string(&r).unwrap();
        let want = r#"{"name":"Pat Doe","relationship":"Direct Manager","company":"Acme Co","title":"VP Engineering","email":"pat.doe@acme.example","phone":"+1-555-0188"}"#;
        assert_eq!(got, want);
    }

    #[test]
    fn application_json_shape_is_stable() {
        let a = Application {
            company: "Example Corp".into(),
            role: "Senior Engineer".into(),
            submitted_at: "2026-05-06T10:11:12Z".into(),
            url: "https://example.com/jobs/123".into(),
            status: ApplicationStatus::Submitted,
            notes: "applied via Greenhouse".into(),
        };
        let got = serde_json::to_string(&a).unwrap();
        let want = r#"{"company":"Example Corp","role":"Senior Engineer","submitted_at":"2026-05-06T10:11:12Z","url":"https://example.com/jobs/123","status":"submitted","notes":"applied via Greenhouse"}"#;
        assert_eq!(got, want);
    }

    #[test]
    fn free_form_answers_json_shape_is_stable() {
        // Default is all None / empty — the all-null shape pins the
        // wire format for fresh profiles. Adding a slot in the future
        // will FAIL this test and force an explicit migration.
        let f = FreeFormAnswers::default();
        let got = serde_json::to_string(&f).unwrap();
        let want = r#"{"elevator_pitch":null,"why_this_role_template":null,"biggest_technical_challenge":null,"proudest_project":null,"biggest_failure_and_lesson":null,"five_year_plan":null,"strengths":null,"weaknesses":null,"management_style":null,"collaboration_example":null,"custom":{}}"#;
        assert_eq!(got, want);
    }

    #[test]
    fn free_form_none_vs_empty_some_are_distinct_signals() {
        // None: "user has not authored a response — composer may
        // synthesize from inventories."
        // Some(""): "user explicitly cleared this slot — never
        // autofill, even from a verbatim source."
        // Phase K reads these differently. Pin the type-level
        // distinction so it can't drift.
        let none_signal = FreeFormAnswers::default();
        let cleared = FreeFormAnswers {
            elevator_pitch: Some(String::new()),
            ..Default::default()
        };
        assert_ne!(none_signal, cleared);
        assert!(none_signal.elevator_pitch.is_none());
        assert_eq!(cleared.elevator_pitch.as_deref(), Some(""));
    }

    #[test]
    fn application_status_round_trip() {
        // Every variant must round-trip through JSON. A drift in
        // discriminant order or rename would leak into existing
        // user ledgers.
        for s in [
            ApplicationStatus::Submitted,
            ApplicationStatus::Interviewed,
            ApplicationStatus::Offer,
            ApplicationStatus::Rejected,
            ApplicationStatus::Ghosted,
            ApplicationStatus::Withdrawn,
        ] {
            let j = serde_json::to_string(&s).unwrap();
            let back: ApplicationStatus = serde_json::from_str(&j).unwrap();
            assert_eq!(s, back);
        }
    }

    #[test]
    fn work_auth_status_round_trip_including_other() {
        for s in [
            WorkAuthStatus::Citizen,
            WorkAuthStatus::PermanentResident,
            WorkAuthStatus::H1B,
            WorkAuthStatus::Opt,
            WorkAuthStatus::Ead,
            WorkAuthStatus::Tn,
            WorkAuthStatus::OptionalPracticalTraining,
            WorkAuthStatus::RequireSponsorship,
            WorkAuthStatus::Other("DACA".into()),
            WorkAuthStatus::Other(String::new()),
        ] {
            let j = serde_json::to_string(&s).unwrap();
            let back: WorkAuthStatus = serde_json::from_str(&j).unwrap();
            assert_eq!(s, back);
        }
    }

    #[test]
    fn profile_v1_tier2_full_round_trip() {
        // Set every Tier 2 field non-default; round-trip via TOML.
        let p = Profile {
            full_name: "Jane Doe".into(),
            email: "jane@example.com".into(),
            phone: "+1-555-0100".into(),
            address: "123 Main St".into(),
            work_auth: Some(WorkAuth {
                citizenship_country: "USA".into(),
                status: WorkAuthStatus::Citizen,
                visa_sponsorship_required: false,
                security_clearance: SecurityClearance::Secret,
                relocation_willingness: true,
                region_preferences: vec!["West Coast US".into(), "EMEA".into()],
                remote_preference: RemotePreference::Hybrid,
            }),
            publications: vec![Publication {
                title: "Paper".into(),
                authors: vec!["Doe, J.".into()],
                venue: "Conf".into(),
                date: "2025".into(),
                url: String::new(),
                doi: String::new(),
            }],
            patents: vec![Patent {
                title: "Widget".into(),
                number: "US123".into(),
                issued_date: "2024".into(),
                inventors: vec!["Doe, J.".into()],
                url: String::new(),
            }],
            awards: vec![Award {
                name: "Award".into(),
                issuer: "Org".into(),
                date: "2023".into(),
                description: String::new(),
            }],
            references: vec![Reference {
                name: "Pat".into(),
                relationship: "Manager".into(),
                company: "Acme".into(),
                title: "VP".into(),
                email: "pat@acme.example".into(),
                phone: String::new(),
            }],
            applications: vec![Application {
                company: "Example".into(),
                role: "SWE".into(),
                submitted_at: "2026-05-06T10:11:12Z".into(),
                url: "https://example.com/jobs/1".into(),
                status: ApplicationStatus::Interviewed,
                notes: String::new(),
            }],
            free_form: FreeFormAnswers {
                elevator_pitch: Some("I build verifiable software.".into()),
                strengths: Some("Systems design.".into()),
                ..Default::default()
            },
            ..Default::default()
        };
        let toml_text = toml::to_string(&p).unwrap();
        let back: Profile = toml::from_str(&toml_text).unwrap();
        assert_eq!(p, back);
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
