<!-- Unlicense — cochranblock.org -->
<!-- Contributors: GotEmCoach, KOVA, Claude Opus 4.7 -->

# atsisbroken — Profile Schema + GitHub-driven Free-form Answers

**Date:** 2026-05-03
**Scope:** Expand the `Profile` struct to cover every category of
question real ATS forms ask. Add a GitHub inventory that powers
free-form question answers (always attributable to verbatim source).
**Predecessor:** `PLAN.md` Phase 4 (user feedback compounding) and
`PLAN_BROWSER_AUTOMATION.md` (the fill loop that consumes Profile).

---

## 1. Field inventory — what ATS forms actually ask

Surveyed across Greenhouse / Lever / Workday / Ashby / iCIMS /
Taleo / SmartRecruiters fixtures. Grouped by category. Optional
fields marked ⚪; required-where-applicable marked ●.

### 1.1 Identity

| Field                | Notes                                                   |
|----------------------|---------------------------------------------------------|
| ● legal_name         | Per government ID; some ATS distinguish from preferred  |
| ⚪ preferred_name    | "Goes by"                                                |
| ⚪ pronouns          | she/her, he/him, they/them, ze/zir, custom               |
| ⚪ date_of_birth     | International forms; some US forms (rarely)              |
| ⚪ ssn_last_four     | NEVER stored; ATS asks; we refuse to retain              |

### 1.2 Contact

| Field           | Notes                                          |
|-----------------|------------------------------------------------|
| ● email_primary | The main email                                  |
| ⚪ email_alt    | For after-grad changes                          |
| ● phone_mobile  | Cell                                            |
| ⚪ phone_alt    | Home/landline if asked                          |
| ⚪ address      | street1 / street2 / city / state / postal_code / country |

### 1.3 Online presence

| Field          | Notes                                        |
|----------------|----------------------------------------------|
| ⚪ linkedin    | Most common                                   |
| ⚪ github      | Engineering roles                             |
| ⚪ gitlab      | Some engineering roles                        |
| ⚪ bitbucket   | Rare but exists                               |
| ⚪ website     | Personal site                                 |
| ⚪ portfolio   | Designers / PMs                               |
| ⚪ blog        | Writers, devrel                               |
| ⚪ twitter     | Often asked                                   |
| ⚪ bluesky     | Rising                                        |
| ⚪ mastodon    | Rare                                          |
| ⚪ stackoverflow| Engineering roles                            |
| ⚪ devto / medium / hashnode | Writers                          |
| ⚪ youtube     | Devrel / educators                            |
| ⚪ dribbble    | Designers                                     |
| ⚪ behance     | Designers                                     |
| ⚪ artstation  | Game / creative                               |

### 1.4 Work authorization (US-centric, expanded for international)

| Field                       | Values                                                                                     |
|-----------------------------|--------------------------------------------------------------------------------------------|
| ● citizenship_country       | ISO-3166-1 alpha-3 list                                                                    |
| ● work_auth_status          | `Citizen` / `PermanentResident` / `H1B` / `OPT` / `EAD` / `TN` / `OptionalPracticalTraining` / `RequireSponsorship` / `Other(String)` |
| ● visa_sponsorship_required | bool                                                                                       |
| ⚪ security_clearance       | `None` / `PublicTrust` / `Confidential` / `Secret` / `TopSecret` / `TS_SCI` / `Other(String)` |
| ⚪ relocation_willingness   | bool + optional `Vec<RegionPreference>`                                                    |
| ⚪ remote_preference        | `Remote` / `Hybrid` / `OnSite` / `Flexible`                                                |

### 1.5 Demographics (EEO / AAP — always voluntary)

| Field              | Notes                                                                |
|--------------------|----------------------------------------------------------------------|
| ⚪ gender          | Categorical: Woman / Man / Non-binary / Prefer-not-to-say / Self-describe |
| ⚪ race_ethnicity  | Multi-select per US EEOC categories                                   |
| ⚪ veteran_status  | NotVeteran / Protected / RecentlySeparated / Disabled / etc.         |
| ⚪ disability_status | Yes / No / Prefer-not-to-say                                       |
| ⚪ lgbtq_self_id   | bool / Prefer-not-to-say                                              |

These are *always optional* in the profile and *always opt-in* per
fill. atsisbroken defaults to "do not autofill demographics fields"
even in Chaos mode unless the user explicitly enables it via a
dedicated flag (not a per-field consent — the user must explicitly
say "share my demographics with this ATS").

