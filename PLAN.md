# atsisbroken — The Plan

The plan to be all plans. Engineering, distribution, narrative,
community, and the "ironically free, attracts interested parties"
business model — all in one document so the strategy is legible to
anyone who clones the repo.

Author: Michael Cochran (GotEmCoach), The Cochran Block, LLC.
Status as of `aacb982` (2026-05-03).

---

## 0. Mission, in one sentence

Build a free, local-first, no-account, no-cloud ATS form-filler that
the user trains themselves — and let the product's existence do the
work that a sales team would do for a paid product.

The product is free **forever**. The Unlicense is non-negotiable. The
"interested parties" are not customers paying for the product — they
are people whose interest in the *author* and the *Cochran Block
portfolio* is unlocked by the product's existence.

---

## 1. Engineering plan

Phased to ship a usable thing every two weeks. Each phase produces a
GitHub Release and a public artifact someone could hold up.

### Phase 0 — Scaffold (DONE)

- Workspace, single binary, Diamond profiles. ✅
- Schemas (`Profile`, `FieldDescriptor`, `Mode`, `Feedback`,
  `Observation`, `FeedbackQueue`). ✅
- Chrome extension + Native Messaging bridge. ✅
- Android scaffold (JNI cdylib + Kotlin WebView). ✅
- 51 tests, exopack TRIPLE SIMS gate green. ✅
- Provenance docs (`TIMELINE_OF_INVENTION`, `PROOF_OF_ARTIFACTS`,
  `USER_STORY_ANALYSIS`, `LICENSE-PROVENANCE`, `BACKLOG`,
  `CONTRIBUTORS`). ✅

### Phase 1 — "Hello, real form" (~80% DONE)

The product fills *one* real ATS form end-to-end on the developer's
own machine. Demoable.

1. ✅ Persist `FeedbackQueue` to `~/.atsisbroken/feedback.jsonl`. (`9ebd1ad`)
2. ✅ `atsisbroken install-bridge` — writes the Native Messaging
   manifest per OS, accepts extension ID as a flag. (`9ebd1ad`)
3. ✅ `atsisbroken init` — parses pasted resume into `Profile`, saves
   TOML. (`9ebd1ad`)
4. ✅ Strategy ladder + CDP attach proof-of-life. `atsisbroken run`
   auto-picks the best tier for the environment; CDP attach lists
   tabs but doesn't yet write into the DOM. The destructive fill
   cycle is the **only** Phase 1 item still open. (`aa93f81`)
5. ⬜ Capture a 60-second demo video filling a Greenhouse form. Post it.

### Phase 1.5 — Browser automation (Detailed in `docs/PLAN_BROWSER_AUTOMATION.md`)

The TUI launches the user's browser of choice, navigates to a URL,
classifies the form, and autofills fields per the user's mode.
Six sub-phases (A through F), each shipping green individually.

- A   default-browser detection per OS (xdg-settings / LSCopy /
      HKCU UserChoice)
- A.5 install Chromium when missing (portable download, zero-admin;
      package-manager fallback with explicit confirmation; never
      auto-sudo)
- B   subprocess launcher with random debug port + tempdir
- C   CDP `Session` (chromiumoxide WebSocket) + form snapshot
- D   per-field fill + verify (Workday re-render defense)
- E   TUI Browse tab (4th tab) for the user-facing flow
- F   `atsisbroken go <url>` for scripting

Anti-bot posture: visible window, no auto-submit, default
`--rate-limit=6/min`, per-domain allowlist with prompt for
unknown domains.

### Phase 2 — Real classifier (target: end of week 3)

Replace the keyword pre-filter with a trained logistic-regression
classifier over hand-engineered features.

6. Convert Chromium's `chrome/test/data/autofill/heuristics` corpus
   into our JSONL training format (~1,500–3,000 high-quality pairs).
