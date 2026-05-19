<!-- Unlicense — cochranblock.org -->
<!-- Contributors: GotEmCoach, KOVA, Claude Opus 4.7 -->

# atsisbroken — Profile Schema + Verbatim-Source Free-form Answers

**Date:** 2026-05-03 (updated 2026-05-06: Blog added as second
verbatim source, peer of GitHub.)
**Scope:** Expand the `Profile` struct to cover every category of
question real ATS forms ask. Add **two parallel verbatim sources**
that sit alongside `profile.toml` and power free-form question
answers (always attributable, audit-logged):
1. **GitHub inventory** — repo metadata, README excerpts, commit
   messages. Covers technical/factual claims.
2. **Blog inventory** — posts pulled from RSS / Atom / sitemap.
   Covers narrative, behavioral, cultural, and "tell me about a
   time when" questions that GitHub commit messages don't reach.
**Predecessor:** `PLAN.md` Phase 4 (user feedback compounding) and
`PLAN_BROWSER_AUTOMATION.md` (the fill loop that consumes Profile).
**Filename note:** kept as `PLAN_PROFILE_AND_GITHUB.md` for now;
will likely become `PLAN_PROFILE_AND_SOURCES.md` when GitLab /
Codeberg / Sourcehut / Gitea land (see §10 open question 2).

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

## 3.5 Blog inventory

A separate JSON file at `~/.atsisbroken/blog_inventory.json`,
peer of `github_inventory.json`. Same audit property: every
emitted token traces back to a dated public URL the user
authored. Different content shape: GitHub gives you facts and
verbs; blog posts give you narrative and stance.

```rust
struct BlogInventory {
    blog_url: String,                     // canonical, e.g. https://cochranblock.org
    feed_url: String,                     // discovered RSS/Atom/sitemap URL
    feed_kind: FeedKind,
    last_synced: String,                  // RFC3339
    posts: Vec<BlogPost>,
}

struct BlogPost {
    title: String,                        // verbatim
    url: String,                          // canonical post URL
    published: String,                    // RFC3339
    excerpt: String,                      // first ~280 chars, verbatim
    body_text: String,                    // full text, HTML-stripped, capped at 16 KiB
    tags: Vec<String>,                    // from feed if present
    word_count: u32,
}

enum FeedKind {
    Rss,
    Atom,
    Sitemap,                              // fell back to /sitemap.xml + per-page crawl
    Manual,                               // user supplied an explicit URL list
}
```

**Multi-blog support.** Some users have more than one — a
technical blog, a personal site, a Substack. The on-disk format
is `Vec<BlogInventory>` so multiple `connect-blog` calls
accumulate. The composer treats them as one pool but cites the
specific source per emission.

### 3.5.1 Discovery order

`atsisbroken connect-blog <url>` tries, in order:

1. `<url>/feed`               — Substack, WordPress, Ghost
2. `<url>/feed.xml`           — Hugo (default), Jekyll
3. `<url>/rss`                — older WordPress, custom
4. `<url>/rss.xml`            — many static-site generators
5. `<url>/atom.xml`           — Hugo (atom output), some Jekyll themes
6. `<url>/index.xml`          — Hugo default
7. `<url>/feeds/all.atom.xml` — Pelican, some Django blogs
8. `<url>/sitemap.xml`        — fallback: parse `<loc>` URLs and crawl

The first feed that returns 200 + parses cleanly wins. Sitemap
crawl is rate-limited (1 req/sec, configurable) and respects
`robots.txt`.

### 3.5.2 HTML-to-text extraction

For sitemap-mode crawls (no feed body), per-page extraction:

1. Fetch with `User-Agent: atsisbroken-blog-sync/<version> (+<repo url>)`
2. Parse with `scraper`; pick text from (in order):
   - `<article>` if present
   - `<main>` if present
   - largest `<div>` by visible text length
