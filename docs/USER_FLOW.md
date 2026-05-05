# atsisbroken — User Flow: Install → First Fill → Graduation

End-to-end walkthrough. Every step is something a real user does or
the system does on their behalf. No hand-waving.

---

## 0. The product the user sees

Two artifacts:
- A single desktop binary `atsisbroken` (downloaded from GitHub Releases).
- A Chrome extension (Web Store, or "Load unpacked" until we ship).

Both are free. No accounts. No cloud. Forever.

---

## 1. Install (one-time, ~3 minutes)

### 1a. Desktop binary
1. User downloads the binary for their platform from
   `github.com/cochranblock/atsisbroken/releases`.
2. macOS: `chmod +x atsisbroken && xattr -d com.apple.quarantine atsisbroken`.
   Linux: `chmod +x atsisbroken`. Windows: just run it.
3. User runs `atsisbroken init` once. Pastes resume text when prompted.
   The binary parses it into a `Profile`, writes
   `~/.atsisbroken/profile.toml`, and seeds the local classifier from
   `assets/seed-corpus.jsonl` + the user's resume-derived pairs.
   Default mode: `TrainingWheels`.

### 1b. Chrome extension
4. User installs the extension. Chrome assigns it a permanent extension
   ID (32 chars).
5. User runs `atsisbroken install-bridge` (one shot). The binary writes
   the Native Messaging host manifest to the right per-OS directory,
   pointing at itself, with the extension ID baked in. (User pastes the
   ID once; the binary handles the path/registry.)
6. User reloads the extension. Click the popup → "Test connection". The
   extension opens a Native Messaging port; the binary's `bridge`
   subcommand answers `HelloAck`. Green check.

That's it. Nothing else to set up. No browser-Chrome-version-pinning,
no JS toolchain, no kernel extensions, no admin password.

---

## 2. First job application (today, 2026-05-05)

7. User navigates to a real ATS form (Workday / Greenhouse / Lever /
   iCIMS / etc.) and copies its URL.
8. User runs `atsisbroken run --url <that URL>`. The binary:
   - launches chromium via `chromiumoxide::Browser::launch`,
   - opens the URL,
   - waits 1.5 s for hydration,
   - snapshots every `<input>`/`<textarea>`/`<select>` via
     `Runtime.evaluate`,
   - classifies each via `predict_field_key` (keyword classifier today),
   - fills every confident-classified field whose key has a non-empty
     profile value, dispatching `input` and `change` events,
   - takes a full-page screenshot to
     `~/.atsisbroken/run-<unix>.png`,
   - appends one `Feedback{accepted: true}` event per fill to
     `~/.atsisbroken/feedback.jsonl`,
   - exits. **Never submits.**
9. The launched browser stays open. User reviews the filled form
   in the visible window and clicks Submit themselves.

What's in scope for v0.1 vs the original spec:

- The **per-field yes/no prompt** for TrainingWheels mode is **not
  yet implemented**. Today every classified field with a profile
  value gets filled. Mode flag is plumbed through CLI but ignored
  by the run loop. (Tracked: BACKLOG "Now — other".)
- The Chrome extension + Native Messaging bridge **exist as code**
  and have a unit-test gate, but the bridge has not been spoken to
  by a real Chrome instance in this session. `chrome.storage.local`
  observation queueing in the extension is real; the connect-native
  drain hasn't been verified end-to-end with a live extension yet.
- Per-field `Observation` events from the extension would land in
  the same `feedback.jsonl` once that bridge is live.

---

## 3. Building confidence (~5–20 applications)

11. Each subsequent application produces more `(FieldDescriptor → key)`
    examples. The classifier updates online (no retrain-from-scratch).
12. The user starts noticing: easy fields (email, phone, full name)
    rarely need correction. The yes/no prompts become annoying.
13. User runs `atsisbroken graduate`. Confirmation prompt. They confirm.

---

## 4. Shadow mode (most users live here)