7. Train pipeline: `atsisbroken train --corpus pairs.jsonl` (gated
   behind `--features train`). SGD on a sparse feature vector. Output:
   `~/.atsisbroken/model.json`.
8. Online updater: each `Feedback` / `Observation` event applies one
   SGD step to the loaded model.
9. Wire `ConfidenceThreshold` into the run loop — `Mode::Shadow`
   becomes real (not just a documented mode).
10. Benchmark against a held-out chunk of the Chromium corpus. Publish
    the F1 number.

### Phase 3 — Cross-platform release (target: end of week 5)

11. GitHub Actions: cross-compile `--profile=diamond-edge` for the
    four target triples. Sign macOS builds (Apple Developer ID — has
    cost; one-time annual). Attach to a versioned GitHub Release.
12. `master` → `main` rename for parity with the rest of cochranblock.
13. Chrome Web Store submission. The store listing IS marketing
    surface — it's where Simplify.us users discover us.
14. Android: wire `cargo-ndk`, build the APK, ship to Google Play
    internal testing track.

### Phase 4 — User feedback compounding (target: end of week 8)

15. `atsisbroken sync-config` + opt-in `FeedbackDelivery::SendWhenOnline`.
    Default stays `LocalOnly`.
16. Public training-data PR template — accepted user contributions land
    in `assets/seed-corpus.jsonl` via PR. Each contribution is auditable.
17. `atsisbroken graduate` (TrainingWheels → Shadow) and
    `--chaos` (Shadow → Chaos) with confirmation prompts.
18. UX sim re-runs against the live app screenshots (not stub output).

### Phase 4.5 — Profile schema + GitHub answers (Detailed in `docs/PLAN_PROFILE_AND_GITHUB.md`)

The profile grows from 11 fields to ~75 to cover every category of
ATS question. A GitHub inventory lives alongside profile.toml; free-
form answers are composed verbatim from the inventory + profile,
attributable to commit messages, README sentences, and Profile
field values. Users extend via `custom_patterns.toml` (regex-based
question routes + extractors), all sandboxed (no exec, verbatim
output only, regex-size limits).

Sub-phases G–L:
  G   Expanded `Profile` + v0→v1 migration
  H   `init` walks every field group
  I   `connect-github` + `sync-github`
  J   Question classifier
  K   Answer composer + audit log
  L.5 Custom patterns hook
  L   TUI Profile + GitHub tabs (5-tab nav)

### Phase 5 — Ecosystem (target: end of week 12)

19. Public roadmap on GitHub Projects so contributors can self-assign.
20. "I trained a custom classifier on my own job hunt" blog post by
    the author. Honest numbers (accuracy, time saved, fields covered).
21. Per-vendor compatibility matrix (Workday / Greenhouse / Lever /
    iCIMS / Taleo / Eightfold). Contributor PRs add fixtures + tests
    for each new vendor.

---

## 2. Distribution plan

The product distributes itself when it has three properties:

(a) **One-click installable** — Chrome Web Store + GitHub Release.
(b) **Demoable in 60 seconds** — short video showing a Workday form
    fill itself. No talking-head.
(c) **Provably honest** — the source is right there. The `LICENSE-
    PROVENANCE.md`, `PROOF_OF_ARTIFACTS.md`, and `TRAINING_DATA.md`
    pre-empt every "but is it really private?" question.

### Channels (in priority order)

1. **Show HN: atsisbroken — A free, local-first alternative to Simplify**
   The HN audience IS the user. Privacy hawks (P2), indie devs (P3),
   power users (P7). Time the post for a Tuesday morning ET.
2. **Product Hunt** — same week. Different audience (less technical,
   more startup-y), broader reach.
3. **Hacker News follow-up posts** — write technical Show HNs about
   *components*, not the product. Examples:
   - "Show HN: I trained a 200KB form-field classifier from a Chromium
     test corpus" (the model)
   - "Show HN: A Native Messaging bridge between a Chrome extension
     and a Rust CLI" (the bridge)
   - "Show HN: Auto-screenshot UX simulation via headless Chromium
     in 80 lines of bash" (the screenshot script)
   Each post is its own surface, each links back to the main repo.
