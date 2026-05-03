# atsisbroken — Chrome Extension

The browser-side observer. Watches form fields, records
`(FieldDescriptor → predicted_key)` events into `chrome.storage.local`,
and hands them off to the desktop `atsisbroken` binary via Chrome's
**Native Messaging** transport. **Never** records the user's typed
value.

## Layout

```
extension/
├── manifest.json                          # MV3, requests nativeMessaging perm
├── background.js                          # service worker, drains queue on alarm
├── content.js                             # observes blur events on inputs
├── popup.html / popup.js                  # tiny status UI + manual sync button
└── native-host-manifest.template.json     # native-messaging host descriptor
```

## How the bridge works

1. `content.js` listens for `blur` on every `<input>`/`<textarea>`/
   `<select>` (re-attached on DOM mutation for Workday-style late renders).
2. On blur, it builds a `FieldDescriptor`, runs a cheap keyword
   pre-classifier, and pushes the result into
   `chrome.storage.local.atsisbroken_observations`. The user's typed
   value never enters the queue.
3. Every 5 minutes (or on the popup's "Sync" button), `background.js`
   opens a Native Messaging port to `org.cochranblock.atsisbroken`.
   Chrome spawns the desktop binary as a subprocess.
4. The extension sends `{ kind: "hello", extension_version: "..." }`.
5. The desktop binary runs `atsisbroken bridge` (the new subcommand).
   It reads the framed JSON, replies with `{ kind: "hello_ack",
   host_version: "...", protocol_version: 1 }`.
6. The extension sends
   `{ kind: "push_observations", events: [...] }`.
7. The binary appends the events to its `FeedbackQueue`, replies with
   `{ kind: "ack", accepted: N }`.
8. The extension trims the first N events from
   `chrome.storage.local`. Closes the port. Done until next alarm.

## Install (developer mode, until we ship to the Web Store)

1. Build the desktop binary: `cargo build --profile=diamond -p atsisbroken`.
2. Open Chrome → `chrome://extensions` → enable Developer mode → Load
   unpacked → pick this `extension/` directory. Note the assigned
   extension ID (looks like `abcdefghijklmnopqrstuvwxyzabcdef`).
3. Copy `native-host-manifest.template.json` into Chrome's native
   messaging hosts dir (per-platform):
   - macOS: `~/Library/Application Support/Google/Chrome/NativeMessagingHosts/org.cochranblock.atsisbroken.json`
   - Linux: `~/.config/google-chrome/NativeMessagingHosts/org.cochranblock.atsisbroken.json`
   - Windows: registry under
     `HKCU\Software\Google\Chrome\NativeMessagingHosts\org.cochranblock.atsisbroken`
4. Replace the two placeholder strings:
   - `path`: absolute path to a wrapper script (or shim) that runs
     `atsisbroken bridge`. Chrome calls this with no args, so a
     two-line shell script is the simplest:
     ```sh
     #!/usr/bin/env bash
     exec /absolute/path/to/atsisbroken bridge
     ```
   - `allowed_origins`: `chrome-extension://<your-extension-id>/`.
5. Reload the extension. Click the popup → "Sync to local binary".
   The desktop process should receive an `Ack`.

## Privacy contract

- `chrome.storage.local` is per-Chrome-profile, never synced to a
  Google account, never leaves the device on its own.
- The extension never reads `el.value`. The queue contains only field
  shape and the predicted/observed *keys*.
- Native Messaging is a local pipe (Chrome ⇄ atsisbroken process). No
  network. No browser-to-cloud relay.
- The desktop binary's `bridge` subcommand stores received events in
  `~/.atsisbroken/feedback.jsonl`. From there, the same opt-in
  `FeedbackDelivery` policy applies: `LocalOnly` by default.