14. Mode flips to `Shadow`. No more yes/no prompts.
15. New behavior:
    - User opens an ATS form. Fields the model is confident on
      (email, phone, full name, LinkedIn — anything where confidence
      ≥ `ConfidenceThreshold`, default 0.85) **auto-fill silently**.
    - Fields below threshold (e.g. weird custom labels, free-text
      essay questions) are **left blank**. User fills those by hand.
    - The binary watches every manual fill and records it as an
      `Observation`. Each consistent observation pushes that field
      type's confidence higher.
16. Within a couple weeks the typical user has model confidence
    ≥ 0.85 on ~80% of common ATS fields. They stop seeing the popup
    confirmations entirely.

This is the steady state for the median user. The model keeps
graduating field-types one at a time, on the user's own data.

---

## 5. Chaos mode (power users)

17. User who's been running Shadow for a while runs
    `atsisbroken graduate --chaos`. Confirmation. Confirmed.
18. Mode flips to `Chaos`. Every classified field auto-fills, even
    low-confidence ones. The user can still flag a wrong fill via the
    extension popup ("This was wrong" button) — that becomes a
    retroactive negative training example.

---

## 6. Optional: opt-in feedback delivery

19. User runs `atsisbroken sync-config --destination mailto:me@example.com`
    (or `--destination https://my.hook/feedback`). This sets
    `FeedbackDelivery::SendWhenOnline { destination }` in
    `~/.atsisbroken/config.toml`. **Default is and stays `LocalOnly`
    until the user runs this.**
20. Now `atsisbroken sync` (or the extension's auto-sync alarm) drains
    the queue to the destination. Each event ships only the field
    shape + the predicted/observed keys — never the user-typed value.

---

## 6.5. Strategy ladder — when CDP isn't available

`atsisbroken run` does not assume Chromium is reachable. It detects the
environment and picks the highest-tier fill strategy that works, then
falls through if the higher tiers can't run. Every user reaches *some*
working tier; nobody hits a "couldn't autofill, sorry" wall.

| Tier | Strategy | When it triggers |
|---|---|---|
| 1 | `CdpAttach`   | Chromium with reachable `--remote-debugging-port` |
| 2 | `CdpLaunch`   | A Chromium-family binary findable on the system |
| 3 | `Extension`   | atsisbroken extension's native-messaging host installed |
| 4 | `Userscript`  | TamperMonkey / Greasemonkey works; emit a userscript |
| 5 | `Bookmarklet` | emit a `javascript:` URL the user drags to the bookmark bar |
| 6 | `Clipboard`   | `pbcopy`/`xclip`/`wl-copy`/`clip.exe` on this OS — `atsisbroken copy <key>` per field |
| 7 | `Speak`       | print `key: value` lines to stdout — the floor; works literally anywhere |

Force a specific tier with `atsisbroken run --strategy=NAME`. Each tier
also has its own subcommand for direct use:

```sh
atsisbroken userscript      # emit TamperMonkey script with profile baked in
atsisbroken bookmarklet     # emit javascript: URL
atsisbroken copy email      # copy a single field to system clipboard
atsisbroken speak           # print profile values to type by hand
atsisbroken cdp-probe       # diagnose Chromium debug port reachability
```

This is intentional: a paid SaaS product fails closed when its
infrastructure isn't there. atsisbroken fails *down* through tiers
until something works. The user always gets *some* leverage.

## 7. Surfaces, summarized

| Surface | Used in flow |
|---|---|
| Desktop binary `atsisbroken` (CLI) | install, init, run, graduate, bridge, sync, status |
| Chrome extension (content + popup) | passive observation, popup-triggered fill, manual sync |
| Native Messaging bridge | extension → desktop, in-process pipe, no network |
| `~/.atsisbroken/profile.toml` | the user's structured profile |
| `~/.atsisbroken/feedback.jsonl` | append-only training ledger |
| `~/.atsisbroken/config.toml` | mode, threshold, delivery destination |

---

## 8. What the user *never* has to do

- Sign up.
- Pay.
- Allow a vendor to host their resume.
- Pick a model from a dropdown.
- Configure a cloud API key.
- Trust a third party with their training data.
- Manage ".env" files or credentials.

If any of those creep into the flow, we've broken the thesis.
