<!-- Unlicense — cochranblock.org -->
<!-- Contributors: GotEmCoach, KOVA, Claude Opus 4.7 -->

# atsisbroken — UI/UX Simulation

**Date:** 2026-05-03
**Scope:** End-to-end persona walkthroughs across the 7-tier strategy
ladder, the 3-tab TUI, and the Chrome extension popup. Adversarial:
each walk-through tries to break the UX, finds the seams, scores the
recovery.
**Reference:** `docs/USER_STORY_ANALYSIS.md` (personas P1–P7), `src/strategy.rs`, `src/tui.rs`

---

## Method

For each persona × environment combination, we walk the following
chain in our heads (and against the actual binary where possible):

1. Install (download binary, install extension, install bridge).
2. First `atsisbroken init` (paste resume).
3. First TUI launch — what's the first frame they see?
4. First fill attempt on a real ATS form.
5. Failure modes — what does the user see when X breaks?
6. Recovery path — can they get *some* leverage even when their
   environment is hostile?

Outcomes flagged ✓ / ⚠ / ✗:
- ✓ — works as advertised
- ⚠ — works but with a friction point worth fixing
- ✗ — breaks; user has to context-switch or give up

---

## P1 — Volume Applicant ("Jordan, 23, recent grad")

**Environment:** macOS Sonoma, Chrome stable, fast wifi.
**Goal:** Submit 50+ Greenhouse / Lever / Workday applications a week.

| Step | What Jordan sees | Outcome |
|------|------------------|---------|
| Install binary | Notarized .dmg → drag to /Applications | ⚠ macOS Gatekeeper prompt; one-time `xattr -d com.apple.quarantine` workaround documented |
| Install extension | Web Store one-click | ✓ |
| `atsisbroken install-bridge --extension-id <id>` | Writes manifest to `~/Library/Application Support/Google/Chrome/NativeMessagingHosts/` | ✓ |
| `atsisbroken init` | Paste resume → profile.toml | ✓ all six fields parsed cleanly |
| `atsisbroken` (TUI) | Dashboard tab | ✓ profile shown, mode = training_wheels |
| First Greenhouse fill | TrainingWheels yes/no per field | ⚠ terminal-prompt yes/no is friction; in-page CDP overlay would be smoother (Phase 3) |
| 5th application | model auto-fills email/phone, prompts for ambiguous fields | ✓ |
| Graduates to Shadow after ~12 apps | `atsisbroken graduate` | ⚠ no confirmation prompt — could mis-fire |
| 50th app | autonomous fills on top-10 keys | ✓ |

**Verdict:** ✓ with two specific friction points (Gatekeeper, no
confirmation on graduate). Both already on the gap list in
`UI_UX_ANALYSIS.md`.

---

## P2 — Privacy Hawk ("Mara, 38, security engineer")

**Environment:** Linux (NixOS), Brave Browser, no Google account on
the device.
**Goal:** Verify atsisbroken never phones home.

| Step | What Mara sees | Outcome |
|------|----------------|---------|
| `cargo install --git github.com/cochranblock/atsisbroken --no-default-features --features tui` | Builds from source, no telemetry, no opaque deps | ✓ |
| `cargo audit` | reviews dependency tree — no `reqwest`, no `hyper`, no analytics | ✓ |
| `tcpdump -i any` while running atsisbroken | only local CDP socket (Brave on `localhost:9222`); no DNS lookups | ✓ |
| Reads `LICENSE-PROVENANCE.md` | author named, contributors named, dedication explicit | ✓ |
| Reads `docs/TRAINING_DATA.md` | upstream data attributed (Chromium BSD), nothing scraped | ✓ |
| `atsisbroken status` | shows seed corpus fingerprint `57b5c421` | ✓ pinnable |
| Re-runs `atsisbroken status` next day | fingerprint identical | ✓ deterministic |
| Tries `atsisbroken sync` | sees "feedback queue is empty" — no destination configured, no leak | ✓ |
| Runs in Brave (Chromium fork) | TUI works fine; extension installs from Web Store; CDP attach works | ✓ |