3. Strip `<script>`, `<style>`, `<nav>`, `<header>`, `<footer>`
4. Normalize whitespace; preserve paragraph breaks (one blank line)
5. Cap at 16 KiB; full text always retrievable via `url`

### 3.5.3 What we deliberately do not store

- Comments (third-party content; not the user's words)
- Drafts or unpublished posts (no public URL = no citation)
- Paywalled content (unless `--auth-cookie` opt-in; see §5)
- Embedded images, videos, or scripts
- Tracking parameters in URLs (stripped before storage)

### 3.5.4 Beyond freetext: blog-derived defaults during `init`

Phase H (`init` walks every field group) consults the blog
inventory if present:

- **Skills** — tag clusters (e.g. 23 posts tagged `rust` →
  suggest `rust` as a default `technical_skills` entry).
- **Areas of interest** — top-N tags by post count.
- **Industry / domain** — frequency-weighted topic extraction
  from titles (deterministic, no model — just bag-of-words +
  stopwords + domain whitelist).
- **Writing samples** — top-3 most-recent posts with word_count
  ≥ 500 surface as candidates for `Profile.writing_samples` (a
  new optional field) that ATS forms occasionally request.

The user reviews and accepts each suggestion; nothing auto-fills
without confirmation. This is the user-stated "can even be used
for other fields to best present the person to the ATS machine"
goal — blog content informs structured Profile fields, not just
freetext essays.

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
| "tell us about yourself", "elevator pitch"    | `free_form.elevator_pitch` OR blog-driven (recent intro/about post) |
| "why do you want this role", "why this position" | `free_form.why_this_role_template` |
| "biggest technical challenge", "hardest problem" | `free_form.biggest_technical_challenge` OR github-driven |
| "proudest project", "favorite project"        | `free_form.proudest_project` OR github-driven |
| "biggest failure", "lessons learned"          | `free_form.biggest_failure_and_lesson` OR blog-driven (post-mortem post) |
| "describe a project where", "a time when"     | github-driven from a `RepoSnapshot`, OR blog-driven from a narrative post |
| "open source contributions"                   | github-driven from `contributed_to` |
| "team / collaboration / leadership"           | github-driven from multi-contributor repos OR blog-driven (leadership/team posts) |
| "engineering philosophy", "how you approach", "your methodology" | blog-driven (philosophy/principles posts) |
| "what have you learned recently", "growth"    | blog-driven (recent posts tagged `learning`/`tutorial`/`til`) |
| "tell me about a time when"                   | blog-driven (narrative posts) OR github-driven |
| "what do you do outside work", "interests"    | blog-driven (off-topic / personal posts) |
| "5 year plan", "where do you see yourself"    | `free_form.five_year_plan`        |
| "strengths"                                   | `free_form.strengths`             |
| "weaknesses"                                  | `free_form.weaknesses`            |
| "management style"                            | `free_form.management_style` OR blog-driven (management posts) |
| (everything else)                             | `unknown` → skip, never invent    |

When two sources are eligible (e.g., GitHub + Blog both have
candidates), the composer picks by **information density**:
GitHub for technical/factual claims, Blog for narrative/stance.
Tie-break by recency, then by source-confidence score (more
matches in the source = higher confidence).

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

### 4.2.5 Blog-driven answer composer

Same contract as §4.2 (verbatim, cited, no paraphrasing) — different
candidate-selection logic and different output template.

When the question routes to a blog-driven slot, the composer:

1. Picks a candidate `BlogPost` (or set of posts):
   - For "tell us about yourself" / "intro": post URL containing
     `about` / `intro` / `hello`, OR most-recent post tagged `intro`.
   - For "engineering philosophy" / "approach": posts tagged
     `philosophy` / `principles` / `methodology` / `approach`, OR
     posts whose title matches `^(how|why|what)\s+I\s+`.
   - For "tell me about a time when" / narrative: posts tagged
     `story` / `lesson` / `experience` / `retrospective`, OR posts
     whose `body_text` contains a `## ` heading + > 500 words after
     it (heuristic for narrative shape).
   - For "what have you learned": posts tagged `til` / `learning` /
     `tutorial` / `notes`, ranked by `published` desc.
   - For "biggest failure" / "post-mortem": posts tagged `postmortem`
     / `incident` / `mistake` / `failure`, OR title containing
     `lessons from` / `what I learned from`.
2. Composes verbatim:
   - One sentence from the post's title (verbatim, quoted).
   - One verbatim sentence pulled from `body_text` — the first
     sentence after the first `<h2>` / `## ` heading, or the second
     paragraph if no heading. Verbatim, quoted.
   - Citation: post URL + `published` date.
3. Output template:
   ```
   On {published_date} I wrote a post titled "{title}".
   From the post: "{verbatim_sentence}".
   Source: {url}.
   ```
4. Same TrainingWheels-mode review flow as §4.2: user accepts,
   edits, or rejects; accepted answers cache in `free_form.custom`
   keyed by question hash.

**Why blog content needs a different composer from GitHub:**

GitHub-driven answers are *structural* — repo name, stars, language,
commit message. They prove "the work exists." Blog-driven answers
are *narrative* — the user already chose how to phrase a story; the
composer just picks which story matches. The verbatim contract is
the same; the question-shape it answers is different.

**Citation density.** GitHub answers cite (repo, commit_sha,
optional readme_section). Blog answers cite (post URL, published
date, paragraph offset). The audit log entry for a blog-driven
answer is structurally identical to GitHub's — recruiters reading
`freeform_audit.jsonl` see consistent shape regardless of source.

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
# `source` is one of:
#   "github:readme_excerpts"
#   "github:recent_commit_messages"
#   "profile:experience_bullets"
#   "profile:raw_resume_text"
#   "blog:posts.title"
#   "blog:posts.excerpt"
#   "blog:posts.body_text"
#   "blog:posts.tags"
#   "audit:freeform_audit"
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

### 5.2.5 Blog access patterns

| Mode                 | Auth                         | Captures                     |
|----------------------|------------------------------|------------------------------|
| Public feed          | none                         | All published posts          |
| Public sitemap crawl | none, respects robots.txt    | All `<loc>`-listed pages     |
| Substack paid        | `--auth-cookie <cookie>` opt-in only | Paid posts (separate file, chmod 600) |

Blog content is already public by definition (the user published
it under a URL anyone can fetch). The fetch is **outbound from
atsisbroken to the user's blog host** (or their feed
aggregator). No auth is required for any fully-public feed. The
sitemap-fallback crawl is **rate-limited to 1 req/sec** by default
and **respects `robots.txt`** — same posture as `curl --robots`.

