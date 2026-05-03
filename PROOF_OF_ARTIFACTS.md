# atsisbroken — Proof of Artifacts

Verifiable evidence behind `TIMELINE_OF_INVENTION.md`. Every entry
reproduces from the named commit.

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
| 1 | `67747352a6cf9f9d3d115768c0d9ccbf05487a8db6520820eb7244047ea326a0` |
| 2 | `67747352a6cf9f9d3d115768c0d9ccbf05487a8db6520820eb7244047ea326a0` |
| 3 | `67747352a6cf9f9d3d115768c0d9ccbf05487a8db6520820eb7244047ea326a0` |

156/156 tests pass on every sim:
- 129 in `atsisbroken` lib (schemas, queue, bridge, paths, resume, strategy, cdp)
- 11 in `atsisbroken` bin (parse_mode, parse_strategy_override, clipboard wiring)
-  9 in `tests/cli_smoke.rs` integration (binary `--help`, `--version`,
   `init` ↔ `status`, `speak`, `bookmarklet`, `run` without init,
   `copy` unknown key, `cdp-probe` no Chrome)
-  7 in `atsisbroken-android` lib (JNI surface JSON shapes)
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