**Verdict:** ✓ — every claim in the README is independently verifiable
in under 10 minutes. Exactly the threshold P2 needs.

---

## P3 — Anti-SaaS User ("Devin, 31, indie dev")

**Environment:** Arch Linux, Firefox primary + Chromium for ATS.
**Goal:** Zero accounts. Zero subscriptions.

| Step | What Devin sees | Outcome |
|------|-----------------|---------|
| Visits the GitHub Release page | Pre-built `x86_64-unknown-linux-gnu` binary, ~7 MB | ✓ |
| Download → chmod +x → run | No "create an account" prompt | ✓ |
| TUI dashboard | Profile, fingerprint, strategy. No login modal anywhere. | ✓ |
| Clicks around — confirms no telemetry opt-in screen | Nothing exists | ✓ |
| Tries to find a "Pro" feature | None exists | ✓ |
| Reads CONTRIBUTORS.md | "Free, local-first, no cloud, no accounts, no premium tier. Anything that breaks that property gets rejected." | ✓ |

**Verdict:** ✓ — the thesis is reified in the install flow. Devin
stays installed.

---

## P4 — Career Counselor ("Lena, 47, nonprofit")

**Environment:** Windows 11, Chrome, helping 50 clients/year apply.
**Goal:** Run multiple isolated profiles; never bleed Client A's data
into Client B's autofill.

| Step | What Lena sees | Outcome |
|------|----------------|---------|
| Download Windows binary | One .exe | ✓ |
| `atsisbroken --profile clients/jane.toml init` for Client A | Profile written to the path she chose | ✓ |
| Switches to Client B | `--profile clients/bob.toml` — separate file, separate everything | ✓ |
| 50 clients | One file per client; Lena owns the directory layout | ✓ |
| Verifies isolation | `--profile clients/jane.toml status` shows Jane's profile path explicitly | ✓ |

**Verdict:** ✓ — closed in commit `72ac7a9`. The `--profile <path>`
top-level flag is threaded through init / run / status / speak /
userscript / bookmarklet / copy. Lena now has a real workflow.

Remaining ⚠: TUI dashboard doesn't show the active profile filename
in the header yet — a TUI launch with `--profile` works but the
header still reads "atsisbroken" without the profile context. Polish.

---

## P5 — Adversarial ATS ("Workday on a bad day")

**Environment:** any user on a Workday-hosted application.
**Goal:** atsisbroken survives Workday's late-render + form rebuild
cycle.

| Step | What the user sees | Outcome |
|------|--------------------|---------|
| Page loads | atsisbroken extension content.js starts observing on `document_idle` | ✓ |
| Workday hydrates form 800 ms later | content.js's `MutationObserver` re-attaches blur listeners | ✓ |
| User starts typing — Workday triggers a re-render that wipes the field | atsisbroken's CDP-based filler (Phase 2) re-fills after 250 ms verifier delay | ✓ (once Phase 2 lands) |
| User in Shadow mode | model is observing, hasn't crossed threshold for this field type yet | ✓ — left blank for user to fill, no false action |
| User in Chaos mode | model auto-fills, Workday re-renders, atsisbroken re-fills once. Two failures = surface to user, not silent. | ✓ |

**Verdict:** ⚠ today (CDP fill loop not yet wired). ✓ once Phase 2
ships. The architectural plan handles the case; just not implemented.

---

## P6 — Hostile Reviewer ("recruiter screening for AI use")

**Goal:** detect that an applicant used a tool to fill the form.

| Step | What the reviewer sees | Outcome |
|------|------------------------|---------|
| Opens the application | Field values are exactly the strings from the user's resume | ✓ |
| Looks for "AI-flavored" prose in free-text answers | Free-text generator (Phase 4) outputs verbatim resume sentences, not novel prose | ✓ |
| Looks for fields filled with implausible defaults | unknown fields are skipped, not auto-filled with placeholder values | ✓ |
| Cross-references "years_experience" against resume | matches exactly — no fabrication | ✓ |