**Substack with paid posts.** The user can opt in via
`atsisbroken connect-blog https://example.substack.com --auth-cookie "$COOKIE"`.
Paid post content is stored in a separate file
`~/.atsisbroken/blog_inventory_paid.json` with `chmod 600`. The
composer treats it as one more verbatim source, but the audit
log marks paid-source citations explicitly so a recruiter
reading the audit knows the source was paywalled.

### 5.3 Free-form answer audit log

When atsisbroken composes a free-form answer, it writes a line to
`~/.atsisbroken/freeform_audit.jsonl`:

```json
{"date":"2026-05-04T10:11:12Z","question":"Describe a project...",
 "source":{"kind":"github","repo":"cochranblock/atsisbroken",
           "commit_sha":"abc123","readme_section":"# atsisbroken"},
 "rendered":"atsisbroken is a Rust project I build..."}
{"date":"2026-05-04T10:14:08Z","question":"Engineering philosophy?",
 "source":{"kind":"blog","url":"https://cochranblock.org/posts/why-rust",
           "published":"2026-02-14T00:00:00Z","paragraph_offset":2},
 "rendered":"On 2026-02-14 I wrote a post titled \"Why Rust\". From the post: \"...\""}
```

The user can grep this if a recruiter asks "did you write this
yourself?" The honest answer is "I quoted myself, here are the
sources" — and that audit log is the receipt. Both source kinds
(`github`, `blog`) follow the same envelope so downstream tools
parse them uniformly.

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

