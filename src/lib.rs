// SPDX-License-Identifier: Unlicense
// Unlicense — public domain — cochranblock.org
// Contributors: GotEmCoach, KOVA, Claude Opus 4.7

//! # atsisbroken
//!
//! The browser for filling job applications. Single Rust binary,
//! Servo-derived engine (in flight) — no Chrome dependency, no
//! extension. Cross-compiled. The user's model is built locally
//! from their own resume + per-field feedback.
//!
//! Architecturally, this crate is the brain: the Profile schema,
//! the field classifier, the run loop, the connectors, the
//! correction overlay. The browser shell + render pipeline live
//! under [`browser`] and call into the brain on every navigation
//! and field encounter. The legacy CDP path (chromiumoxide,
//! [`cdp`]) is still wired for the `run --strategy cdp-attach`
//! flow but retires once the in-tree engine renders real ATS
//! pages.
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
#[cfg(feature = "gui")]
pub mod browser;
pub mod browser_detect;
pub mod cdp;
pub mod config;
pub mod github;
pub mod learning;
pub mod run_loop;

// The kova ats_fixtures re-export and the legacy CDP e2e test were
// retired when atsisbroken pivoted to "the browser is the product"
// (Phase A series). The browser shell renders pages directly through
// its own engine; chromiumoxide-driven fixture testing belongs to the
// previous architecture. Vendor coverage is now demonstrated by the
// browser navigating to real ATS postings, not by HTML fixtures.
pub mod paths;
pub mod resume;
pub mod strategy;
#[cfg(feature = "tui")]
pub mod tui;

// In-binary tests — the cochranblock exopack pattern. Gated by
// `#[cfg(feature = "tests")]` so production builds (no `tests`
// feature) don't compile the test surface; the `atsisbroken-test`
// binary is the only consumer (it requires the feature, see
// Cargo.toml). cargo test runs un-migrated `#[cfg(test)] mod
// tests {}` blocks in their original locations; once a module's
// tests live in `src/tests/`, the original block has been
// deleted and this module owns its coverage.
#[cfg(feature = "tests")]
pub mod tests;

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
    /// Voluntary EEO / AAP demographics. None by default. Today's
    /// run loop has no demographics-fill path at all — fields are
    /// skipped by absence, not by an enforced gate. The contract
    /// when a fill path lands: setting `Some(Demographics{..})` is
    /// necessary but not sufficient; a per-run opt-in (not
    /// implemented yet) is also required. Even
    /// `Some(Demographics::default())` does NOT enable autofill on
    /// its own. Documented here so the gate is the first thing
    /// designed when the demographics path is wired.
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
    /// (Phase K, not yet shipped) is intended to fill these by
    /// quoting public material the user has already published —
    /// GitHub READMEs, blog posts, etc. — so each emitted
    /// fragment can be cited back to a public URL. Until the
    /// composer lands, the only writers are user edits; the
    /// citation invariant is design intent, not enforced today.
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

/// Cross-link target. When a `Project` corresponds to a GitHub
/// repo, `Project.github_repo` carries this so the answer composer
/// (Phase K, not yet shipped) can fetch the matching inventory
/// entry's README excerpts (paragraph-joined; see
/// [`crate::github::extract_excerpts`] for the join behavior) and
/// commit messages (byte-for-byte from the GitHub API) by
/// (owner, name).
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

/// Voluntary self-identification fields. Off by default. Today's
/// run loop has no demographics-fill path; the planned gate, when
/// it lands, requires both this struct populated AND a per-run
/// opt-in (not implemented yet). The two-key gesture is the
/// design — keep it the design when the path is wired.
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
/// response"; the (eventual) composer is allowed to synthesize
/// from inventories. `Some("")` = "user explicitly cleared this
/// slot — never autofill, even from a public-source citation."
/// Two distinct signals; the composer reads them separately when
/// it ships.
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

