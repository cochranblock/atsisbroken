# atsisbroken — Timeline of Invention

Author: Michael Cochran (GotEmCoach). Org: The Cochran Block, LLC.
License: Unlicense.

## 2026-05-01 — Conception

`atsisbroken` repository created on `kova-thick-beast` (n2/bt). Original
framing: Chrome extension + WASM, inference via Chrome built-in AI.

## 2026-05-03 — Architectural pivots

**Pivot 1.** Built-in-AI rejected. Adopt Cochran Block stack:
`pixel-forge` training → custom `.safetensors` → `kova-engine` inference →
`exopack` TRIPLE SIMS. Path A/B/C analyzed. Path B (extension + WASM
core) selected. Diamond profiles wired. Commit `e09cf05`.

**Pivot 2.** WASM dropped. Adopt single-binary CDP architecture
(chromiumoxide). Workspace collapsed to single package. Bake variants
(tiny / cinder / cinder-f16 / quench) wired via `include_bytes!`.
Provenance docs added. Commit `f495a88`.

**Pivot 3.** Baked third-party weights dropped. Architecture: ship a
seed corpus + on-device trainer; on first run, atsisbroken trains a
personal classifier from the user's resume + seed corpus, saved at
`~/.atsisbroken/`. The user's model is the only model. Three autonomy
modes: `TrainingWheels` (yes/no per fill, online learning), `Shadow`
(observe user filling manually, auto-fill once per-key confidence
crosses `ConfidenceThreshold`, default 0.85), and `Chaos` (fully
autonomous, post-hoc flagging still trains).

