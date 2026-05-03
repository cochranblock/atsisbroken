# atsisbroken — Proof of Artifacts

Verifiable evidence behind `TIMELINE.md`. Every entry reproduces from the
named commit.

## Commit ledger

| Date (UTC-4) | Hash | Subject |
|---|---|---|
| 2026-05-03 | `e09cf05` | Initial scaffold (Path B, since superseded) |
| 2026-05-03 | `f495a88` | Add provenance docs + exopack TRIPLE SIMS gate |
| 2026-05-03 | (this commit) | Pivot to single-binary CDP + on-demand user-trained model |

## Build verification (this commit, on n2/bt)

```
$ cargo check
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.79s
```

## Exopack TRIPLE SIMS gate

`cargo test` × 3, byte-identical normalized output.

| Sim | SHA-256 |
|---|---|
| 1 | `5661f393e2bd730eb80cce11066ac0d483605ca29c6743786295a1f628ae07e8` |
| 2 | `5661f393e2bd730eb80cce11066ac0d483605ca29c6743786295a1f628ae07e8` |
| 3 | `5661f393e2bd730eb80cce11066ac0d483605ca29c6743786295a1f628ae07e8` |

40/40 tests pass on every sim (33 atsisbroken + 7 atsisbroken-android).
Tautological tests removed; remaining suite is behavioral or schema-
stability checks. Reproduce:
```sh
for i in 1 2 3; do
  cargo test --quiet 2>&1 | grep -E "^test |result:" | sort | sha256sum
done
```

## Network surface

Crate dep tree contains no HTTP client. The only socket the binary opens
is the local Chromium CDP WebSocket via `chromiumoxide`.

## Repository

- URL: https://github.com/cochranblock/atsisbroken
- Visibility: PUBLIC
- Default branch: `master`
