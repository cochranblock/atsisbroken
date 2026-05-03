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
`~/.atsisbroken/`. The user's model is the only model. Two autonomy
modes: `TrainingWheels` (yes/no per fill, online learning) and `Chaos`
(autonomous fill, post-hoc flagging still trains).

## Forward plan

- CDP loop in `src/main.rs` (currently stubbed).
- Trainer + online updater for `TrainingWheels` feedback.
- In-page CDP overlay for the yes/no prompt.
- Cross-compile pipeline: GitHub Actions + `--profile=diamond-edge`.
- `master` → `main` for default-branch parity.
