# atsisbroken

**ATS is broken.** A free, local-first, no-account, no-cloud alternative
to Simplify.us. Single Rust binary that drives Chromium via CDP and
fills job applications using a model **you train locally** from your
own resume.

By **The Cochran Block, LLC**. Public domain — `Unlicense`. Forever.

## Modes

- **Training wheels.** Every fill prompts you for yes/no. Each response
  trains the model online.
- **Chaos.** You graduate when ready. Autonomous fills. Post-hoc flagging
  still trains.

```
$ atsisbroken                       # default: launch the TUI
$ atsisbroken tui                   # same, explicit
$ atsisbroken init                  # paste resume → Profile + seed model
$ atsisbroken status                # current config / first-run hint
$ atsisbroken run                   # auto-pick best fill strategy for env
$ atsisbroken graduate              # TrainingWheels → Shadow → Chaos
$ atsisbroken sync                  # drain feedback queue if delivery configured
$ atsisbroken export --out FILE     # write feedback queue to disk
$ atsisbroken bridge                # Chrome Native Messaging host loop
$ atsisbroken install-bridge --extension-id ID
                                    # write Chrome Native Messaging manifest
```

The TUI is the primary interface — running with no subcommand opens a
three-tab terminal app (dashboard / queue / strategy) modeled on the
Claude Code aesthetic. Subcommands are for scripting and one-shot
tasks.

When CDP isn't available, `run` falls through a strategy ladder so the
product is usable in every environment:

```
$ atsisbroken userscript            # output a TamperMonkey userscript
$ atsisbroken bookmarklet           # output a javascript: bookmarklet
$ atsisbroken copy email            # copy a single field to the clipboard
$ atsisbroken speak                 # print every populated field
$ atsisbroken cdp-probe             # diagnose Chromium debug port
```

## Build

```sh
cargo build                                              # dev
cargo build --profile=diamond                            # speed-Diamond
cargo build --profile=diamond-edge --target=<triple>     # release
```

## Verify

```sh
cargo test
```

For the exopack TRIPLE SIMS determinism gate, see [`PROOF_OF_ARTIFACTS.md`](PROOF_OF_ARTIFACTS.md).

## Docs

Top-level (Cochran Block convention):
- [`README.md`](README.md)
- [`PLAN.md`](PLAN.md) — engineering + distribution + narrative + interested-parties
- [`USER_STORY_ANALYSIS.md`](USER_STORY_ANALYSIS.md)
- [`TIMELINE_OF_INVENTION.md`](TIMELINE_OF_INVENTION.md)
- [`PROOF_OF_ARTIFACTS.md`](PROOF_OF_ARTIFACTS.md)
- [`BACKLOG.md`](BACKLOG.md)
- [`CONTRIBUTORS.md`](CONTRIBUTORS.md)
- [`LICENSE-PROVENANCE.md`](LICENSE-PROVENANCE.md)
- [`UNLICENSE`](UNLICENSE)

Topic-specific (in `docs/`):
- [`docs/USER_FLOW.md`](docs/USER_FLOW.md) — install → first fill → graduation → chaos
- [`docs/UI_UX_ANALYSIS.md`](docs/UI_UX_ANALYSIS.md) — visual hierarchy, accessibility, gaps & recs
- [`docs/UI_UX_SIMULATION.md`](docs/UI_UX_SIMULATION.md) — persona × environment walkthroughs (P1–P7, E1–E6)
- [`docs/PLAN_BROWSER_AUTOMATION.md`](docs/PLAN_BROWSER_AUTOMATION.md) — TUI → launch browser → navigate → autofill, plus install-when-missing
- [`docs/TRAINING_DATA.md`](docs/TRAINING_DATA.md) — responsibly-sourced data tiers

Sub-product READMEs:
- [`extension/README.md`](extension/README.md) — Chrome extension + Native Messaging bridge
- [`android/README.md`](android/README.md) — Kotlin WebView shell + Rust JNI core
