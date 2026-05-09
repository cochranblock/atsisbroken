// SPDX-License-Identifier: Unlicense
// Unlicense — public domain — cochranblock.org

//! atsisbroken-test — TRIPLE SIMS gate with byte-identical hash.
//!
//! Run via `cargo run --features tests --bin atsisbroken-test`
//! (alias: aibte from the kova-aliases convention).
//!
//! What it does:
//! 1. Run `cargo test` three times in sequence.
//! 2. Each run's stdout gets normalized (test-result lines sorted
//!    alphabetically, timing-bearing lines stripped). Test-output
//!    line order is non-deterministic because cargo runs tests in
//!    parallel; sorting recovers determinism. "finished in 0.17s"
//!    varies per run; stripping handles it.
//! 3. blake3-hash each normalized output.
//! 4. Verify all three hashes are equal — if so, print the hash.
//!    Different hashes mean the test suite is non-deterministic
//!    (a bug worth fixing).
//!
//! Output on success:
//!   atsisbroken: TRIPLE SIMS gate (3× cargo test + blake3 hash)
//!   project: /home/mcochran/atsisbroken
//!   sim 1/3 ok (12345 bytes normalized)  hash=abc123…def456
//!   sim 2/3 ok (12345 bytes normalized)  hash=abc123…def456
//!   sim 3/3 ok (12345 bytes normalized)  hash=abc123…def456
//!   3 sims byte-identical: abc123…def456
//!   atsisbroken: TRIPLE SIMS gate green
//!
//! The byte-identical hash is the same shape kova / cochranblock
//! gates produced in prior commits (e.g. `c1d6d90eb930…`). It
//! goes into commit messages as proof the test suite is
//! deterministic at this revision.
//!
//! exopack's `triple_sims::f61_with_args` would also run the
//! 3-pass check, but it doesn't produce a hash. We do both jobs
//! ourselves so the hash claim is real and verifiable.

use std::path::Path;
use std::process::Command;

fn main() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR"));

    eprintln!("atsisbroken: TRIPLE SIMS gate (3× cargo test + blake3 hash)");
    eprintln!("project:     {}", project.display());

    let mut hashes: Vec<blake3::Hash> = Vec::new();

    for i in 1..=3u32 {
        eprintln!("sim {i}/3 …");
        let out = Command::new("cargo")
            .args(["test", "--features", "gui"])
            .current_dir(project)
            .env("CARGO_TERM_COLOR", "never")
            .output()
            .expect("spawn cargo test");
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let stdout = String::from_utf8_lossy(&out.stdout);
            eprintln!("\n=== sim {i} FAILED ===");
            eprintln!("{stdout}");
            eprintln!("{stderr}");
            std::process::exit(1);
        }
        let normalized = normalize_test_output(&out.stdout);
        let hash = blake3::hash(&normalized);
        eprintln!(
            "  ok ({} bytes normalized) hash={}",
            normalized.len(),
            hash.to_hex()
        );
        hashes.push(hash);
    }

    let first = hashes[0];
    if hashes.iter().all(|h| *h == first) {
        println!("3 sims byte-identical: {}", first.to_hex());
        println!("atsisbroken: TRIPLE SIMS gate green");
    } else {
        eprintln!("\n=== non-deterministic ===");
        for (i, h) in hashes.iter().enumerate() {
            eprintln!("  sim {}: {}", i + 1, h.to_hex());
        }
        eprintln!(
            "Test output differs across runs after normalization. \
             Likely a test that depends on time / pid / random state \
             without a fixed seed. Bisect by stripping suspect tests \
             and re-running until hashes match."
        );
        std::process::exit(1);
    }
}

/// Make `cargo test` output reproducible by:
/// - Sorting per-test result lines alphabetically (parallel
///   execution emits them in non-deterministic order).
/// - Removing timing-bearing lines ("finished in X.XXs",
///   "Finished `dev` profile … in Y.YYs").
/// - Removing blank lines.
/// - Removing "Compiling …" and "Running …" build chatter that
///   varies with rebuild cache state.
///
/// What stays: every "test foo ... ok / FAILED" line (sorted),
/// every "test result: ok. N passed; M failed; …" summary.
fn normalize_test_output(stdout: &[u8]) -> Vec<u8> {
    let s = String::from_utf8_lossy(stdout);
    let mut test_lines: Vec<String> = Vec::new();
    let mut summary_and_other: Vec<String> = Vec::new();

    for raw_line in s.lines() {
        let line = raw_line.trim_end();
        if line.is_empty() {
            continue;
        }
        // Per-test result line: starts with "test " and has " ... ".
        // Excluded: "test result:" summary lines.
        if line.starts_with("test ") && line.contains(" ... ") && !line.starts_with("test result:")
        {
            test_lines.push(line.to_string());
            continue;
        }
        // Summary lines and proptest output keep their order.
        // Timing-bearing chatter we strip.
        if line.contains("finished in ") || line.contains("Finished ") {
            // strip the timing part; keep the structural part only
            // for "test result:" summaries (they have a real line
            // anyway). For "Finished `dev` profile …" we drop
            // entirely — purely build-time chatter.
            if line.starts_with("test result:") {
                let no_timing = line
                    .split("; finished in")
                    .next()
                    .unwrap_or(line)
                    .to_string();
                summary_and_other.push(no_timing);
            }
            continue;
        }
        // Build chatter that varies by compile cache state.
        if line.starts_with("   Compiling ")
            || line.starts_with("    Finished ")
            || line.starts_with("     Running ")
            || line.starts_with("    Updating ")
            || line.starts_with("     Locking ")
            || line.starts_with("warning: ")
            || line.starts_with("note: ")
            || line.starts_with("help: ")
            || line.starts_with("error:")
        {
            // Compile/warn chatter — exclude. Real test failures
            // would have caused a non-zero exit before we reach
            // normalization, so dropping these here is safe.
            continue;
        }
        summary_and_other.push(line.to_string());
    }

    test_lines.sort();
    let mut out = String::new();
    for l in &test_lines {
        out.push_str(l);
        out.push('\n');
    }
    for l in &summary_and_other {
        out.push_str(l);
        out.push('\n');
    }
    out.into_bytes()
}