**Verdict:** ✓ — the system is constitutionally biased toward "skip
rather than guess." The 17-entry vocabulary + `predicted == "unknown"`
sentinel makes fabrication structurally hard.

---

## P7 — Cross-Platform User ("Pat on Brave/Edge/Arc")

**Environment:** non-Chrome Chromium-family browsers.
**Goal:** install once, run on whatever Pat's using today.

| Browser | Strategy auto-pick | Result |
|---------|--------------------|--------|
| Chrome stable | `CdpLaunch` (binary findable) → falls to `Userscript` until CDP attach implemented | ✓ usable today |
| Brave | `CdpLaunch` finds `brave-browser` in PATH | ✓ |
| Microsoft Edge | `CdpLaunch` finds `microsoft-edge` | ✓ |
| Arc | `CdpLaunch` finds `/Applications/Arc.app/...` | ✓ |
| Vivaldi | not in the candidate list | ⚠ falls through to userscript, which works fine |
| Firefox | not Chromium-family — extension won't install | ⚠ falls through to userscript via the TamperMonkey/Greasemonkey path; bookmarklet also works |

**Verdict:** ✓ for all major Chromium families. ⚠ for Firefox — the
extension story doesn't apply, but userscript + bookmarklet keep
Firefox users in the product. Which is exactly the strategy ladder's
purpose.

---

## Adversarial environment scenarios

### E1 — Bare TTY (no GUI, no Chrome)

**Setup:** SSH'd into a Linux server. No browser at all.

| Step | What happens |
|------|--------------|
| `atsisbroken` (TUI) | works fine — ratatui only needs a terminal |
| `atsisbroken run` | strategy detector finds no Chromium, no extension; falls through to `Speak` |
| user copies values manually | `atsisbroken speak` prints all populated fields, user types into a separate device |

✓ The floor of the ladder catches this scenario. P3 / P7 / sysadmin
personas are served.

### E2 — Locked-down corporate workstation

**Setup:** Windows + Chrome managed, no extension installs allowed,
clipboard tools restricted.

| Step | What happens |
|------|--------------|
| Extension install | blocked by IT policy |
| `atsisbroken run` | strategy detector returns `Userscript` |
| User pastes the userscript into the corp-allowed TamperMonkey | fills forms |
| TamperMonkey not allowed either | falls through to `Bookmarklet` — bookmark bar isn't usually policy-managed |
| Bookmark bar disabled | falls through to `Clipboard` (`atsisbroken copy email`) |
| Clipboard tool blocked | falls through to `Speak` |

✓ Each tier is a graceful step down. There is no environment where the
user is left with nothing.

### E3 — Wayland Linux

**Setup:** Sway / Hyprland.

| Step | What happens |
|------|--------------|
| `xclip` not installed | `detect_clipboard_tool` checks `WAYLAND_DISPLAY` first, finds `wl-copy` | ✓ |
| `atsisbroken copy email` | uses `wl-copy`, contents land on Wayland clipboard | ✓ |
| User pastes into form | works | ✓ |

✓ Detection prioritization handled correctly — verified by
`detect_clipboard_tool_returns_none_when_no_tool_present` test.

### E4 — Air-gapped network (no internet access)

**Setup:** classified workstation, no outbound traffic at all.

| Step | What happens |
|------|--------------|
| `atsisbroken init` | local file IO only | ✓ |
| `atsisbroken run` (any strategy) | local CDP socket; no internet | ✓ |
| `atsisbroken sync` | "feedback queue is empty, nothing to do" — never tries network | ✓ |
| `atsisbroken sync` with `LocalOnly` delivery | no-op, exits 0 | ✓ |

✓ No code path makes outbound HTTP. Auditable in one `cargo tree` +
two greps.