**Bridge.** Chrome extension + desktop binary connected via Native
Messaging (4-byte LE length-prefixed UTF-8 JSON, max 1 MiB per
Chrome's spec). Extension records `(FieldDescriptor → predicted_key)`
events into `chrome.storage.local` (never the user's typed value);
background service worker drains the queue every 5 minutes through
`chrome.runtime.connectNative` to the desktop host's `bridge`
subcommand. Commit `0386909`.

**Doc structure.** Aligned with Cochran Block convention:
`PROOF_OF_ARTIFACTS.md`, `TIMELINE_OF_INVENTION.md`,
`USER_STORY_ANALYSIS.md`, `BACKLOG.md`, `CONTRIBUTORS.md`, `PLAN.md`
all at the repo root. Topic docs in `docs/`. Commits `aacb982`,
`b0c7120`.

## 2026-05-03 — PLAN Phase 1 ship

**#1 Persistent feedback queue.** `FeedbackQueue::{load_from, save_to}`
with atomic `.tmp + rename`. Bridge subcommand reads on entry, writes
on exit (even on protocol error mid-session). Commit `9ebd1ad`.

**#2 `install-bridge` subcommand.** Auto-writes the Chrome Native
Messaging host manifest to the per-OS path (macOS:
`~/Library/Application Support/Google/Chrome/NativeMessagingHosts/`;
Linux: `~/.config/google-chrome/NativeMessagingHosts/`). Commit
`9ebd1ad`.

**#3 `init` resume parser.** Hand-rolled, regex-free heuristic. Pulls
email (with dotted-domain check), phone (10–15 digit run), LinkedIn /
GitHub / generic website URLs, plausible name detection. End-to-end
verified on the host. Commit `9ebd1ad`.

**#4 Strategy ladder.** `atsisbroken run` auto-detects the environment
and picks the highest-tier fill strategy that works:
`CdpAttach` → `CdpLaunch` → `Extension` → `Userscript` →
`Bookmarklet` → `Clipboard` → `Speak`. The floor (`Speak`) prints
`key: value` to stdout and works literally anywhere atsisbroken runs.
Per-strategy subcommands also dispatchable directly: `userscript`,
`bookmarklet`, `copy <key>`, `speak`, `cdp-probe`. CDP attach itself
is proof-of-life only at this commit (lists tabs; the destructive fill
loop is the next iteration). Commit `aa93f81`.

**Coverage push.** Test suite grew from 77 to 156. JSON-shape pinning
on every wire-format struct, behavioral diversity on the resume
parser (13 distinct inputs), userscript syntactic checks, integration
tests against the actual built binary in `tests/cli_smoke.rs`.
Exopack TRIPLE SIMS gate: 156/156 × 3 byte-identical. Commit `aa93f81`.

**TUI as default surface.** Three-tab terminal app (dashboard / queue
/ strategy) with Claude-Code aesthetic — minimal borders, dim status
footer with keybind hints, vim/arrow/digit navigation. Real-pixel
TUI screenshots via ratatui's `TestBackend` → styled HTML →
headless Chromium. Commit `6eed497`.

**Browser automation Phase A.** `browser_detect` module: per-OS
default-browser detection (Linux `xdg-settings`, macOS LaunchServices
plist parser, Windows PATH scan). `BrowserKind` enum with
`.supports_cdp()` — Firefox is the lone false, falls through to
userscript automatically. Commit `72ac7a9`.

**Multi-profile support.** `--profile <path>` global flag threaded
through every subcommand that touches a profile. Closes the P4
career-counselor finding from `UI_UX_SIMULATION.md`. Commit `72ac7a9`.

**`Profile::is_meaningfully_populated`.** Distinguishes "init ran but
parser found nothing" from "real profile" — `status` now reports
three states (`no` / `exists but empty` / `yes`). Closes the E6
finding. Commit `72ac7a9`.

## 2026-05-05 — Real production-source attribution + research

**ATS fixture sources cited.** Live extension code from Jeffrey Hui
(`workpls`), Joseph Ajibodu, and Nathaniel Ubangura's Workday
automator surface real production selectors (`job_application[*]`
brackets, single Lever `name`, Workday `data-automation-id`,
`urls[Github ]` trailing-space quirk). 5-vendor fixture generator
in `kova::exopack::ats_fixtures`, gated by feature `ats_fixtures`.
Atsisbroken consumes via re-export. Static `.html` fixtures
deleted. Commit `0cebacf`.

**PERFECTION_PLAN.md** — research-driven contract for 1.0. Eleven
R-phases (R1 market census → R11 beta cohort) with measurable
targets per phase. No shipping until R3 closes ≥98% accuracy on
canonical fields. Commit `da59c76`.

**R1 (HN cohort).** 683 URLs scraped from "Ask HN: Who is hiring?
(May 2026)" thread (item 47975571). Distribution: Ashby 49%,
Greenhouse 30%, Lever 5%, Workday 0.9% (HN is startup-biased).
`docs/research/ATS_MARKET_SHARE_2026Q2.md`. Commit `67e7cd3`.

**R1-broader.** F500 / overall-market reconciliation citing
AppsRunTheWorld (iCIMS 10.7% mkt leader), SHRM 2025 (Workday 39%
of F500), Ongig 2019 (historical baseline), Workday Wikipedia,
Maximize Market Research. Two cohorts, two priorities — Ashby
priority 1 for HN/startup applicants; Workday priority 2 for F500.
`docs/research/ATS_MARKET_SHARE_F500_2026.md`. Commit `a008ffc`.

**R2 — Ashby first capture.** Live capture of
`jobs.ashbyhq.com/lago/.../application` (2026-05-05). Stable
`ashby-application-form-*` class names verified; React-hashed
prefixes ignored as drift. Real Ashby uses single `Name` field
(like Lever, NOT split first/last). Kova fixture `render_ashby`
rewritten with verified selectors. Confidence promoted: C → B (3
tenants for A; 2 retries deferred). Commits `fbc6631` (kova local)
+ `a008ffc` (atsisbroken).

**KEYSTONE: real CDP fill loop.** `src/run_loop.rs::run_against_url`
launches chromium, snapshots, classifies, fills, screenshots,
persists feedback queue. `cmd_run --url <ATS-URL>` takes this path.
Constraint C1 enforced — never auto-submits; explicit warning to
user. Smoke-tested end-to-end on the dev box: 6/6 fields filled on
a Greenhouse-shaped form, screenshot saved, feedback persisted.
Commit `695a1d8`. **Product can now actually fill an ATS form for
a real user.**

**Mode persistence (Config struct + working `graduate`).** New
`src/config.rs` with atomic save (`.toml.tmp + rename`),
forward-compat field defaults (older configs load even when new
fields are added). `cmd_graduate(yes, back)` replaces the
`eprintln!("not yet wired")` stub:
- TrainingWheels → Shadow → Chaos (forward, default)
- `--back`: Chaos → Shadow → TrainingWheels
- Refuses at either end with actionable error
  (`AlreadyAtTop` hints `--back`, `AlreadyAtBottom` notes "already
  the most supervised")
- `--yes` skips the y/N prompt for scripting
- `cmd_status` now reads persisted mode + surfaces
  "(default — never run `atsisbroken graduate`)" vs
  "(persisted in ~/.atsisbroken/config.toml)"
8 unit tests on Config. Commit `45fa177`. **Mode is no longer a
stub — it persists across runs.**

**Mode-aware run loop (TrainingWheels prompts + Shadow gating).**
New `decide(mode, key, confidence, threshold, has_value, has_dom_id)
-> FillDecision::{Skip{reason}, Fill, Prompt}` — pure, testable,
mode-policy-encoding fn. `SkipReason` enum surfaces *why*
(NotClassified / NoProfileValue / NoDomId / BelowConfidenceThreshold).
Skip-precedence honored: NotClassified > NoProfileValue > NoDomId >
mode-specific.
- **TrainingWheels:** stderr/stdin prompt per field
  `field "Email" → email = "jane@..." — fill? [y/N]`. Yes → fill +
  `accepted: true` Feedback. No → skip + `accepted: false` Feedback
  with `actual=""` so a future trainer learns from rejections.
- **Shadow:** `confidence >= ConfidenceThreshold` to fill. Today
  the keyword classifier returns 1.0 for matches and 0.0 for
  unknowns, so any threshold ≤ 1.0 passes everything matched. The
  gating is structurally correct — when R4 lands a real-confidence
  trained classifier, the threshold actually filters. Not over-claimed.
- **Chaos:** fill everything classified.
`predict_field_key_with_confidence(f) -> (&str, f32)` added.
Signature stays stable for R4 swap.
End-to-end smoke verified all three modes on the dev box. 10 unit
tests on `decide` covering all skip paths, mode dispatch,
inclusive threshold (`>=`), precedence ordering. Commit `f413829`.
**Mode flag is no longer a stub — it changes behavior.**

## Forward plan

- Real CDP fill loop (chromiumoxide WebSocket, DOM snapshot,
  classify-and-fill cycle, Workday re-render verifier).
- Train pipeline (`--features train`): logistic regression over
  hand-engineered FieldDescriptor features, online SGD updater.
- Convert Chromium's `chrome/test/data/autofill/heuristics` corpus to
  our JSONL training format (~1.5–3k high-quality pairs).
- `atsisbroken sync-config --destination` to opt into
  `FeedbackDelivery::SendWhenOnline`.
- Cross-compile pipeline: GitHub Actions + `--profile=diamond-edge`.
- `master` → `main` for default-branch parity.
<!-- COCHRANBLOCK-BRAND-FOOTER:START - generated by cochranblock/scripts/brand-stamp.sh -->

---

<sub>&#9656; **THE COCHRAN BLOCK, LLC** &#183; CAGE `1CQ66` &#183; UEI `W7X3HAQL9CF9` &#183; UNLICENSE &#183; [cochranblock.org](https://cochranblock.org)</sub>
<!-- COCHRANBLOCK-BRAND-FOOTER:END -->
