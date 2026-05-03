# atsisbroken — Timeline of Invention

Authoritative invention timeline for IP / prior-art / provenance purposes.
Every event is anchored to a verifiable artifact: a git commit hash, a file
mtime, or a tmux session log on the IRONHIVE cluster (The Cochran Block, LLC hardware).

Author: Michael Cochran (GotEmCoach). Org: The Cochran Block, LLC.
License: Unlicense (public domain dedication).

---

## 2026-05-01 — Conception & initial scaffold

**Event.** `atsisbroken` repository conceived as a free, local-first alternative
to Simplify.us. Original architectural framing: Chrome extension + Rust/WASM
core, inference via Chrome's built-in AI (Gemini Nano Prompt API).

**Artifacts.**
- `Cargo.toml` initial mtime: `2026-05-01` (pre-rewrite).
- `cli/`, `core/`, `ext/manifest.json`, `UNLICENSE` directories created
  `2026-05-01 14:02` local time on `kova-thick-beast` (n2/bt) IRONHIVE node.

## 2026-05-01 — Architectural pivot

**Event.** Built-in-AI dependency (Gemini Nano) rejected on principle.
The Cochran Block, LLC's "no open-source models, no cloud, no premium tier" stance
required a custom-trained, locally-bundled model. Stack realigned:
`pixel-forge` (training pipeline) → custom `.safetensors` field-deduction
model → `kova-engine` inference + browser interface → `exopack` TRIPLE SIMS
gate.

**Path decision tree captured.** Three options analyzed:
- **A.** Native Rust binary drives Chrome via `enigo` + screen capture.
- **B.** Chrome extension with WASM-compiled core. Model bundled.
- **C.** Hybrid — extension + native helper via `chrome.runtime.connectNative`.

**Decision: Path B.** Driver: user-friction parity with Simplify.us. One-click
install is the only viable distribution surface for a free product competing
on UX. (Path A is faster to ship but loses to install friction.)

**Artifacts.**
- `Cargo.toml` rewritten to point `kova-engine` and `pixel-forge` at sibling
  workspace paths.
- DIAMOND RUST BINARY ARCHITECTURE profiles wired:
  `[profile.diamond]` (speed-Diamond, CLI / daemon) and
  `[profile.diamond-edge]` (size-Diamond, WASM core). Canonical reference:
  `https://cochranblock.org/diamond-profile.toml`.

## 2026-05-03 — Path B scaffold completed and committed

**Event.** Kova-drives-Chrome assumption removed from the workspace; WASM
boundary materialized.

**Concrete changes** (commit `e09cf05`, root commit, `master`):
- Workspace `kova-engine` dep stripped to `inference`-only feature; `browser`
  and `screenshot` features dropped (those are the Path A paradigm).
- `wasm-bindgen` and `serde-wasm-bindgen` added as workspace dependencies.
- `core/` configured as `cdylib + rlib`. `wasm-bindgen` deps gated under
  `cfg(target_arch = "wasm32")` so native CLI builds skip them.
- `core::predict_field_key(&FieldDescriptor) -> &'static str` — deterministic
  keyword-heuristic field-deduction stub. To be replaced by the trained
  `.safetensors` model loaded via `kova-engine` inference.
- WASM exports added: `predict_field_key_js(JsValue) -> Result<String, JsValue>`
  and `version_js() -> String`. These are the JS↔WASM boundary the Chrome
  extension's content script will call.

**Verifications run on this date:**
- `cargo check --workspace` — clean.
- `cargo check -p atsisbroken-core --target wasm32-unknown-unknown` — clean.
- `cargo test -p atsisbroken-core` — 3/3 tests pass (
  `predict_field_key_matches_fixtures`,
  `triple_sims_determinism`,
  `no_fabrication_for_unknown_fields`).
- Exopack TRIPLE SIMS gate (kova `exopack::triple_sims::f61` semantics):
  three sequential `cargo test` invocations, byte-identical output. SHA-256
  of normalized test output, three runs, all match. Hash recorded in
  `docs/PROOF.md`.

**Repository published:** `https://github.com/cochranblock/atsisbroken`,
public, default branch `master`. Created via `gh repo create` from the Mac
Mini (`mm`) under the `cochranblock` GitHub organization. Pushed from
`kova-thick-beast` (n2/bt) over SSH using the `kovakey` identity.

## Forward plan (not yet artifacted)

- Train custom field-deduction model in `pixel-forge`.
- Bundle resulting `.safetensors` into `core/`; load via `kova-engine`
  inference in WASM.
- Implement `ext/background.js`, `ext/content.js`, `ext/popup.html` —
  the user-facing Chrome extension surface.
- CSP-harden `ext/manifest.json`: `connect-src 'none'`, `script-src 'self'
  'wasm-unsafe-eval'`. Structurally enforces "no network".
- Replace `master` → `main` for default-branch parity with the rest of the
  cochranblock repos.
