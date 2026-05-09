// SPDX-License-Identifier: Unlicense
// Unlicense — public domain — cochranblock.org

//! atsisbroken-test — TRIPLE SIMS gate, in-binary edition.
//!
//! Run via `cargo run --features tests --bin atsisbroken-test`
//! (alias: aibte from the kova-aliases convention).
//!
//! What it does:
//! 1. Calls [`atsisbroken::tests::run_all_and_print`] three times.
//! 2. Each run hashes a deterministic projection of the result
//!    vector (test name + PASS/FAIL + error, NOT elapsed_ms which
//!    varies per machine load) with blake3.
//! 3. Verifies all three hashes are equal — if so, prints the hash.
//!    Different hashes mean the test suite is non-deterministic
//!    (a bug worth fixing).
//!
//! ## Why this binary, not `cargo test`?
//!
//! The cochranblock pattern: tests live as `pub fn`s gated behind
//! `#[cfg(feature = "tests")]`, called directly by this binary
//! rather than through cargo's `#[test]` harness. Reasons:
//!
//! - The test binary ships as a single distributable artifact.
//!   The host running the gate doesn't need cargo or the source
//!   tree.
//! - Production builds (no `tests` feature) don't compile any of
//!   the test surface — the production `atsisbroken` binary IS
//!   the same code minus the tests, by construction.
//! - Result aggregation is in our hands, so the TRIPLE SIMS
//!   determinism check operates on a stable byte projection
//!   chosen by us (not on cargo test's stdout, which mixes test
//!   output with build chatter and timing).
//!
//! ## Migration status
//!
//! The lib's `tests` module covers a growing slice of the
//! crate's tests, one module per phase. Modules not yet
//! converted still have their `#[cfg(test)] mod tests {}` blocks
//! and run under `cargo test` separately. This binary's gate
//! covers ONLY the converted tests. Once migration finishes,
//! `cargo test` becomes a no-op and this binary is the sole
//! source of test signal.

#[cfg(feature = "tests")]
fn main() {
    use atsisbroken::tests;

    eprintln!("atsisbroken: TRIPLE SIMS gate (3× in-binary tests + blake3 hash)");

    let mut hashes: Vec<blake3::Hash> = Vec::new();
    let mut last_results: Option<Vec<tests::TestResult>> = None;

    for i in 1..=3u32 {
        eprintln!("\nsim {i}/3 …");
        let (results, all_passed) = tests::run_all_and_print();
        if !all_passed {
            let failures: Vec<&tests::TestResult> =
                results.iter().filter(|r| !r.passed).collect();
            eprintln!("\n=== sim {i} FAILED — {} failure(s) ===", failures.len());
            for f in failures {
                eprintln!("  {}: {}", f.name, f.error.as_deref().unwrap_or("(no reason)"));
            }
            std::process::exit(1);
        }
        // Deterministic projection: test results minus elapsed_ms.
        // Same byte sequence across runs → same blake3 hash.
        let mut bytes = Vec::new();
        for r in &results {
            r.deterministic_bytes(&mut bytes);
        }
        let hash = blake3::hash(&bytes);
        eprintln!(
            "  ok ({} tests, {} bytes) hash={}",
            results.len(),
            bytes.len(),
            hash.to_hex()
        );
        hashes.push(hash);
        last_results = Some(results);
    }

    let first = hashes[0];
    if hashes.iter().all(|h| *h == first) {
        let total = last_results.map(|r| r.len()).unwrap_or(0);
        println!(
            "\n3 sims byte-identical: {}\n{total} tests passed",
            first.to_hex()
        );
        println!("atsisbroken: TRIPLE SIMS gate green");
    } else {
        eprintln!("\n=== non-deterministic ===");
        for (i, h) in hashes.iter().enumerate() {
            eprintln!("  sim {}: {}", i + 1, h.to_hex());
        }
        eprintln!(
            "Test results differ across runs after determinism \
             projection. Likely a test reading time, pid, env, or \
             random state without a fixed seed. Bisect by toggling \
             tests off in src/tests/<module>.rs::run() until hashes \
             match."
        );
        std::process::exit(1);
    }
}

#[cfg(not(feature = "tests"))]
fn main() {
    eprintln!(
        "atsisbroken-test was built without the `tests` feature. \
         Run via `cargo run --features tests --bin atsisbroken-test`."
    );
    std::process::exit(2);
}
