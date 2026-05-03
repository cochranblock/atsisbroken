<!-- Unlicense — cochranblock.org -->
<!-- Contributors: GotEmCoach, KOVA, Claude Opus 4.7 -->

# atsisbroken — Browser Automation Plan

**Date:** 2026-05-03
**Scope:** TUI-driven flow that detects the user's default browser,
launches it with a debug port, navigates to a URL, classifies +
autofills the form, then hands control back to the user to submit.
**Predecessor:** `PLAN.md` Phase 1 (DONE) and Phase 2 (the classifier
+ CDP fill loop). This doc lives between them — Phase 1.5.

---

## 1. Research findings

### 1.1 Default-browser detection per OS

| OS      | Mechanism | Reliability | Citation |
|---------|-----------|-------------|----------|
| Linux   | `xdg-settings get default-web-browser` returns a `.desktop` filename; resolve to its `Exec=` line | High when xdg-utils installed; degrades to `BROWSER` env var | [xdg-settings docs](https://portland.freedesktop.org/doc/xdg-settings.html), [slatecave write-up](https://slatecave.net/blog/xdg-settings/) |
| macOS   | `LSCopyDefaultHandlerForURLScheme` via the LaunchServices framework, or shell out to `defaults read com.apple.LaunchServices/com.apple.launchservices.secure` | High when `objc` / `core-foundation` linked | macOS LaunchServices docs |
| Windows | `HKCU\SOFTWARE\Microsoft\Windows\Shell\Associations\UrlAssociations\http\UserChoice\ProgId`, then map ProgId to `HKLM\SOFTWARE\Classes\<ProgId>\shell\open\command` | High; the registry is the source of truth | x-default-browser ([github.com/jakub-g/x-default-browser](https://github.com/jakub-g/x-default-browser)) |

**No mature pure-Rust crate** does all three. We hand-roll a thin
`browser_detect` module: each branch ~50 LOC, total ~150.

### 1.2 CDP attach constraints

- **Chrome 137+ requires `127.0.0.1`-only CDP.** External IPs are
  refused for security. Same-host attach is fine.
  ([chromedevtools.github.io/devtools-protocol](https://chromedevtools.github.io/devtools-protocol/),
   [BrowserStack](https://www.browserstack.com/guide/playwright-connect-to-existing-browser))
- **You cannot attach to an already-running Chrome that wasn't
  launched with `--remote-debugging-port`.** This is by design.
  Implication: atsisbroken must either launch its own browser
  subprocess, or instruct the user to relaunch their default browser
  with the flag.
- **Chrome 144+ has zero-config CDP** via `chrome://inspect/#remote-debugging`,
  but that's still a manual user step.
- **`webSocketDebuggerUrl`** from `/json/version` is the connection
  endpoint; chromiumoxide handles the WebSocket framing for us.

### 1.3 ATS automation policy landscape

- **No platform publishes an explicit ban list** on form-filler tools.
  ([Sprout AI Job Bots Guide](https://www.usesprout.com/blog/ai-job-application-bots-complete-guide))
- **Greenhouse does not bot-score auto-reject;** rejection is always
  a human action.
- **79% of orgs use AI in their ATS;** 64% deploy auto-reject filters,
  but those filters key off content (years of experience, location,
  degree), not the *means* of submission.
- **Existing commercial competitors** (Simplify Copilot, AutoApplier,
  ResumeUp.ai) operate without facing platform-level bans. Confirms
  the activity is acceptable when the user reviews each submission.
- **Risk vector** is mass-apply *without* user review — bot-style
  applies in volumes that look automated to recruiters. atsisbroken's
  TrainingWheels / Shadow modes structurally prevent that mode of use.

### 1.4 Implication summary

The architecture must:
1. **Detect** the user's default browser, but
2. **Launch** a fresh subprocess (cannot attach to existing instance).
3. **Hand the *visible* window back to the user** so they review and
   click Submit themselves — both for ethics and to dodge anti-bot
   patterns.
4. **Confine** the autofill to fields the model is confident on
   (Shadow mode threshold) — never hallucinate values for human
   review questions.

---

## 2. Hard constraints

| # | Constraint | Source |
|---|------------|--------|
| C1 | atsisbroken must not auto-submit forms; the user clicks Submit | Ethics + anti-bot policy |
| C2 | atsisbroken must not bypass robots.txt or platform ToS | Cochran Block, LLC house rules |
| C3 | The launched browser shows a visible window (not headless) | User must see what's being filled |
| C4 | Profile data never leaves the device unless user opts in via `FeedbackDelivery::SendWhenOnline` | The product's thesis |
| C5 | Per-field confidence must clear `ConfidenceThreshold` before auto-fill in Shadow mode | Existing API contract |
| C6 | If the default browser is Firefox, fall through (Firefox lacks CDP) | Firefox uses Marionette, not CDP |
| C7 | Each tier of the strategy ladder remains a fall-through option | UX continuity with shipped behavior |
| C8 | Never `sudo` automatically; package-manager installs require explicit user `[y/N]` per command | No silent privilege escalation |
| C9 | Portable browser download must verify SHA-256 against the official manifest before exec | Defense against MITM on first-install |

---

## 3. Canonical user flow

```
[TUI dashboard]
       │
       │ (user presses 'b' for Browse, or types URL in a new "go" tab)
       ▼
[TUI prompts: "URL?" + recent URLs from a local history file]
       │
       │ (user enters: https://boards.greenhouse.io/example/jobs/1234)
       ▼
[atsisbroken detects default browser]
       │
       │  Linux:   xdg-settings get default-web-browser → /usr/bin/firefox
       │  macOS:   LSCopyDefaultHandlerForURLScheme → com.google.Chrome
       │  Windows: HKCU UserChoice → Chrome
       │
       │  if default is Chromium-family → use it
       │  if default is Firefox        → fall through to atsisbroken-default Chromium
       │
       ▼
[Launch browser subprocess with --remote-debugging-port=<random>
                              --user-data-dir=<temp>
                              --no-first-run
                              <URL>]
       │
       ▼
[atsisbroken connects to localhost:<port>/json/version, gets WebSocket URL]
       │
       ▼
[Wait for Page.lifecycleEvent { name: "networkIdle" }]
       │
       ▼
[Runtime.evaluate: walk every <input>, <textarea>, <select>; emit
                    FieldDescriptor[] as JSON]
       │
       ▼
[Per field, classify via local model. Mode-dependent action:]
  TrainingWheels  → TUI prompts user yes/no per field
  Shadow          → fields above ConfidenceThreshold auto-fill silently;
                     user fills the rest
  Chaos           → all classified fields auto-fill
       │
       ▼
[atsisbroken: "5 fields filled. Tab back to your browser to review.
              Press Enter when you've submitted, q to bail."]
       │
       ▼
[User clicks Submit in the visible browser. Returns to TUI.]
       │
       ▼
[Feedback events appended to ~/.atsisbroken/feedback.jsonl]
       │
       ▼
[Browser subprocess optionally kept open for next URL, or closed]
```

---

## 4. Architecture additions

| Module                     | Purpose                                                | Status |
|----------------------------|--------------------------------------------------------|--------|
| `src/browser_detect.rs`    | Default-browser detection per OS                       | NEW |
| `src/browser_launcher.rs`  | Launch Chromium subprocess with debug port             | NEW |
| `src/browser_install.rs`   | Portable download + package-manager fallback           | NEW |
| `src/cdp.rs` (extend)      | Attach via WebSocket, navigate, snapshot, fill         | EXTEND |
| `src/tui.rs::Browse`       | New TUI tab: URL input + recent history                | EXTEND |
| `src/history.rs`           | URL history at `~/.atsisbroken/history.toml`           | NEW |
| `src/main.rs::cmd_go`      | `atsisbroken go <url>` — non-TUI entry to the same flow| NEW |

The new `cmd_go` subcommand is the scriptable form: pipe job URLs
from a script to atsisbroken. Same code path as the TUI's Browse tab.

---

## 5. Phased implementation

Each phase ships green: gate stays at 100% pass, new tests added per
change. No phase reaches over the next.

### Phase A — Browser detection (DONE — commit `72ac7a9`)

`browser_detect::default_browser()` returns `Option<DefaultBrowser>`
with `{ kind: BrowserKind, path: Option<PathBuf> }`. `BrowserKind`
covers Chrome / Chromium / Firefox / Edge / Brave / Arc / Vivaldi /
Opera / Other(String) and exposes `.supports_cdp()` (Firefox = false).

Implementation:
- Linux: `xdg-settings get default-web-browser` → desktop name →
  `parse_xdg_desktop_name` → kind; `which` per kind for the path.
- macOS: `defaults read com.apple.LaunchServices/com.apple.launchservices.secure`
  → `parse_macos_launchservices_http_handler` walks the plist
  blocks looking for `LSHandlerURLScheme = http`, captures the
  matching `LSHandlerRoleAll` bundle id; `bundle_id_to_kind` maps
  it; `path_for_macos_kind` resolves to the `.app` binary.
- Windows: PATH-scan stub for now (full HKCU UserChoice registry
  read deferred to Phase A.5 alongside install support).

17 tests cover every parser branch; smoke test on the dev box prints
`default browser: Chromium (/usr/bin/chromium)` in `status`.

`Strategy::detect()` consults `default_browser()`: Firefox default
→ skip CDP tiers, jump to userscript. Chromium-family → existing
`CdpLaunch` path wins.

### Phase A.5 — Browser installation when missing (~2 days)

For environments where no Chromium-family browser is detected,
atsisbroken offers to install one — **never silently, never with
auto-sudo**. Three sub-paths, in order of preference:

#### A.5.1 — Portable download (zero-admin)

- Detect online via TCP probe to `1.1.1.1:53` or DNS lookup of a
  known-stable host. Skip if offline.
- Download the latest Chromium snapshot for the OS triple from
  `https://download-chromium.appspot.com` (or the Chromium
  Snapshot CDN at `commondatastorage.googleapis.com/chromium-browser-snapshots/`).
  Targets: `Linux_x64`, `Mac`, `Mac_Arm`, `Win_x64`.
- Verify SHA-256 against a checksum the atsisbroken release pipeline
  ships in the binary itself (`include_str!` of a manifest produced
  by CI). C9 enforced.
- Extract to `~/.atsisbroken/chromium/<version>/` (per-version dir
  so an upgrade doesn't clobber). ~150 MB on disk.
- Set `chrome_path` for subsequent `CdpLaunch` to this binary.
- Tests:
  - Manifest parser (mocked checksum file).
  - SHA-256 verifier rejects mismatched archives.
  - Online-probe returns false when DNS fails.
  - Extraction returns the chromium binary path; binary is
    chmod +x on Unix.

#### A.5.2 — Native package manager (with confirmation)

- Detect package manager:
  - Linux: `apt`/`dnf`/`pacman`/`zypper`/`apk` (check `which`)
  - macOS: `brew`
  - Windows: `winget` / `choco`
- Print the exact command atsisbroken would run; prompt `[y/N]`.
  Never run without explicit yes. Constraint C8.
- Run only the install command, never anything else. The command
  is the literal string surfaced in the prompt — no expansion, no
  curl-piped scripts.
- After install, re-run `default_browser()` to find the new binary.
- Tests:
  - Package-manager detector returns the right tool per OS.
  - Confirmation gate: `[N]` exits without running anything.
  - The constructed command matches a fixture per package manager.

#### A.5.3 — Repo-aware (when atsisbroken is itself installed via
              a system repo)

- If `which atsisbroken` resolves to `/usr/bin/atsisbroken` (system
  package), that means the user has an active package manager
  configured. Use A.5.2's path with extra trust.
- If `cargo install` was the install method, prefer A.5.1
  (cargo-install users tend to want zero-admin solutions).
- Tests:
  - Detect own-install method by walking `argv[0]`'s path prefix.

#### Decision flow

```
have_chromium_family?
├── yes → use it (existing path)
└── no
    ├── online?
    │   ├── no  → tell user, fall through to userscript/etc
    │   └── yes
    │       ├── prefer portable (A.5.1) by default
    │       └── --install-via=apt|brew|winget overrides to A.5.2
    └── (handled above)
```

#### Risk: Chromium snapshot CDN goes away

- Mitigation: the manifest in the atsisbroken binary lists multiple
  download mirrors. Try each in order. If all fail, fall through
  cleanly with a "couldn't download chromium; here's the userscript
  instead" message.

### Phase B — Launcher (~1 day)

- `browser_launcher::launch(path, url, port) -> LaunchedBrowser`
  with a `Drop` impl that kills the subprocess on TUI exit.
- Args: `--remote-debugging-port=<port>`, `--user-data-dir=<temp>`,
  `--no-first-run`, `--no-default-browser-check`, `<url>`.
- Random port via `TcpListener::bind("127.0.0.1:0")` + immediate
  release (race window acceptable; CDP is local-only).
- Tests:
  - Constructed `Command` has the expected flags (introspect via
    a `Vec<String>` builder, not actually spawning).
  - Port allocator returns a non-privileged port.
  - LaunchedBrowser::Drop sends SIGTERM, then SIGKILL after 2s
    grace.

### Phase C — CDP navigate + snapshot (~2 days)

- Move from the current HTTP-only CDP probe to a proper WebSocket
  client via chromiumoxide.
- New `cdp::Session` wrapper:
  - `Session::connect(endpoint)`
  - `Session::wait_for_idle(timeout)`
  - `Session::snapshot_form() -> Vec<FieldDescriptor>`
- Tests:
  - Wire-shape pin for the JSON we send to `Runtime.evaluate`.
  - Parser test: a static fixture HTML → expected
    FieldDescriptor list.

### Phase D — Per-field fill via CDP (~2 days)

- `Session::fill(field_id, value)` → `Runtime.evaluate` writes
  `el.value = ...`, then dispatches `input` + `change` events.
- `Session::verify(field_id) -> bool` → re-query, check value
  persisted (Workday re-render defense).
- Mode-conditional flow in `cmd_go`:
  - TrainingWheels: prompt user in TUI per field (or terminal
    `[y/N]` for non-TUI invocations).
  - Shadow: only fill where the (forthcoming) classifier's
    confidence ≥ `ConfidenceThreshold`.
  - Chaos: fill all classified fields; surface a summary at end.
- Tests:
  - Fixture-based unit tests on the JS dispatch shape.
  - Integration test against a local-served HTML fixture (no real
    ATS) — runs only in CI with chromium installed.

### Phase E — TUI Browse tab (~1 day)

- New 4th tab: `browse`.
- Renders: text input field, recent URLs from history, current
  status (idle / launching / classifying / filling / done).
- Keybinds: `b` from any tab → jump to Browse, `Enter` submits
  URL, `q` bails out and kills the launched browser.
- Update `tabs_constant_matches_documented_count` test → 4.

### Phase F — `atsisbroken go <url>` subcommand (~half day)

- Same code path as Browse tab, but stdin/stdout-only for
  scripting.
- Pipes for batch use:
  ```
  cat job-urls.txt | xargs -I{} atsisbroken go {}
  ```
- Per-URL summary printed after each fill.
- Tests:
  - Smoke: `atsisbroken go --help` lists URL arg.
  - Smoke: `atsisbroken go file://fixture.html` writes
    feedback events.

---

## 6. Anti-bot posture (ethics)

- **No auto-submit.** Constraint C1 is non-negotiable. The user
  always clicks Submit. This is documented in the README and the
  user is reminded at the end of every fill: "Tab to your browser
  to review and submit."
- **Visible window.** Constraint C3. atsisbroken never runs the
  launched browser headless during the live `go` flow.
  (Headless is fine for `tui-snapshot` / dev tooling.)
- **One URL at a time per session.** No concurrent attacks on
  multiple sites.
- **Rate limiting** at the script level: `atsisbroken go --rate-limit=PER-MIN`
  caps how many fills happen per minute when piped from a script.
  Default: 6/minute (one every 10 seconds), generous for human
  review pace.
- **Robots.txt respected** by the launched browser by default;
  atsisbroken doesn't override it.
- **Per-platform allowlist** at first: ship with Greenhouse, Lever,
  iCIMS, Workday, Ashby, SmartRecruiters as known-good. Other URLs
  prompt: "atsisbroken hasn't seen this domain before. Continue?"
  This is a *speed bump*, not a firewall — it just makes
  accidental scope-creep visible.

---

## 7. TUI surface changes

```
atsisbroken  0.0.0                   mode: training_wheels    queue: 0 events
dashboard  •  queue  •  strategy  •  browse

  go to URL   ▎_______________________________________

  recent
    https://boards.greenhouse.io/acme/jobs/1234   2 days ago   filled
    https://jobs.lever.co/example/abc-def         5 days ago   filled
    https://example.workday.com/job/anywhere/123  1 week ago   reviewed

  status   idle
```

Status states (right side of the tab body):
- `idle` — no browser launched
- `launching <browser path>` — browser subprocess starting
- `connecting <port>` — WebSocket attaching
- `classifying N fields` — CDP snapshot done, model running
- `filling i/N: <field label>` — per-field progress
- `awaiting submit` — user's turn
- `done — N filled, M skipped` — summary

---

## 8. Tests + acceptance criteria

| Property | Test | Phase |
|---|---|---|
| Default-browser detection returns *something* on dev box | `default_browser_returns_some` | A |
| Linux xdg-settings parser (mocked) | `parse_xdg_settings_chromium` | A |
| macOS plist parser (mocked) | `parse_lsregister_chrome` | A |
| Windows registry-path mock | `parse_userchoice_chrome` | A |
| Firefox detection downgrades to userscript | `firefox_default_falls_to_userscript` | A |
| Launch command has expected flags | `launch_command_has_remote_debugging_port` | B |
| LaunchedBrowser::Drop kills subprocess | `launched_browser_drops_kill_subprocess` | B |
| Random port allocator returns unprivileged port | `random_port_in_unprivileged_range` | B |
| WebSocket connect → version reports protocol ≥1.3 | `cdp_session_connect_yields_version` | C |
| Snapshot of fixture HTML → correct FieldDescriptors | `snapshot_form_extracts_known_fields` | C |
| Fill JS dispatch shape pinned | `fill_js_dispatches_input_and_change` | D |
| Verifier detects Workday re-render | `verifier_retries_once_then_surfaces` | D |
| Browse tab is the 4th tab | `tabs_constant_matches_documented_count` (updated) | E |
| `cmd_go` rejects unknown domains without `--allow-any` | `go_unknown_domain_prompts` | F |
| `--rate-limit` caps fills per minute | `rate_limit_pacer` | F |

Acceptance: Phase A–F ship green per phase. After F, the TUI's
Browse tab does the documented end-to-end flow against a real
Greenhouse posting on the developer's machine.

---

## 9. Risk table

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Chrome version pin breaks `--remote-debugging-port` flag | Low | Medium | CI matrix tests against the last 3 stable Chrome releases |
| User's default is Firefox; `CdpLaunch` falls back to atsisbroken's Chromium subprocess | Medium | Low | Documented; userscript path stays available |
| `xdg-settings` not installed on minimal Linux | Medium | Low | Read `BROWSER` env var, then `which firefox/chromium` |
| Anti-bot fingerprinting flags the launched browser as automated | Medium | Medium | Avoid `--headless`; use a plausible user-data-dir; rate-limit |
| User runs out of disk on temp `--user-data-dir` | Low | Low | `tempfile`-managed; cleaned on Drop |
| Port allocator races and chrome fails to bind | Low | Low | Retry up to 3 times with new random ports |
| User pipes 1000 URLs and gets recruiter-flagged | Medium | High | Default rate limit; max 6/min unless overridden; doc warning |
| Local model is wrong and fills bad data | High before classifier ships, Low after | High | TrainingWheels confirms each value; Shadow only fills above threshold |
| Browser subprocess outlives atsisbroken | Low | Medium | Drop impl SIGTERMs; ctrl-C handler also kills |

---

## 10. Open questions

1. Should the launched browser inherit the user's Chrome profile
   (cookies, autofill credentials), or use a clean temp profile?
   - Inheriting → user feels at home, but credential leakage if
     atsisbroken is buggy.
   - Clean temp → safer, but the user has to log into job-board
     accounts every session.
   - **Default proposal:** clean temp; offer
     `atsisbroken go --use-default-profile` as an opt-in.
2. Should we cache the form layout per-domain so repeat applies
   skip the classifier entirely?
   - **Proposal:** yes, blake3-keyed by `(domain, form-DOM-hash)` →
     cached `Vec<FieldDescriptor>`. Cuts inference cost on the
     50th Greenhouse posting to ~zero.
3. What about CAPTCHA at submit time?
   - **Out of scope.** atsisbroken doesn't try to solve CAPTCHAs;
     the user is at the keyboard for submit. Document that.

---

## 11. Forward-link

This plan is bridged into:
- `BACKLOG.md` — items A1–F2 added under "Now" (browser-automation cluster)
- `PLAN.md` — Phase 1.5 added between current Phases 1 and 2
- `TIMELINE_OF_INVENTION.md` — to be updated when each phase ships

Sources cited in this document:
- [Chrome DevTools Protocol — chromedevtools.github.io](https://chromedevtools.github.io/devtools-protocol/)
- [BrowserStack — Connecting to existing browser via CDP](https://www.browserstack.com/guide/playwright-connect-to-existing-browser)
- [xdg-settings reference — portland.freedesktop.org](https://portland.freedesktop.org/doc/xdg-settings.html)
- [slatecave — xdg-settings: Setting a default browser isn't that simple](https://slatecave.net/blog/xdg-settings/)
- [x-default-browser (cross-platform reference impl, JS)](https://github.com/jakub-g/x-default-browser)
- [Sprout — AI Job Application Bots: The Complete Guide for February 2026](https://www.usesprout.com/blog/ai-job-application-bots-complete-guide)