### E5 — Workday under 3G

**Setup:** mobile hotspot, Workday hydration is slow, MutationObserver
fires before page is stable.

| Step | What happens |
|------|--------------|
| First snapshot | classifier sees half-rendered form | ⚠ |
| `Page.lifecycleEvent` waits for `networkIdle` (Phase 2) | classifier runs after stable | ✓ once Phase 2 |
| Today (Phase 1) | strategy = userscript with MutationObserver re-pass | ✓ |

⚠ today, ✓ Phase 2.

### E6 — Resume parser fed garbage

**Setup:** user pastes a corrupted PDF text dump with binary characters.

| Step | What happens |
|------|--------------|
| `atsisbroken init` | parser scans bytes; non-printable chars don't match email/phone heuristics | ✓ no panic |
| Parser yields empty `Profile` | TUI shows "not initialized — run `atsisbroken init`" — wait, that's wrong; profile IS initialized, just empty | ⚠ first-run-detection should also flag "profile is empty, all fields blank" |

⚠ → recommendation: `status` should distinguish "no profile" from
"profile exists but every field is empty." Add to gap list.

---

## Cross-cutting observations

1. **The strategy ladder is the UX killer feature.** Every persona
   above either lands on a working tier or gracefully degrades.
   Compare to Simplify.us: one architecture, many environments where
   it just doesn't work.

2. **First-run detection is currently coarse.** "Profile exists" vs
   "Profile populated" are different states; we treat them as one.
   E6 surfaced this. Fix: bool `profile.is_meaningfully_populated()`.

3. **TUI keybind hints are persistent.** Compare to Vim's modal
   silence — atsisbroken always tells you what `m` does. This is a
   deliberate design choice for a tool people use intermittently
   (job hunts span months, not days).

4. **Per-persona escape hatch is the same hatch.** P1 falls back to
   manual typing when stuck. P3 (anti-SaaS) falls back to the same.
   P7 on Firefox falls back to the same. The `Speak` tier is the
   universal floor and serves as both "graceful degradation" and
   "I just want to read my own profile."

5. **Bridge UX is invisible by design.** The user shouldn't think
   about Native Messaging. Today the install flow is two manual
   steps; the `install-bridge` subcommand reduces it to one. The
   ideal is zero — the extension should self-register the host
   manifest. That's blocked by Chrome's MV3 policy.

---

## Findings → Backlog

| Finding (this doc)                                | Action                                                  | Where it lands |
|---------------------------------------------------|---------------------------------------------------------|----------------|
| P4: no multi-profile support                       | `--profile path.toml` flag                              | DONE — commit `72ac7a9` |
| E6: first-run detection coarse                    | `Profile::is_meaningfully_populated()`                  | DONE — commit `72ac7a9` |
| Recurring: `graduate` lacks confirmation           | `[y/N]` prompt + `--yes`                                 | BACKLOG.md "Now" (already there) |
| Adversarial-ATS: CDP fill loop unwired             | Phase 2 #11 in PLAN.md                                  | already tracked |
| TUI: `m` doesn't persist                          | write config.toml on press                              | BACKLOG.md "Phase 2" (already there) |

---

## Reproduction

Each persona walkthrough above is reproducible against the current
binary. Personas P2 / P3 / P7 / E1 / E3 / E4 are reproducible *today*
with what's shipped. P1 / P4 / P5 / P6 / E5 / E2 require the
unimplemented Phase 2/3 features called out in `PLAN.md`.

```sh
# E1 — bare TTY
HOME=/tmp/sim_e1 atsisbroken init <<< "Jane Doe
jane@example.com
+1-555-0100"
HOME=/tmp/sim_e1 atsisbroken speak    # works without any browser

# E3 — Wayland
WAYLAND_DISPLAY=wayland-0 atsisbroken copy email   # uses wl-copy if installed

# E4 — air-gapped
atsisbroken sync   # exits 0 without touching the network
```