4. **Reddit:** r/cscareerquestions, r/recruitinghell, r/jobs,
   r/ExperiencedDevs, r/rust. Tailor the post per sub.
5. **Twitter/X + Bluesky + LinkedIn** — short demo video. No threads,
   no jargon. The product speaks louder than a marketing pitch.
6. **Privacy press**: The Markup, Wired's privacy desk, EFF Deeplinks.
   Pitch angle: "Engineer fights ATS hellscape with a free, no-cloud,
   no-account browser tool."
7. **Career-counselor newsletters**: ASK Network (career-counselor
   association), university career centers, veteran transition orgs.
   Lena (P4 persona) is *the* multiplier — one counselor with 50
   clients = 50 users per touch.
8. **Conferences (cheap)**: lightning talks at RustConf, Open Source
   Summit, BSidesXxx. The talk title is the marketing.

### Cadence

- Day 1 of public launch: HN + PH + r/cscareerquestions + Twitter
  thread + LinkedIn post. One coordinated push.
- Week 2: technical Show HN #1.
- Week 4: technical Show HN #2.
- Week 6: privacy-press pitches go out.
- Week 8: counselor-newsletter outreach.
- Month 3: conference talks announced.

---

## 3. Narrative plan

Three messages. Repeated everywhere. Never diluted.

### Message 1: "ATS is broken."
The product name is the thesis. Don't soften it. Don't bury it.
Every README, every blog post, every store listing leads with it.

### Message 2: "Free forever, by design."
Not "free tier." Not "free for now." The Unlicense is the contract
with the user. This is the differentiator from Simplify.us — the
moment the user thinks "but they'll lock me out later," they bounce.

### Message 3: "You train it. Your data. Your model. Your machine."
This is what stops the "is this just another telemetry trap" question
before it's asked. The on-device-trained classifier is BOTH the
technical architecture AND the marketing pitch. They're the same thing.

### Content calendar (rough)

- Launch post (Cochran Block blog): "I built atsisbroken because
  Simplify.us asked me to pay $19/mo to autofill forms with my own
  resume."
- Technical post #1: "How atsisbroken's 200KB on-device classifier
  beats heuristics on real ATS forms" — published with benchmark
  numbers.
- Technical post #2: "How the Chrome extension talks to a Rust
  binary without leaking a byte to the internet."
- Op-ed pitch: "The case for browser tools that don't phone home."
  Submit to Wired/The Markup.
- 60-second demo videos × 3, posted to YouTube + Twitter:
  - One Workday fill.
  - One Greenhouse fill.
  - "From paste-resume to first auto-fill in 90 seconds."

---

## 4. Community plan

The repo is the community. Treat it that way.

### Governance

- Single maintainer (GotEmCoach) on `main`. No committee.
- Decisions documented in `BACKLOG.md` and commit messages, not in a
  Discord nobody reads.
- PRs follow the repo's conventions (Unlicense header, exopack
  TRIPLE SIMS green, no self-licking tests).

### Contributor onboarding

- `CONTRIBUTORS.md` is the registry. Every PR adds a line.
- "Good first issues" tagged in GitHub Issues. Examples:
  - "Add a fixture for `<vendor>` to the test corpus."
  - "Translate seed corpus to French/German/Spanish/etc."
  - "Wire the popup's 'This was wrong' button."
- Hard rule: no PR that breaks the "free, local, no-cloud" thesis.
  Politely closed with a link to `LICENSE-PROVENANCE.md`.

### Communication

- GitHub Issues for bugs and feature requests.
- GitHub Discussions for "how do I" questions.
- A Matrix room (`#atsisbroken:matrix.org`) for real-time, federated,
  no-Discord-required chat.
