// SPDX-License-Identifier: Unlicense
// Unlicense — public domain — cochranblock.org

//! atsisbroken-test — exopack TRIPLE SIMS gate.
//!
//! Run via `cargo run --features tests --bin atsisbroken-test`
//! (alias: aibte from the kova-aliases convention) when wired up.
//!
//! The gate runs `cargo test` 3 times against this crate. All
//! three must pass for the gate to exit 0. This is the same
//! exopack pattern kova-test, cochranblock-test, oakilydokily-test
//! use across the Cochran Block portfolio — single command,
//! deterministic outcome.

fn main() {
    let project = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));

    eprintln!("atsisbroken: TRIPLE SIMS gate (3× cargo test)");
    eprintln!("project: {}", project.display());

    let (ok, msg) = exopack::triple_sims::f61_with_args(
        project,
        3,
        // We test with the default features — that covers the
        // gui, tui, and core lib paths. Add --all-features here
        // when the engine + connectors land if they're behind
        // their own feature gates.
        &[],
    );

    if !ok {
        eprintln!("\n=== gate FAILED ===\n{msg}");
        std::process::exit(1);
    }

    println!("3 sims pass");
    println!("atsisbroken: TRIPLE SIMS gate green");
}
