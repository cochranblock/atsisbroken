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
| 1 | `e497e24316d0641ae196c944a4d200547d348ca4e5fe79a3e0552eb33cf52c01` |
| 2 | `e497e24316d0641ae196c944a4d200547d348ca4e5fe79a3e0552eb33cf52c01` |
| 3 | `e497e24316d0641ae196c944a4d200547d348ca4e5fe79a3e0552eb33cf52c01` |

(After `sed 's/finished in [0-9.]*s/finished/'` to strip cargo's
wall-clock timing from the e2e test result line — the live ATS test
takes 1.3±0.05 seconds and the variance is harmless.)

220/220 tests pass on every sim:
- 192 in `atsisbroken` lib (schemas, queue, bridge, paths, resume,
  strategy, cdp, tui, browser_detect; **+ predict_field_key with 18
  test cases including a regression for the "tel" inside
  "websiteLinkedIn" bug caught by the e2e test**)
-  11 in `atsisbroken` bin
-   1 in `tests/ats_e2e.rs` — **real chromium spawn against 3 mock
   ATS fixtures (Greenhouse / Lever / Workday), snapshot via
   `page.evaluate`, classify, fill, screenshot. Live test catches
   classifier bugs the unit tests miss.**
-   9 in `tests/cli_smoke.rs` integration
-   7 in `atsisbroken-android` lib
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