### Phase I.5 — Blog sync (~2 days)

- New subcommand: `atsisbroken connect-blog <url>` (re-runnable to
  add additional blogs; on-disk format is `Vec<BlogInventory>`).
- Discovery: try the 8 standard feed paths in §3.5.1 in order;
  fall back to `<url>/sitemap.xml` + per-page crawl.
- Parsing: `feed-rs` for unified RSS/Atom; `quick-xml` for sitemap;
  `scraper` for per-page HTML-to-text extraction.
- Crawl etiquette: `User-Agent: atsisbroken-blog-sync/<ver> (+<repo>)`,
  1 req/sec default, respects `robots.txt`, configurable via
  `~/.atsisbroken/config.toml` `[blog_sync] rate_limit_per_sec = N`.
- HTML-to-text: prefer `<article>` → `<main>` → largest `<div>`;
  strip `<script>` / `<style>` / `<nav>` / `<header>` / `<footer>`;
  preserve paragraph breaks; cap each post's `body_text` at 16 KiB.
- URL hygiene: strip tracking params (`utm_*`, `fbclid`, `gclid`)
  before storage so citations don't leak attribution chains.
- Refresh: `atsisbroken sync-blog` re-runs incrementally (HTTP
  HEAD with `If-Modified-Since` / `If-None-Match` per known post);
  removes posts whose URLs 404 on refresh.
- Substack paid mode: `--auth-cookie` opt-in writes
  `~/.atsisbroken/blog_inventory_paid.json` chmod 600.
