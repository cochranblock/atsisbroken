# atsisbroken — Backlog

Live working list. Items get moved to `## Done` (with the commit hash)
as they ship. Order under each section reflects priority.

## Now — Research debt (see `docs/PERFECTION_PLAN.md`)

R-phases gate everything else. No ship until these close.

- [x] **R1 (HN cohort)**  ATS market census — 683 URLs from 300
      comments in HN's "Who is hiring? (May 2026)" thread,
      classified by host. Result: Ashby 49% / Greenhouse 30% /
      Lever 5% / Workable 4% / others; Workday only 0.9% (HN
      is startup-biased). Full report:
      `docs/research/ATS_MARKET_SHARE_2026Q2.md`.
      **Decision triggered:** promote Ashby fixture from C → A
      BEFORE the Workday wizard (R5).
- [ ] **R1-broader**  Re-run R1 against Indeed top-search /
      LinkedIn jobs / USAJOBS to get F500 distribution. HN
      sample alone misses Workday/iCIMS/Taleo/SuccessFactors.
- [ ] **R2**  Real DOM capture — `Page.captureSnapshot` MHTMLs of
      3-5 public postings per top vendor. Hand-label each field
      with expected classifier key. Replaces inferred fixtures.
- [ ] **R3**  Classifier accuracy benchmark — run `predict_field_key`
      against the R2 capture set. Measure per-vendor %. If <98%,
      triggers R4.
- [ ] **R4**  Train logistic-regression classifier — gated on R3
      result. Chromium autofill heuristics corpus + R2 captures
      + USAJOBS templates. Online SGD updater.
- [ ] **R5**  Multi-page Workday wizard — render
      myExperiencePage / voluntaryDisclosuresPage /
      selfIdentificationPage; e2e advances through with the
      `bottom-navigation-next-button`.
- [ ] **R6**  Anti-bot fingerprint research — what each vendor
      flags + what we mitigate (no CAPTCHA solving, no human-fake).
- [ ] **R7**  Demographics field safety audit — regression test
      that classifier returns "unknown" on every EEO/AAP field
      across all R2 captures. CRITICAL for trust.
- [ ] **R8**  Selector decay monitoring — Wayback Machine baselines
      + monthly GitHub Action that pages on drift.
- [ ] **R9**  Legal / ToS review per vendor — written analysis
      reviewed by counsel before public 1.0.
- [ ] **R10** Hiring manager interviews — 5-10 real recruiters,
      30-min structured. Replaces inferred quotes in
      `HIRING_MANAGER_ANALYSIS.md`.
- [ ] **R11** Beta cohort — opt-in 20-50 users; instrumented
      install funnel + per-vendor success rate + corrections.

After R3 + R5 + R7 close: 1.0 candidate.

## Now — Browser Automation cluster (see `docs/PLAN_BROWSER_AUTOMATION.md`)

- [x] **A**   `browser_detect::default_browser()` — xdg-settings (Linux),
      `defaults read` LaunchServices plist (macOS), PATH scan
      (Windows; full HKCU UserChoice deferred to phase E.5). 17 tests
      covering xdg-desktop kind inference, plist HTTP-handler parsing,
      bundle-id mapping, Windows ProgId mapping, CDP-support gate.
      Smoke-tested on the dev box: `default browser: Chromium
      (/usr/bin/chromium)` shows in `status`.
- [ ] **A.5** `browser_install` — portable Chromium download (zero-
      admin, SHA-256 verified) + package-manager fallback (with
      explicit `[y/N]` confirmation, no auto-sudo).
- [ ] **B**   `browser_launcher::launch()` — Chromium subprocess
      with `--remote-debugging-port=<random>` + temp user-data-dir;
      `Drop` SIGTERMs.
- [ ] **C**   `cdp::Session` — WebSocket attach via chromiumoxide,
      `wait_for_idle`, `snapshot_form`.
- [x] **mock ATS fixtures + e2e**: 3 hand-crafted fixture HTMLs
      (greenhouse, lever, workday). `tests/ats_e2e.rs` launches real
      chromium, navigates, snapshots fields via `page.evaluate`,
      classifies each, asserts predictions against `expected.toml`,
      fills via CDP, saves screenshots to `docs/screenshots/ats/`.
      **Live test caught a real classifier bug** (substring "tel"
      inside "websiteLinkedIn" routing to phone instead of linkedin)
      that no unit test would have caught. Fixed; regression-pinned.