### 1.6 Compensation

| Field                | Notes                                                         |
|----------------------|---------------------------------------------------------------|
| ⚪ salary_expectation_min | Numeric, currency-tagged                                  |
| ⚪ salary_expectation_max | Numeric, currency-tagged                                  |
| ⚪ salary_currency       | ISO 4217                                                  |
| ⚪ compensation_notes    | Free text (e.g., "open to equity-heavy")                  |
| ⚪ desired_base / variable / equity   | Optional sub-fields                          |

### 1.7 Education (extends current `Education`)

| Field                     | Already? |
|---------------------------|----------|
| school                    | yes      |
| degree                    | yes      |
| field_of_study            | yes      |
| start_date                | yes      |
| end_date                  | yes      |
| gpa                       | yes      |
| honors                    | NEW: `Vec<String>` (Cum Laude, Phi Beta Kappa, Dean's List) |
| minor                     | NEW                                                         |
| relevant_coursework       | NEW: `Vec<String>`                                          |
| thesis_title              | NEW                                                         |
| extracurriculars          | NEW: `Vec<String>`                                          |
| location                  | NEW: city/state                                             |

### 1.8 Experience (extends current `Experience`)

| Field                       | Already? |
|-----------------------------|----------|
| company                     | yes      |
| title                       | yes      |
| start_date / end_date       | yes      |
| bullets                     | yes      |
| location                    | NEW                                              |
| employment_type             | NEW: `FullTime` / `PartTime` / `Contract` / `Internship` / `Volunteer` |
| supervisor_name             | NEW (some ATS asks)                              |
| supervisor_email            | NEW                                              |
| supervisor_phone            | NEW                                              |
| reason_for_leaving          | NEW                                              |
| can_we_contact              | NEW: bool — for reference checks                 |

### 1.9 Skills (replaces current `Vec<String>`)

```rust
struct Skill {
    name: String,
    years: Option<f32>,
    level: SkillLevel,  // Beginner / Intermediate / Advanced / Expert
    last_used: Option<String>,  // year
}
struct Language {
    name: String,
    proficiency: LanguageProficiency,  // Native / Fluent / Conversational / Beginner
}
```

| Sub-field        | Notes                                  |
|------------------|----------------------------------------|
| technical_skills | `Vec<Skill>`                           |
| languages        | `Vec<Language>`                        |
| soft_skills      | `Vec<String>` (no level)               |

### 1.10 Certifications

```rust
struct Certification {
    name: String,
    issuer: String,
    issue_date: String,
    expiry_date: Option<String>,
    credential_id: Option<String>,
    credential_url: Option<String>,
}
```

### 1.11 Projects (the GitHub-fed slot)

```rust
struct Project {
    name: String,
    description: String,
    url: Option<String>,
    start_date: Option<String>,
    end_date: Option<String>,
    technologies: Vec<String>,
    role: Option<String>,
    highlights: Vec<String>,
    /// Cross-link into the GitHub inventory if this project lives there.
    github_repo: Option<RepoRef>,
}
```

### 1.12 Publications / Patents / Awards

```rust
struct Publication {
    title: String,
    authors: Vec<String>,
    venue: String,
    date: String,
    url: Option<String>,
    doi: Option<String>,
}
struct Patent {
    title: String,
    number: String,
    issued_date: String,
    inventors: Vec<String>,
    url: Option<String>,
}
struct Award {
    name: String,
    issuer: String,
    date: String,
    description: Option<String>,
}
```

### 1.13 References

```rust
struct Reference {
    name: String,
    relationship: String,
    company: Option<String>,
    title: Option<String>,
    email: Option<String>,
    phone: Option<String>,
}
```

### 1.14 Free-form answer bank

```rust
struct FreeFormAnswers {
    elevator_pitch: Option<String>,
    why_this_role_template: Option<String>,
    biggest_technical_challenge: Option<String>,
    proudest_project: Option<String>,
    biggest_failure_and_lesson: Option<String>,
    five_year_plan: Option<String>,
    strengths: Option<String>,
    weaknesses: Option<String>,
    management_style: Option<String>,
    collaboration_example: Option<String>,
    /// Generic key→answer map for prompts not in the standard set.
    custom: std::collections::BTreeMap<String, String>,
}
```

These are user-authored. The GitHub-driven generator (Section 4) can
*propose* values for these fields, but the user reviews and approves
before they land in the bank.

### 1.15 Application history (atsisbroken's own ledger)

```rust
struct Application {
    company: String,
    role: String,
    submitted_at: String,
    url: String,
    status: ApplicationStatus,  // Submitted / Interviewed / Offer / Rejected / Ghosted / Withdrawn
    notes: Option<String>,
}
```

This is **populated automatically** as the user submits via
atsisbroken (the `awaiting submit → user pressed Enter → submitted`
moment in `PLAN_BROWSER_AUTOMATION.md`'s flow).

---

## 2. Proposed expanded `Profile` (Rust)

```rust
struct Profile {
    // ─── identity ──────────────────────────────────────
    legal_name: String,
    preferred_name: Option<String>,
    pronouns: Option<String>,

    // ─── contact ───────────────────────────────────────
    email: String,
    email_alt: Option<String>,
    phone: String,
    phone_alt: Option<String>,
    address: Option<Address>,

    // ─── online presence ───────────────────────────────
    presence: OnlinePresence,

    // ─── work authorization ────────────────────────────
    work_auth: WorkAuth,

    // ─── demographics (always optional, never auto-fill) ──
    demographics: Option<Demographics>,

    // ─── compensation ──────────────────────────────────
    compensation: Option<Compensation>,

    // ─── histories ─────────────────────────────────────
    education: Vec<Education>,
    experience: Vec<Experience>,
    projects: Vec<Project>,
    certifications: Vec<Certification>,
    publications: Vec<Publication>,
    patents: Vec<Patent>,
    awards: Vec<Award>,
    references: Vec<Reference>,

    // ─── skills ────────────────────────────────────────
    technical_skills: Vec<Skill>,
    languages: Vec<Language>,
    soft_skills: Vec<String>,

    // ─── free-form ─────────────────────────────────────
    free_form: FreeFormAnswers,

    // ─── ledger ────────────────────────────────────────
    applications: Vec<Application>,

    // ─── raw ───────────────────────────────────────────
    raw_resume_text: String,
}
```

Compatibility: the **current** `Profile` is a subset (`legal_name`
maps to existing `full_name`, etc.). A migration in
`Profile::from_v0(old) -> Profile` keeps existing TOML readable.

The new field-key vocabulary expands from 11 entries to **~75**.
The seed corpus and the model both grow accordingly. Field key
names align with snake_case Rust field names so the JSON/TOML
shape is mechanical.

---

## 3. GitHub inventory

A separate JSON file at `~/.atsisbroken/github_inventory.json`,
**not** mixed into `profile.toml` (different refresh cadence,
different privacy posture).

```rust
struct GithubInventory {
    handle: String,                       // e.g. "GotEmCoach"
    last_synced: String,                  // RFC3339 timestamp
    public_repos: Vec<RepoSnapshot>,
    contributed_to: Vec<RepoRef>,         // External repos via PRs
    language_distribution: BTreeMap<String, f32>, // lang → fraction of bytes
    total_public_commits: u32,
    follower_count: u32,
    starred_count: u32,
}

struct RepoSnapshot {
    owner: String,
    name: String,
    description: Option<String>,
    primary_language: Option<String>,
    languages: Vec<(String, u64)>,        // (lang, bytes)
    stars: u32,
    forks: u32,
    open_issues: u32,
    closed_issues: u32,
    last_commit: String,
    total_commits: u32,
    topics: Vec<String>,
    license: Option<String>,
    is_fork: bool,
    /// Excerpts (verbatim) from README.md, capped at 2 KiB per repo.
    /// Source for any free-form generation pulling from this repo.
    readme_excerpts: Vec<String>,
    /// Recent commit messages, verbatim, capped at 50 entries per repo.
    /// Used as quotable evidence in technical-challenge questions.
    recent_commit_messages: Vec<String>,
    /// Top files by line count (path, lang, lines) — purely structural.
    top_files: Vec<FileSummary>,
}

struct RepoRef {
    owner: String,
    name: String,
    url: String,
}

struct FileSummary {
    path: String,
    language: Option<String>,
    lines: u32,
}
```

**What we deliberately do not store:**
- Source code itself (only file structure + line counts)
- Issue / PR bodies (only counts)
- Email addresses found in commits
- Private repo content of any kind

---

## 4. How free-form questions get answered

The classifier already returns one of `known_key | "freetext" | "unknown"`.
For `freetext` fields, atsisbroken now has a routing layer:

```
freetext label/question → question classifier → answer generator
```

### 4.1 Question classifier (cheap, deterministic)

Map common ATS prompts to known answer slots. Pattern-based, no
neural model:

| Prompt pattern (lowercased substring)          | Answer slot                       |
|-----------------------------------------------|-----------------------------------|
| "tell us about yourself", "elevator pitch"    | `free_form.elevator_pitch`        |
| "why do you want this role", "why this position" | `free_form.why_this_role_template` |
| "biggest technical challenge", "hardest problem" | `free_form.biggest_technical_challenge` OR github-driven |
| "proudest project", "favorite project"        | `free_form.proudest_project` OR github-driven |
| "biggest failure", "lessons learned"          | `free_form.biggest_failure_and_lesson` |
| "describe a project where", "a time when"     | github-driven from a `RepoSnapshot` |
| "open source contributions"                   | github-driven from `contributed_to` |
| "team / collaboration / leadership"           | github-driven from multi-contributor repos |
| "5 year plan", "where do you see yourself"    | `free_form.five_year_plan`        |
| "strengths"                                   | `free_form.strengths`             |
| "weaknesses"                                  | `free_form.weaknesses`            |
| "management style"                            | `free_form.management_style`      |
| (everything else)                             | `unknown` → skip, never invent    |

### 4.2 GitHub-driven answer composer

When the question routes to a github-driven slot, the composer:

1. Picks a candidate `RepoSnapshot`:
   - For "proudest project": highest stars among non-fork repos, tie-break by most recent commit.
   - For "technical challenge": repo with the most `closed_issues` (proxy for problems-solved).
   - For "team work": repos with `contributors > 1` and the user's
     own commit count > 30% (real co-authorship, not drive-by).
   - For "open source contribution": from `contributed_to` list.
2. Composes the answer using **only verbatim sources**:
   - Repo name + primary language (factual).
   - One sentence from `readme_excerpts` (verbatim, quoted).
   - One commit message from `recent_commit_messages` (verbatim, quoted).
   - Star count if > 0 (factual).
3. Output template (P6 Hostile Reviewer compatible — every claim is
   sourceable):
   ```
   {repo_name} is a {primary_language} project I built and maintain.
   The README describes it as: "{readme_excerpt_sentence}".
   A recent commit captures the kind of work: "{commit_message}".
   {star_blurb_if_applicable}
   ```
4. The generated answer is **labelled as proposed** in TrainingWheels
   mode — user can accept, edit, or reject. Once accepted, it's
   cached in `free_form.custom` keyed by question hash, so the same
   question on a different ATS reuses it.

**Important contracts:**
- No paraphrasing. No "in other words." Every sentence in a
  generated answer either:
  (a) States a verifiable fact (repo name, language, star count), or
  (b) Quotes a verbatim source (README sentence, commit message,
      Profile field).
- The user reviews before submission. The constraint isn't
  "atsisbroken doesn't lie" — it's "atsisbroken makes lying
  structurally impossible."
- This is the same principle as P6 in `USER_STORY_ANALYSIS.md`.

### 4.5 Custom patterns — user-defined regex hooks

Power users (and orgs deploying atsisbroken internally) want to
extend both the **question classifier** and the **extractor** that
pulls verbatim sentences. Two kinds of user-defined regex:

#### 4.5.1 Question routes

A user-defined route maps a regex over the question text to an
answer slot. Lives in `~/.atsisbroken/custom_patterns.toml`:

```toml
[[question_route]]
name = "scaling_systems"
# Question text is lowercased before match; user writes case-insensitive
# (?i) flags or relies on the lowercasing.
pattern = "scal(e|ing|ed) .* (users|requests|throughput)"
slot = "extractor:scaling_quotes"
priority = 100   # Higher beats built-in defaults

[[question_route]]
name = "open_source_leadership"
pattern = "open source.*(maintain|lead|review)"
slot = "extractor:maintainer_quotes"
```

User-defined routes are checked **before** built-in routes when
priority ≥ 100. This lets users override "tell me about your
projects" with a more specific match.

#### 4.5.2 Extractors

An extractor pulls verbatim sentences from a named source via a
regex. The composer renders only the sentences the regex matched,
quoted, with citation:

```toml
[[extractor]]
name = "scaling_quotes"
# `source` is one of: "github:readme_excerpts",
# "github:recent_commit_messages", "profile:experience_bullets",
# "profile:raw_resume_text", "audit:freeform_audit"
source = "github:readme_excerpts"
# Matches one capture group; capture(0) is the full sentence quoted.
pattern = "[^.!?]*scaled[^.!?]*(?:users|throughput)[^.!?]*[.!?]"
# Optional: cap how many matches the composer emits
max_matches = 3

[[extractor]]
name = "maintainer_quotes"
source = "github:recent_commit_messages"
pattern = "review|approve|merge"
max_matches = 5
```

When the question classifier routes to `extractor:scaling_quotes`,
the composer:
1. Loads `github_inventory.public_repos[*].readme_excerpts`.
2. Runs the regex over each excerpt.
3. Emits the matching sentences (verbatim, quoted, with
   `(repo: <name>)` attribution).
4. Logs every match to `freeform_audit.jsonl` with the extractor
   name + source location.

#### 4.5.3 Custom Profile slots

Users can also define answer slots beyond the built-in 12:

```toml
[[answer_slot]]
name = "team_culture_take"
prompt = "What kind of team culture do you thrive in?"
# Static value the user wrote once; the question classifier maps
# matching prompts to this.
value = "Trust, async-first, written norms over verbal."
```

Built-in `FreeFormAnswers.custom: BTreeMap<String, String>` already
holds these — `custom_patterns.toml` provides a declarative way to
populate it with prompt-pattern routing.

#### 4.5.4 Safety net

User regex is sandboxed by:
- **No code execution.** Patterns are pure regex (the `regex` crate),
  no embedded scripts.
- **Verbatim-only output.** Even with custom patterns, the composer
  emits only text the regex matched in the named source. No
  paraphrasing, no rewriting.
- **Resource caps.** `max_matches` per extractor, regex compile
  timeout (10 ms), per-pattern execution timeout (50 ms across all
  sources combined).
- **Audit log entries** still tag every output with the source URL,
  the extractor name, and the matched span.

### 4.3 Why this beats LLM-driven answer generation

LLMs hallucinate. Verbatim quote-and-cite doesn't. The **honesty
mode** is the differentiator:

| Property                          | atsisbroken | LLM autofill (Simplify Pro / etc.) |
|-----------------------------------|-------------|--------------------------------------|
| Generated text traceable to source | ✓           | ✗                                   |
| Pass a recruiter audit ("did the AI write this?") | ✓ — every quote has a URL | ⚠ — generated prose looks generated |
| Behavior on unfamiliar prompts    | skip        | confabulate                         |

---

## 5. Privacy posture

### 5.1 GitHub access patterns

| Mode                  | Auth                                | Captures            |
|-----------------------|-------------------------------------|---------------------|
| Anonymous public      | none — REST API rate-limited 60/h   | public repos only   |
| Token (recommended)   | classic PAT or fine-grained, scope `public_repo`, `read:user` | Higher rate limit; same data |
| Token + private scope | adds `repo`                         | Includes private repos in the inventory (with explicit user opt-in) |

The token, when present, is stored at `~/.atsisbroken/github_token`
with `chmod 600`. Never shipped in `feedback.jsonl`. Never sent
across the Native Messaging bridge. Never committed in any sync
output.

### 5.2 What leaves the device

Nothing, by default. The GitHub API calls are **outbound from
atsisbroken to api.github.com** to *populate* the inventory. The
inventory itself stays local. The model uses it to generate answers
that go into the user's browser; no telemetry to atsisbroken's
authors, ever.

### 5.3 Free-form answer audit log

When atsisbroken composes a free-form answer, it writes a line to
`~/.atsisbroken/freeform_audit.jsonl`:

```json
{"date":"2026-05-04T10:11:12Z","question":"Describe a project...",
 "source":{"repo":"cochranblock/atsisbroken","commit_sha":"abc123",
           "readme_section":"# atsisbroken"},
 "rendered":"atsisbroken is a Rust project I build..."}
```

The user can grep this if a recruiter asks "did you write this
yourself?" The honest answer is "I quoted myself, here are the
sources" — and that audit log is the receipt.

---

## 6. Phased implementation

### Phase G — Schema migration (~2 days)

- Define the expanded `Profile` struct.
- Implement `Profile::migrate_from_v0(old: ProfileV0) -> Profile`
  for back-compat with shipped TOML.
- Update the seed corpus vocab from 11 keys to ~75 (one per
  field-name leaf).
- Tests: round-trip every new field; v0 → v1 migration produces
  the expected layout.

### Phase H — `init` walks every field group (~2 days)

- Currently `atsisbroken init` parses raw resume text. Now it
  also walks the user through each field group with sensible
  defaults from the parser:
  ```
  Identity:
    legal_name (parsed: "Jane Q. Doe")  [Enter to accept, edit, ?]
    preferred_name [skip]
    pronouns [skip]
  ```
- TUI offers a "Profile" tab (5th tab) for editing post-`init`.
- Tests: walkthrough completes with all defaults; resume-derived
  values populate; explicit "skip" leaves Optional fields None.

### Phase I — GitHub sync (~2 days)

- New subcommand: `atsisbroken connect-github [--token TOKEN]`.
- Fetches public-repo metadata via the REST API (paginated).
- Writes `~/.atsisbroken/github_inventory.json`.
- Periodic refresh: `atsisbroken sync-github` re-runs the fetch.
- Tests:
  - Mock API responses → expected `GithubInventory` shape.
  - Rate-limit handling: 60 req/h anonymous, 5000 with token.
  - Token storage chmod 600.
  - Refresh detects deleted repos and removes them from the inventory.

### Phase J — Question classifier (~1 day)

- Pattern-matched routing from question text → answer slot.
- 12 prompt patterns initially, extensible via the seed corpus.
- Tests: each pattern routes to the right slot;
  no-match → `unknown`.

### Phase K — Answer composer (~2 days)

- `compose_answer(slot, profile, github_inventory) -> Option<ComposedAnswer>`
  where `ComposedAnswer { text, sources: Vec<SourceCitation> }`.
- The renderer that produces the answer template (Section 4.2).
- Audit log entry on every composition.
- Tests:
  - Every produced sentence's tokens appear verbatim in either a
    Profile field, README excerpt, or commit message.
  - No "the user is..." style fabrication leaks (regex deny-list
    on a small vocabulary of LLM-tells: "passionate", "synergy",
    "leveraged" without source attribution).

### Phase L.5 — Custom patterns (~1 day)

- Load `~/.atsisbroken/custom_patterns.toml` if present.
- `regex::Regex` for compilation; per-pattern compile timeout 10 ms
  via `regex::RegexBuilder::size_limit` + a wall-clock guard.
- Custom question_routes consulted before built-in routes when
  `priority >= 100`; otherwise built-in wins on conflict.
- Per-extractor execution: walk the source, collect matches up to
  `max_matches`, emit verbatim quoted with `(source: <citation>)`
  suffix.
- Fail-safe: any regex compile error → log to stderr, skip that
  pattern, continue. Never crash the run.
- Tests:
  - Custom route with priority 100 overrides a built-in route.
  - Extractor against a fixture source emits expected matches.
  - Catastrophic regex (e.g. `(a+)+`) caught by size_limit; pattern
    skipped, no panic.
  - Custom answer_slot reachable via classifier.
  - Audit log entry includes extractor name + source citation.

### Phase L — TUI Profile tab + GitHub tab (~1 day)

- Profile tab: scrollable view of the structured Profile.
- GitHub tab: one row per repo, sorted by stars, showing the
  `RepoSnapshot` summary.
- Keybindings consistent with existing tabs.

---

## 7. Tests + acceptance criteria

| Property                                                      | Test                                           | Phase |
|---------------------------------------------------------------|------------------------------------------------|-------|
| v0 Profile migrates cleanly                                   | `profile_v0_migrates_to_v1`                   | G |
| All ~75 fields round-trip through TOML                        | `expanded_profile_full_round_trip`            | G |
| `init` accepts every field group's prompt then produces TOML  | `init_full_walkthrough_writes_toml`           | H |
| GitHub sync against fixture API yields expected inventory     | `sync_github_fixture_yields_inventory`        | I |
| Token file is chmod 600                                       | `github_token_file_has_user_only_perms`       | I |
| Each prompt-pattern routes to the right slot                  | `question_classifier_known_patterns`          | J |
| Unknown question routes to `unknown` (skip, never invent)     | `question_classifier_unknown_routes_to_skip`  | J |
| Composer's output tokens trace to verbatim sources            | `composed_answer_sources_are_verbatim`        | K |
| LLM-tell deny-list catches confabulation                      | `composed_answer_no_llm_tells`                | K |
| Audit log entry written on every composition                  | `composer_writes_audit_jsonl`                 | K |
| Custom route with priority ≥100 overrides built-in            | `custom_route_priority_overrides_builtin`     | L.5 |
| Catastrophic regex caught by size_limit, no panic             | `custom_pattern_catastrophic_regex_safe`      | L.5 |
| Custom extractor matches emit verbatim with citation          | `custom_extractor_emits_verbatim_with_source` | L.5 |
| Bad pattern logs but doesn't crash                            | `custom_pattern_compile_error_skipped`        | L.5 |
| Profile tab renders all top-level field groups                | `tui_profile_tab_lists_groups`                | L |
| GitHub tab sorts by stars descending                          | `tui_github_tab_sorted_by_stars`              | L |

Acceptance: a real applicant with their actual resume + a real
GitHub account can run:
```
atsisbroken init
atsisbroken connect-github --token $GH_TOKEN
atsisbroken
```
…and see every Profile field populated where data exists, the
github inventory loaded, and TUI tabs 4 (Profile) and 5 (GitHub)
fully functional. When the next ATS form asks "describe a project,"
the auto-composed answer cites a real repo and a real commit message.

---

## 8. UX surfaces (TUI)

The TUI grows from 3 tabs to 5:

```
dashboard  •  queue  •  strategy  •  profile  •  github
```

- **profile** tab: scrollable structured view; press `e` to enter
  edit mode for the cursored field; `s` saves to TOML.
- **github** tab: list view of `RepoSnapshot`; press `Enter` on
  a repo to see its readme excerpts + commit messages.

Footer hints update: `1/2/3/4/5 tab` etc. The `tabs_constant_matches_documented_count`
test expects 5.

---

## 9. Risks

| Risk                                              | Mitigation                                                                |
|---------------------------------------------------|---------------------------------------------------------------------------|
| GitHub API rate-limiting                           | Anonymous = 60/h; token bumps to 5k/h; backoff + persist partial inventory |
| User's GitHub account is mostly forks             | Composer skips `is_fork: true` repos for project answers                   |
| GitHub token leaked via screenshot or accidentally | Token file is chmod 600; never appears in any TUI tab; never in audit log |
| Composer picks a controversial repo              | User can pin `Profile::projects[*].github_repo` to override the auto-pick |
| Schema bloat slows TOML write                     | Empty optional fields serialize to `null`; profile.toml stays under 8 KiB for typical user |
| ATS demanding fields we won't autofill (e.g. SSN) | `unknown` route → skip; user fills by hand; never stored                 |

---

## 10. Open questions

1. Should the GitHub inventory include **starred repos**? (Signal
   for "things I'm interested in" in some answers.)
   - **Proposal:** yes, but separate field `starred: Vec<RepoRef>`
     with no readme excerpts. Used only for the "what tech are you
     learning" question pattern.
2. Should we support **GitLab / Codeberg / Sourcehut / Gitea**
   parallel to GitHub?
   - **Proposal:** yes via the same `GithubInventory`-shaped struct
     (rename to `GitInventory`); each forge's API client behind a
     trait; ship GitHub first, others as Phase M+.
3. **Cover-letter generation** from the same GitHub inventory?
   - **Proposal:** out of scope for this plan. Cover letters are a
     long-form artifact; the architectural approach
     (verbatim-source-attribution) extends, but the UX surface is
     different (file output vs in-form fill). Track separately.

---

## 11. Forward-link

- `BACKLOG.md` — items G–L added under "Now"
- `PLAN.md` — new Phase 4.5 between current Phase 4 (feedback
  compounding) and Phase 5 (ecosystem). Phase 4.5 ships the
  expanded Profile + GitHub inventory.
- `TIMELINE_OF_INVENTION.md` — to be updated as each phase lands.
<!-- COCHRANBLOCK-BRAND-FOOTER:START - generated by cochranblock/scripts/brand-stamp.sh -->

---

<sub>&#9656; **THE COCHRAN BLOCK, LLC** &#183; CAGE `1CQ66` &#183; UEI `W7X3HAQL9CF9` &#183; UNLICENSE &#183; [cochranblock.org](https://cochranblock.org)</sub>
<!-- COCHRANBLOCK-BRAND-FOOTER:END -->