- Tests:
  - Mock RSS feed (canonical WordPress shape) → expected
    `BlogInventory` shape with all post fields populated.
  - Mock Atom feed (Hugo default shape) → expected shape.
  - Sitemap fallback when no feed: parses `<loc>` URLs and crawls
    each (mocked HTTP responses).
  - HTML-to-text strips scripts/styles, preserves paragraphs.
  - Substack-shaped feed parses (`https://*.substack.com/feed`).
  - `robots.txt` Disallow respected (skip + warn, don't crash).
  - Rate limit honored under load (no >1 req in any 1s window).
  - Multi-blog: two `connect-blog` calls produce
    `Vec<BlogInventory>` with both entries; no clobber.
  - Paid-mode file chmod 600.
  - URL tracking params stripped on storage.
  - Refresh removes 404'd posts; keeps 200'd posts unchanged.

### Phase J — Question classifier (~1 day)

- Pattern-matched routing from question text → answer slot.
- ~17 prompt patterns initially (12 base + 5 blog-routed; see §4.1
  table), extensible via the seed corpus.
- Per-pattern source preference: which inventory to consult first
  (Profile static → GitHub → Blog), with fall-through to the next
  source if the preferred source has no candidate.
- Tests:
  - Each pattern routes to the right slot.
  - No-match → `unknown`.
  - Source-preference ordering honored (e.g., a "philosophy"
    question prefers Blog over GitHub even when both are present).
  - Fall-through: if a Blog-preferred prompt fires but no blog
    inventory exists, falls through to GitHub or `unknown`
    (never makes up a source).

### Phase K — Answer composer (~2 days)

- `compose_answer(slot, profile, github_inventory, blog_inventory) -> Option<ComposedAnswer>`
  where `ComposedAnswer { text, sources: Vec<SourceCitation> }` and
  `SourceCitation` is an enum over `{ Github(...), Blog(...), Profile(...) }`.
- Two renderer paths: GitHub template (§4.2 step 3) and Blog
  template (§4.2.5 step 3). Selected by source kind, not by slot,
  so a slot routed to Blog renders with blog citation shape.
- Audit log entry on every composition; `source.kind` field
  distinguishes `github` / `blog` / `profile`.
- Tests:
  - Every produced sentence's tokens appear verbatim in either a
    Profile field, README excerpt, commit message, or blog post
    body — across all three source types.
  - No "the user is..." style fabrication leaks (regex deny-list
    on a small vocabulary of LLM-tells: "passionate", "synergy",
    "leveraged" without source attribution).
  - Blog-rendered output cites URL + `published` date.
  - GitHub-rendered output cites repo + commit_sha.
  - Mixed-source answers (e.g., Profile fact + Blog quote) cite
    both in the audit log.

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

### Phase L — TUI Profile tab + Sources tab (~1 day)

- **Profile tab** (4th): scrollable view of the structured Profile;
  `e` enters edit mode for the cursored field, `s` saves to TOML.
- **Sources tab** (5th): unified view of GitHub + Blog inventories,
  with two collapsible sections (`[ github ]`, `[ blog ]`).
  - GitHub section: one row per repo, sorted by stars desc.
    `Enter` shows readme excerpts + commit messages.
  - Blog section: one row per post, sorted by `published` desc.
    `Enter` shows the post excerpt + tags + URL.
  - `g` / `b` jump between sections.
- Keybindings consistent with existing tabs.

**Why one Sources tab and not two.** Six tabs is cramped on
narrow terminals; collapsing GitHub + Blog into "Sources" keeps
the TUI at 5 tabs (matches the existing
`tabs_constant_matches_documented_count` test) while preserving
the per-source structure. Future GitLab / Codeberg additions
slot in as additional sections under Sources without inflating
the tab strip.

---

## 7. Tests + acceptance criteria

| Property                                                      | Test                                           | Phase |
|---------------------------------------------------------------|------------------------------------------------|-------|
| v0 Profile migrates cleanly                                   | `profile_v0_migrates_to_v1`                   | G |
| All ~75 fields round-trip through TOML                        | `expanded_profile_full_round_trip`            | G |
| `init` accepts every field group's prompt then produces TOML  | `init_full_walkthrough_writes_toml`           | H |
| Blog tag clusters seed `init` skill suggestions               | `init_blog_tags_seed_skill_defaults`          | H |
| GitHub sync against fixture API yields expected inventory     | `sync_github_fixture_yields_inventory`        | I |
| Token file is chmod 600                                       | `github_token_file_has_user_only_perms`       | I |
| Blog sync against fixture RSS feed yields expected inventory  | `sync_blog_rss_fixture_yields_inventory`      | I.5 |
| Blog sync against fixture Atom feed yields expected inventory | `sync_blog_atom_fixture_yields_inventory`     | I.5 |
| Sitemap fallback works when no feed exists                    | `sync_blog_sitemap_fallback_crawls_pages`     | I.5 |
| HTML-to-text strips scripts/styles, preserves paragraphs      | `blog_html_to_text_clean_extraction`          | I.5 |
| Substack feed shape parses correctly                          | `sync_blog_substack_feed_shape`               | I.5 |
| `robots.txt` Disallow respected                               | `sync_blog_respects_robots_txt`               | I.5 |
| Rate limit honored: never >1 req in any 1s window             | `sync_blog_rate_limit_honored`                | I.5 |
| Multi-blog: two `connect-blog` calls accumulate, no clobber   | `sync_blog_multi_blog_accumulates`            | I.5 |
| Paid-mode inventory file is chmod 600                         | `blog_inventory_paid_file_has_user_only_perms` | I.5 |
| URL tracking params stripped on storage                       | `blog_post_url_tracking_params_stripped`      | I.5 |
| Refresh removes 404'd posts                                   | `sync_blog_refresh_drops_dead_posts`          | I.5 |
| Each prompt-pattern routes to the right slot                  | `question_classifier_known_patterns`          | J |
| Unknown question routes to `unknown` (skip, never invent)     | `question_classifier_unknown_routes_to_skip`  | J |
| Source-preference ordering honored across Profile/GitHub/Blog | `question_classifier_source_preference_order` | J |
| Fall-through when preferred source missing                    | `question_classifier_fallthrough_when_missing` | J |
| Composer's output tokens trace to verbatim sources            | `composed_answer_sources_are_verbatim`        | K |
| Blog-rendered output cites URL + published date               | `composed_answer_blog_cites_url_and_date`     | K |
| GitHub-rendered output cites repo + commit_sha                | `composed_answer_github_cites_repo_and_sha`   | K |
| Mixed-source answers cite all sources in audit log            | `composed_answer_mixed_source_audit_complete` | K |
| LLM-tell deny-list catches confabulation                      | `composed_answer_no_llm_tells`                | K |
| Audit log entry written on every composition                  | `composer_writes_audit_jsonl`                 | K |
| Custom route with priority ≥100 overrides built-in            | `custom_route_priority_overrides_builtin`     | L.5 |
| Catastrophic regex caught by size_limit, no panic             | `custom_pattern_catastrophic_regex_safe`      | L.5 |
| Custom extractor matches emit verbatim with citation          | `custom_extractor_emits_verbatim_with_source` | L.5 |
| Custom extractor over `blog:posts.body_text` source works     | `custom_extractor_blog_source_emits_matches`  | L.5 |
| Bad pattern logs but doesn't crash                            | `custom_pattern_compile_error_skipped`        | L.5 |
| Profile tab renders all top-level field groups                | `tui_profile_tab_lists_groups`                | L |
| Sources tab GitHub section sorts by stars descending          | `tui_sources_github_section_sorted_by_stars`  | L |
| Sources tab Blog section sorts by published descending        | `tui_sources_blog_section_sorted_by_date`     | L |

Acceptance: a real applicant with their actual resume + a real
GitHub account + a personal blog can run:
```
atsisbroken init
atsisbroken connect-github --token $GH_TOKEN
atsisbroken connect-blog https://example.com
atsisbroken
```
…and see every Profile field populated where data exists, the
GitHub inventory loaded, the Blog inventory loaded, and TUI
tabs 4 (Profile) and 5 (Sources) fully functional. When the next
ATS form asks "describe a project," the auto-composed answer
cites a real repo and a real commit message. When it asks "what's
your engineering philosophy," the answer cites a real blog post
URL and date.

---

## 8. UX surfaces (TUI)

The TUI grows from 3 tabs to 5:

```
dashboard  •  queue  •  strategy  •  profile  •  sources
```

- **profile** tab: scrollable structured view; press `e` to enter
  edit mode for the cursored field; `s` saves to TOML.
- **sources** tab: two collapsible sections side-by-side or
  stacked depending on width:
  - `[ github ]` — list of `RepoSnapshot`, sorted by stars desc.
    `Enter` on a repo shows readme excerpts + commit messages.
  - `[ blog ]` — list of `BlogPost`, sorted by `published` desc.
    `Enter` on a post shows excerpt + tags + URL.
  - `g` jumps to GitHub section; `b` jumps to Blog section.
  - Future GitLab / Codeberg additions slot in as additional
    sections without changing tab count.

Footer hints update: `1/2/3/4/5 tab` etc. The `tabs_constant_matches_documented_count`
test expects 5.

---

## 9. Risks

| Risk                                              | Mitigation                                                                |
|---------------------------------------------------|---------------------------------------------------------------------------|
| GitHub API rate-limiting                           | Anonymous = 60/h; token bumps to 5k/h; backoff + persist partial inventory |
| User's GitHub account is mostly forks             | Composer skips `is_fork: true` repos for project answers                   |
| GitHub token leaked via screenshot or accidentally | Token file is chmod 600; never appears in any TUI tab; never in audit log |
| Composer picks a controversial repo / blog post  | User can pin `Profile::projects[*].github_repo` to override; blog-side, user can mark URLs `excluded_urls: Vec<String>` in `~/.atsisbroken/blog_excludes.toml` |
| Schema bloat slows TOML write                     | Empty optional fields serialize to `null`; profile.toml stays under 8 KiB for typical user |
| ATS demanding fields we won't autofill (e.g. SSN) | `unknown` route → skip; user fills by hand; never stored                 |
| Blog feed format changes silently (vendor moves CMS) | Try the 8 standard feed paths in order on every `sync-blog`; sitemap fallback; persist last-known-good inventory and warn on diff |
| Paywalled content surfaces in answers without consent | Paid-mode requires explicit `--auth-cookie`; paid inventory is a separate file; audit log marks paid-source citations explicitly |
| Blog content drifts from current self (old opinions) | `last_synced` and per-post `published` displayed in TUI; user can mark stale posts `excluded_urls`; composer warns when a cited post is >2 years old |
| Aggressive sitemap crawl looks like scraping     | 1 req/sec default; respects `robots.txt`; user-agent identifies the tool + repo URL; configurable rate limit |
| Bloated `body_text` per post (10k-word essays)   | 16 KiB cap per post with `…` truncation marker; full text always retrievable via `url`; cap configurable in `[blog_sync] max_body_kib = N` |

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
     trait; ship GitHub first, others as Phase M+. When this
     lands, this doc renames to `PLAN_PROFILE_AND_SOURCES.md`.
3. **Cover-letter generation** from the same source pool
   (GitHub + Blog)?
   - **Proposal:** still out of scope for this plan, but blog
     content makes cover-letter generation strictly more powerful
     than GitHub alone. Track separately as a future Phase M.
4. **Multi-blog support** — user has a tech blog + personal blog
   + Substack newsletter?
   - **Proposal:** yes from day one. On-disk format is
     `Vec<BlogInventory>`; each `connect-blog` call appends.
     Citations always identify which blog the post came from.
     This is the existing design.
5. **`body_text` cap per post** — some bloggers write 10k-word
   essays; should we store full text or truncate?
   - **Proposal:** cap at 16 KiB per post with `…` truncation
     marker; full text always retrievable via `url`. Configurable
     via `[blog_sync] max_body_kib = N`.
6. **Newsletter / podcast transcripts** as a third verbatim
   source kind?
   - **Proposal:** defer. Substack newsletters fit under blog
     (same RSS feed shape). Podcast transcripts are a different
     fetch+parse pipeline (would need YouTube / RSS-with-MP3
     handling). Track as Phase M+.
7. **Blog post recency cap** — should the composer prefer recent
   posts over older ones, or treat all equally?
   - **Proposal:** recency-weighted ranking with half-life of
     ~18 months. Older posts still eligible but require higher
     pattern match strength to surface. Tunable.

---

## 11. Forward-link

- `BACKLOG.md` — items G, H, I, **I.5**, J, K, L.5, L under
  "Now — Profile + GitHub cluster".
- `PLAN.md` — Phase 4.5 between Phase 4 (feedback compounding)
  and Phase 5 (ecosystem). Phase 4.5 ships the expanded Profile
  + GitHub inventory + Blog inventory + verbatim-source composer.
- `TIMELINE_OF_INVENTION.md` — to be updated as each phase lands.
- `HIRING_MANAGER_ANALYSIS.md` — already references this plan;
  recruiter-audit posture extends naturally to blog citations
  (URL + date is even more legible than commit_sha).
<!-- COCHRANBLOCK-BRAND-FOOTER:START - generated by cochranblock/scripts/brand-stamp.sh -->

---

<sub>&#9656; **THE COCHRAN BLOCK, LLC** &#183; CAGE `1CQ66` &#183; UEI `W7X3HAQL9CF9` &#183; UNLICENSE &#183; [cochranblock.org](https://cochranblock.org)</sub>
<!-- COCHRANBLOCK-BRAND-FOOTER:END -->