- [x] Split full_name into first_name / last_name. Profile schema
      gets first_name + last_name fields (alongside full_name for
      single-name vendors). Resume parser splits on whitespace
      (last token = last_name; rest = first_name; one-word names
      stay full_name only). Classifier disambiguates via phrase-
      based matching ("first name" / "first_name" / "firstName" /
      "first-name" all normalize via camelCase tokenization);
      bare id="first" no longer false-positives. profile_value_
      for_key falls back to full_name when first_name/last_name
      are empty (forward-compat for older profiles).
      Per-vendor expected_keys in kova fixture: Greenhouse/Workday
      /iCIMS split first/last; Lever/Ashby use combined "Name" →
      full_name.
      Visible win: Greenhouse-shape e2e screenshot now shows
      First name="Jane Q." and Last name="Doe" — different
      strings.
- [x] Split address into structured sub-fields. Profile gains
      street1, street2, city, state, postal_code, country
      (all #[serde(default)] for forward-compat). Classifier
      disambiguates: postal_code (zip/postcode/postal) → city →
      state (province/region) → country (nation) → street2
      (address line 2 / apt / suite) → street1 (street, address
      line 1) → bare "address" stays legacy single-line. Order
      reshuffled so work_authorization wins over country before
      address sub-fields fire (Workday's "authorized to work in
      this country?" was misclassifying as country).
      profile_value_for_key: each structured field falls back to
      the legacy `address` string when empty (forward-compat for
      older profiles). Live e2e fills 6/6 structured sub-fields:
      Street=123 Main St, Apt=Apt 5, City=Anywhere, State=CA,
      Zip=94000, Country=USA — all distinct.
- [ ] **D**   `cdp::Session::fill` + `verify` — write value, dispatch
      input/change events, re-snapshot 250ms later for Workday
      re-render defense.
- [ ] **E**   TUI Browse tab (4th tab) — URL input + history +
      live status (idle/launching/classifying/filling/done).
- [ ] **F**   `atsisbroken go <url>` subcommand — non-TUI entry to
      the same flow; pipe-friendly with `--rate-limit`.

## Now — Profile + GitHub cluster (see `docs/PLAN_PROFILE_AND_GITHUB.md`)

- [ ] **G**   Expanded `Profile` (~75 fields) + v0→v1 migration
- [ ] **H**   `init` walks every field group, with parser-derived defaults
- [ ] **I**   `connect-github` + `sync-github` → `~/.atsisbroken/github_inventory.json`
- [ ] **J**   Question classifier: 12 prompt patterns → answer slots
- [ ] **K**   Answer composer: verbatim-source free-form answers + audit log
- [ ] **L.5** Custom patterns at `~/.atsisbroken/custom_patterns.toml`:
      user-defined question routes + extractors + answer slots, all
      regex-safe (size_limit, compile/exec timeouts, verbatim-only output)
- [ ] **L**   TUI tabs 4 (Profile) + 5 (GitHub)

## Now — other

- [x] `atsisbroken graduate` — confirmation prompt + flip mode in
      `~/.atsisbroken/config.toml`. `--yes` skips prompt; `--back`
      steps backward (Chaos → Shadow → TrainingWheels). Refuses at
      either end with actionable error. Smoke-verified end-to-end.
- [x] **Gap #7 — feedback consumer (`learning::CorrectionOverlay`)**.
      The accumulated feedback.jsonl now actually changes future
      runs. Field fingerprint = (label, placeholder, aria_label,
      name, kind), DOM id deliberately excluded so dynamic Workday
      ids don't bust the cache. Most-recent decision wins. Run
      loop short-circuits BEFORE mode policy: prior `Rejected` →
      Skip silently in any mode; prior `Accepted` → Fill in any
      mode (overrides TrainingWheels prompt and Shadow threshold).
      New `atsisbroken feedback` subcommand summarizes total /
      accepted / rejected / overlay size / top rejected keys.
      Live smoke verified: 1st run prompts y/n; 2nd run takes the
      same form and skips both prompts (overlay carries decisions).

- [ ] `atsisbroken sync-config --destination ...` — toggle
      `FeedbackDelivery::SendWhenOnline`.
- [ ] `atsisbroken status` — show *current* mode (not just default)
      once config.toml is wired.
- [x] `atsisbroken --profile <path>.toml` — multi-profile flag for
      P4 (career counselors). Threaded through init / run / status /
      speak / userscript / bookmarklet / copy. Surfaced by
      `UI_UX_SIMULATION.md`.
- [x] `Profile::is_meaningfully_populated()` — `status` now reports
      "exists but empty" vs "yes" vs "no". E6 finding closed.

## Next

- [ ] Real classifier — logistic regression over hand-engineered
      features from FieldDescriptor. Trains in seconds on CPU.
- [ ] Online updater — apply each `Feedback` / `Observation` event to
      the model with SGD.
- [ ] Confidence threshold gate in `Shadow` mode — wire
      `ConfidenceThreshold` into the run-loop.
- [ ] `atsisbroken bake` (with `--features train`) — train a personal
      classifier from the user's accumulated `feedback.jsonl`.

## Soon

- [ ] Convert Chromium autofill heuristics test corpus to our JSONL
      format. ~1,500–3,000 high-quality pairs from
      `chrome/test/data/autofill/heuristics`.
- [ ] In-page CDP overlay for `TrainingWheels` yes/no prompts (instead
      of terminal prompts).
- [ ] GitHub Actions: cross-compile `--profile=diamond-edge` for the
      four target triples, attach to a Release.
- [ ] `master` → `main` for default-branch parity with the rest of
      cochranblock.

## Android

- [ ] Wire `cargo-ndk` cross-compile for arm64-v8a / armeabi-v7a /
      x86_64.
- [ ] WebView JavaScriptInterface for DOM extraction + autofill
      (mobile equivalent of `extension/content.js`).
- [ ] Play Store listing: privacy policy, screenshots, store description.

## Later

- [ ] Web Store submission for the Chrome extension.
- [ ] Native Messaging install path automated for Brave / Edge / Arc /
      Chromium variants.

## Done

- [x] Pivot to single-binary CDP architecture (`e09cf05` superseded by `8758903`).
- [x] On-demand user-trained model (`8758903`).
- [x] Offline feedback queue + opt-in sync (`a73b958`).
- [x] Auto-screenshots + UX sim + Android scaffold (`26e82e5`).
- [x] Real test coverage + training-data sourcing posture (`e840cb8`).
- [x] Training data: real sources verified (`b92d59b`).
- [x] `Mode::Shadow` + Chrome extension ↔ desktop bridge (`0386909`).
- [x] `docs/USER_FLOW.md` end-to-end walkthrough (`98bc907`).
- [x] Doc structure aligned with Cochran Block convention (`aacb982`).
- [x] PLAN.md — engineering + distribution + interested-parties (`b0c7120`).
- [x] PLAN Phase 1 #1: persist FeedbackQueue to ~/.atsisbroken/feedback.jsonl (this commit).
- [x] PLAN Phase 1 #2: `install-bridge` writes per-OS Native Messaging manifest (this commit).
- [x] PLAN Phase 1 #3: `init` parses resume text → Profile, writes profile.toml (`9ebd1ad`).
- [x] `status` first-run detection + feedback queue depth (`9ebd1ad`).
- [x] PLAN Phase 1 #4: strategy ladder + CDP attach proof-of-life
      (CDP-attach → CDP-launch → extension → userscript → bookmarklet
      → clipboard → speak; `run` auto-picks; per-strategy subcommands).
- [x] TUI as default interface (Claude Code aesthetic, 3 tabs:
      dashboard/queue/strategy). Behind `tui` feature, default-on. TUI
      screenshots auto-captured via ratatui TestBackend → HTML →
      headless Chromium. (this commit)
- [x] Coverage push to 156 tests (was 77): JSON-shape pinning for every
      bridge variant + Experience + Education; resume parser fed 13
      diverse inputs; userscript brace/paren/metadata + don't-clobber
      checks; bookmarklet single-line / no-unencoded-quotes-or-hash;
      parse_mode + parse_strategy_override branch coverage; 9-test
      integration suite running the actual binary.
