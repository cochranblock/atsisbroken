# atsisbroken — Proof of Artifacts

Verifiable evidence that the claims in `TIMELINE.md` are real. Every entry
here is reproducible from the repo at the named commit.

---

## Commit ledger

| Date (UTC-4) | Hash | Subject |
|---|---|---|
| 2026-05-03 09:43:19 | `e09cf05` | Initial scaffold: Path B (Chrome extension + WASM core) |

`git log --pretty=format:"%h %ai %s"` reproduces this ledger.

---

## Build verifications (commit `e09cf05`, on `kova-thick-beast` n2/bt)

### Native workspace
```
$ cargo check --workspace
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.00s
```
Status: **PASS**.

### WASM core (`wasm32-unknown-unknown`)
```
$ cargo check -p atsisbroken-core --target wasm32-unknown-unknown
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 10.04s
```
Status: **PASS**. The Path B target compiles.

---

## Exopack TRIPLE SIMS — determinism gate

The TRIPLE SIMS gate (kova `exopack::triple_sims::f61`, N=3) requires three
sequential successful `cargo test` runs in the project directory. Stronger
property exercised here: **the output of all three runs is byte-identical**,
proving determinism — not merely that all three pass.

### Method
1. `cargo test -p atsisbroken-core --quiet` invoked three times.
2. The `^test ` and `result:` lines of each run extracted, sorted, hashed
   with SHA-256.

### Results

| Sim | SHA-256 |
|---|---|
| 1 | `508ccf0caab04524322354faa6cd646705f291e95d5e29834ab9816384b8094d` |
| 2 | `508ccf0caab04524322354faa6cd646705f291e95d5e29834ab9816384b8094d` |
| 3 | `508ccf0caab04524322354faa6cd646705f291e95d5e29834ab9816384b8094d` |

**All three hashes identical.** Determinism contract held. Status: **PASS**.

### Reproduce locally
```sh
for i in 1 2 3; do
  cargo test -p atsisbroken-core --quiet 2>&1 \
    | grep -E "^test |result:" | sort | sha256sum
done
```
All three lines must be identical.

### In-process determinism
The same property is also asserted inside the test suite, so any future
non-determinism breaks `cargo test` directly (no external runner needed):

- `core::tests::triple_sims_determinism` — runs `predict_field_key` over the
  full fixture set three times, asserts all three result vectors are
  byte-identical.

---

## Test coverage of user-trust properties

| Property | Test | Status |
|---|---|---|
| Determinism (P2 / A7) | `triple_sims_determinism` | PASS |
| No fabrication for unknown fields (P6 / A6) | `no_fabrication_for_unknown_fields` | PASS |
| Heuristic correctness over canonical fixture set | `predict_field_key_matches_fixtures` | PASS |

Personas and adversarial scenarios are catalogued in `docs/USER_STORIES.md`.

---

## Network-surface audit

`atsisbroken-core` dependency tree at commit `e09cf05`:
- `serde`, `serde_json` — pure data.
- `wasm-bindgen`, `serde-wasm-bindgen` — JS bridge, no I/O.

No HTTP client. No socket library. No DNS resolver. No file system access.
The crate is structurally incapable of phoning home.

This is enforced at the build level: any future addition of a networking
crate to `core/Cargo.toml` is reviewable in a single diff.

---

## Repository publication

- URL: https://github.com/cochranblock/atsisbroken
- Visibility: PUBLIC
- Default branch: `master`
- Created: 2026-05-03 via `gh repo create cochranblock/atsisbroken --public`
  from `mm` (Mac Mini), authenticated as `GotEmCoach`.
- Pushed from `kova-thick-beast` (n2/bt) over SSH (`git@github.com`,
  identity `~/.ssh/kovakey`).

`gh repo view cochranblock/atsisbroken --json url,visibility,defaultBranchRef`
reproduces the publication record.