- Mailing list optional, only if traffic demands it.

---

## 5. The "interested parties" mechanism

Users pay nothing. The product itself generates leads for *adjacent*
opportunities the author can monetize. Each is a public funnel; none
of them violate the free-forever thesis.

### Funnel A — Hiring leads for The Cochran Block, LLC

Every fill of every form passes through code GotEmCoach wrote. The
repo IS the resume. Engineers, PMs, and founders evaluating "is this
person good?" find a Rust workspace with TRIPLE SIMS, on-device
classifier, native messaging bridge, Android JNI scaffold, and a real
GitHub Release. Inbound: contract gigs, advisory roles, fractional
CTO, full-time offers.

**Trigger:** repo passes 1k stars, or a Show HN hits the front page.
**Action:** put "Available for X" line on the repo's About page.

### Funnel B — Acquirer interest

Free products with viral growth are acquired regularly (Tweetbot,
Sunrise, Tellybug, etc.). Even with no revenue, the user count is
the asset. Buyers are: career platforms (Indeed, ZipRecruiter),
HR tech (Workday — yes, ironically — Greenhouse, Lever), or
privacy-positioned consumer brands (Proton, Mullvad).

**Trigger:** ≥100k Chrome Web Store users.
**Action:** the conversation is initiated by them. No outreach.
Position: free-forever in any deal — sale of the *brand* and the
*author's services*, not the *Unlicense'd code*.

### Funnel C — Enterprise white-label

Career counseling orgs, university career centers, veteran transition
nonprofits, defense-contractor recruiting groups. They want a
deployable, brandable, locally-trained autofill for their own
clients. The Unlicense lets them ship it under their own name. The
Cochran Block sells *implementation* services on top.

**Trigger:** any inbound from a recognizable org.
**Action:** consultation rate card, $X/hour, $Y/project.

### Funnel D — Government / DARPA / GSA

The Cochran Block already has DARPA whitepapers in the cochranblock
repo. Federal hiring is broken in well-documented ways; USAJOBS is
public-domain; a free, auditable, on-device tool fits the
"government wants public-domain stack" narrative. atsisbroken
becomes a referenced artifact in a future grant proposal: "see also,
working public-domain implementation at github.com/cochranblock/atsisbroken."

**Trigger:** next DARPA / NSF / GSA RFP that touches federal
employment platforms.
**Action:** cite this repo in the proposal. Link to the on-device
training story. The free product de-risks the proposal's
feasibility claim.

### Funnel E — Speaking, writing, advisory

Engineer who built a free tool the press writes about → conference
keynote invites, podcast appearances, advisory shares at startups
working in adjacent spaces. Compounds over years.

**Trigger:** any organic press mention.
**Action:** opportunistic accept.

### Funnel F — Education / curriculum

The codebase is a teachable artifact: workspace layout, single
binary cross-compile, Native Messaging, JNI, on-device ML. Bootcamp
or university curricula license *teaching* materials around it
(materials, not code — code is Unlicense).

**Trigger:** any bootcamp or program inquiry.
**Action:** flat-fee curriculum-development engagement.

---

## 6. Risk plan

What kills this product, and what we do about it.

