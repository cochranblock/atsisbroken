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
| 1 | `13b5149b77370e17283e22e0c9d5cf2eb71b8024203d776d819c559fbe7a01cf` |
| 2 | `13b5149b77370e17283e22e0c9d5cf2eb71b8024203d776d819c559fbe7a01cf` |
| 3 | `13b5149b77370e17283e22e0c9d5cf2eb71b8024203d776d819c559fbe7a01cf` |

181/181 tests pass on every sim:
- 154 in `atsisbroken` lib (schemas, queue, bridge, paths, resume,
  strategy, cdp, **tui** — App state machine + key-action mapping +
  HTML snapshot renderer)
-  11 in `atsisbroken` bin (parse_mode, parse_strategy_override,
  clipboard wiring)
-   9 in `tests/cli_smoke.rs` integration (binary `--help`, `--version`,
  `init` ↔ `status`, `speak`, `bookmarklet`, `run` without init,
  `copy` unknown key, `cdp-probe` no Chrome)
-   7 in `atsisbroken-android` lib (JNI surface JSON shapes)
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
