# atsisbroken — Backlog

Live working list. Items get moved to `## Done` (with the commit hash)
as they ship. Order under each section reflects priority.

## Now — Browser Automation cluster (see `docs/PLAN_BROWSER_AUTOMATION.md`)

- [ ] **A**   `browser_detect::default_browser()` — xdg-settings /
      LSCopyDefault / HKCU UserChoice per OS.
- [ ] **A.5** `browser_install` — portable Chromium download (zero-
      admin, SHA-256 verified) + package-manager fallback (with
      explicit `[y/N]` confirmation, no auto-sudo).
- [ ] **B**   `browser_launcher::launch()` — Chromium subprocess
      with `--remote-debugging-port=<random>` + temp user-data-dir;
      `Drop` SIGTERMs.
- [ ] **C**   `cdp::Session` — WebSocket attach via chromiumoxide,
      `wait_for_idle`, `snapshot_form`.
- [ ] **D**   `cdp::Session::fill` + `verify` — write value, dispatch
      input/change events, re-snapshot 250ms later for Workday
      re-render defense.
- [ ] **E**   TUI Browse tab (4th tab) — URL input + history +
      live status (idle/launching/classifying/filling/done).
- [ ] **F**   `atsisbroken go <url>` subcommand — non-TUI entry to
      the same flow; pipe-friendly with `--rate-limit`.

## Now — other

- [ ] `atsisbroken graduate` — confirmation prompt + flip mode in
      `~/.atsisbroken/config.toml`.
- [ ] `atsisbroken sync-config --destination ...` — toggle
      `FeedbackDelivery::SendWhenOnline`.
- [ ] `atsisbroken status` — show *current* mode (not just default)
      once config.toml is wired.
- [ ] `atsisbroken --profile <path>.toml` — multi-profile flag for
      P4 (career counselors). Surfaced by `UI_UX_SIMULATION.md`.
- [ ] `Profile::is_meaningfully_populated()` — distinguish "no
      profile" from "profile exists but empty" (E6 finding).

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
