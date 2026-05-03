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
$ atsisbroken init      # paste resume → Profile + seed model
$ atsisbroken run       # CDP attach + fill loop (training wheels)
$ atsisbroken graduate  # take the wheels off
$ atsisbroken status    # current config
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

For the exopack TRIPLE SIMS determinism gate, see [`docs/PROOF.md`](docs/PROOF.md).

## Docs

- [`docs/USER_STORIES.md`](docs/USER_STORIES.md)
- [`docs/TIMELINE.md`](docs/TIMELINE.md)
- [`docs/PROOF.md`](docs/PROOF.md)
- [`LICENSE-PROVENANCE.md`](LICENSE-PROVENANCE.md)
- [`UNLICENSE`](UNLICENSE)
