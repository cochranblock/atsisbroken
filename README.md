# atsisbroken

**ATS is broken.** A free, local-first, no-account, no-cloud alternative to
Simplify.us. Resume autofill that runs entirely inside your browser.

By **The Cochran Block, LLC**. Public domain — `Unlicense`. Forever.

---

## Status

Pre-alpha scaffold. The workspace compiles to native + `wasm32-unknown-unknown`.
The Chrome-extension JS surface is not yet implemented. See
[`docs/TIMELINE.md`](docs/TIMELINE.md) for what shipped when.

## Architecture

Path B — **Chrome extension + WASM core.**

- `core/` — `cdylib + rlib`. Profile schema, field-deduction. Compiles to
  WASM for the extension; usable as a normal Rust lib for the CLI.
- `cli/` — local profile management. Resume parsing, profile editing,
  outside-the-browser inference.
- `ext/` — Chrome MV3 extension. Loads the WASM core. Background script,
  content script, popup all TBD.
- Sibling deps: `kova-engine` (inference, no HuggingFace fallback) and
  `pixel-forge` (model training pipeline). Both authored by
  The Cochran Block, LLC.

Path-decision rationale (A vs B vs C) and architectural reasoning are in
[`docs/TIMELINE.md`](docs/TIMELINE.md).

## Build

```sh
# Native (CLI + core as rlib)
cargo build --workspace

# Speed-Diamond CLI binary
cargo build --profile=diamond -p atsisbroken-cli

# Size-Diamond WASM core (every byte ships in the extension)
cargo build --profile=diamond-edge -p atsisbroken-core --target wasm32-unknown-unknown
```

## Verify

```sh
cargo test -p atsisbroken-core
```

For the exopack TRIPLE SIMS determinism gate (kova
`exopack::triple_sims::f61` semantics), see
[`docs/PROOF.md`](docs/PROOF.md).

## Docs

- [`docs/USER_STORIES.md`](docs/USER_STORIES.md) — personas, adversarial
  scenarios, P0 next moves.
- [`docs/TIMELINE.md`](docs/TIMELINE.md) — invention timeline, dated and
  artifact-anchored.
- [`docs/PROOF.md`](docs/PROOF.md) — reproducible build + test verifications.
- [`LICENSE-PROVENANCE.md`](LICENSE-PROVENANCE.md) — author, dedication,
  trademark posture.
- [`UNLICENSE`](UNLICENSE) — the dedication itself.

## License

Unlicense. The brand "The Cochran Block, LLC" and the project name
`atsisbroken` are not licensed under the Unlicense — see
[`LICENSE-PROVENANCE.md`](LICENSE-PROVENANCE.md) for the trademark split.