| Risk | Mitigation |
|---|---|
| Chrome Web Store rejection (over `nativeMessaging` or `<all_urls>`) | Publish from `cochranblock` org with a clean privacy policy. Manifest requests minimum permissions. Resubmit with rationale if rejected. Backup: GitHub Release as a side-loadable extension. |
| ATS vendors break the DOM intentionally to block autofill | The extension uses CDP-equivalent post-hydration snapshotting + MutationObserver. Vendor cat-and-mouse is a long game we win because we can ship fast (single binary, no infra). |
| Simplify.us forks our code under their name | The Unlicense allows it. They lose the "I am Simplify, why is my code MIT-flavored Unlicense'd?" trust battle. We win the audit. |
| Big Tech ships a built-in form-filler that obsoletes us | Chrome already has Autofill; we're already differentiated by training-on-the-user's-resume + free-text generation. iCloud Keychain etc. don't address ATS-specific fields. |
| The author burns out on a free product with no revenue | The "interested parties" funnels are the answer. The author is paid via Funnel A/C/E, not the product. |
| Critical CVE in `chromiumoxide` / `tokio` / `wasm-bindgen` | Standard `cargo audit` in CI. Pin versions. Patch within 24h. |
| Apple / Google Play rejects the mobile app | Apple is risky (we're not making an iOS app for now). Google Play accepts WebView-based apps that don't claim privileges they don't use; our manifest requests only `INTERNET`. Should be fine. |
| Adversarial training data poisoning via opt-in feedback | Every accepted contribution is a public PR. Maintainer reviews. Reproducible build pins what's in the binary at any commit. |

---

## 7. 90-day calendar

| Week | Engineering | Distribution / narrative |
|---|---|---|
| 1 | Phase 1 ship: real `init`, persisted queue, install-bridge, demo video. | (silent) |
| 2 | Phase 2 starts: corpus conversion, classifier training. | Technical Show HN #1 (the bridge). |
| 3 | Phase 2 ships: classifier benchmark published. | (silent) |
| 4 | Phase 3 starts: GitHub Actions cross-compile, sign, release. | Technical Show HN #2 (the classifier). |
| 5 | Phase 3 ships: v0.1.0 GitHub Release with all four binaries. Web Store submitted. | **LAUNCH WEEK**: Show HN, Product Hunt, Reddit, Twitter, LinkedIn — coordinated. |
| 6 | Bug-fix cadence from launch feedback. | Privacy-press pitches go out. Demo video pinned. |
| 7 | Phase 4 starts: opt-in sync, user feedback compounding. | Counselor-newsletter outreach. Track install metrics. |
| 8 | Phase 4 ships. | Conference CFPs submitted (RustConf, OSS Summit, BSides). |
| 9 | Phase 5 starts: roadmap, public benchmarks, vendor matrix. | "I built atsisbroken because…" long-form blog post. |
| 10 | Vendor fixtures: Workday + Greenhouse with full test coverage. | Engage first-wave contributors on good-first-issues. |
| 11 | Vendor fixtures: Lever + iCIMS. | (silent) |
| 12 | Stable v0.2.0 Release. Android Play internal track → public. | 90-day retrospective post: numbers, surprises, what's next. |

---

## 8. The honest yardstick

How we know the plan worked, by day 90:

- ≥ 5,000 Chrome Web Store installs.
- ≥ 1,000 GitHub stars.
- ≥ 1 piece of unprompted press coverage in a tech outlet
  (HN front page counts; better still, a Wired/Markup/Verge piece).
- ≥ 3 inbound conversations from the funnels in §5.
- ≥ 10 community-contributed PRs landed (vendor fixtures, translations,
  bug fixes).
- Accuracy ≥ 90% on a held-out slice of the Chromium corpus for
  the top 10 most-common Profile keys.

Less than half of those = we missed the window; rethink distribution.
All of them = we proved the thesis: free can fund itself, ironically.

---

## 9. What this plan deliberately does NOT do

- It does **not** add a paid tier. The thesis dies the day it does.
- It does **not** add telemetry, even "anonymous." The privacy claim
  must be auditable in a single `grep`.
- It does **not** chase enterprise revenue at the cost of the consumer
  product. Funnel C runs adjacent, not on top of.
- It does **not** go closed-source for a "premium" version. The
  Unlicense is a one-way commitment.
- It does **not** target any vendor for legal action over DOM
  changes. We win on shipping speed, not on lawyers.

---

## Updates

This file is the source of truth. When the plan changes, update this
file in the same PR as the change. Don't keep a parallel "current
plan" in a Notion doc.
