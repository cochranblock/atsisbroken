# atsisbroken — Contributors

Authoritative attribution. Mirrors the `Contributors:` line in
`Cargo.toml` and the per-file `// Contributors:` headers.

## People

- **Michael Cochran** (GotEmCoach) — design, architecture, code, lead.
  `mcochran@cochranblock.org`. Cochran Block, LLC.

## AI assistants used during development

- **KOVA** — sibling Cochran Block engine. Inference loader (`kova-engine`)
  is a path dependency. Authored by GotEmCoach.
- **Claude Opus 4.7** (Anthropic) — pair-programming assistant during
  scaffold sessions on 2026-05-01 and 2026-05-03. No copyrightable
  contribution claimed by the assistant; outputs are work-for-hire to
  the human author per Anthropic's terms.

## Sibling repos credited

- `kova` — inference engine. Same author. Unlicense.
- `pixel-forge` — model training pipeline patterns. Same author.
  Unlicense.
- `exopack` — TRIPLE SIMS determinism gate (now absorbed into kova).
  Same author. Unlicense.

## Third-party data attribution

- **Chromium** authors — autofill heuristics test corpus (BSD-3-Clause)
  is the primary upstream training data. See
  [`docs/TRAINING_DATA.md`](docs/TRAINING_DATA.md). Required notice
  ships alongside any binary distribution that includes derived data.

## How to contribute

PRs welcome. The thesis is non-negotiable: free, local-first, no
cloud, no accounts, no premium tier. Anything that breaks that
property gets rejected. Otherwise: open a PR.

License contributions under the repo's `Unlicense`. Add yourself to
this file in the same PR.
