// SPDX-License-Identifier: Unlicense
// Unlicense — public domain — cochranblock.org

//! In-binary test surface — the cochranblock exopack pattern.
//!
//! Tests live as plain `pub fn`s returning [`TestResult`] records,
//! NOT as `#[test]` functions. Reasons:
//!
//! - The whole module is gated behind `#[cfg(feature = "tests")]`
//!   in `lib.rs`. Production builds (no `tests` feature) don't
//!   compile any of this — the `atsisbroken` binary is the same
//!   code minus the test surface, by construction.
//!
//! - The `atsisbroken-test` bin (the only declared test binary)
//!   calls [`run_all`] directly instead of spawning `cargo test`.
//!   The test runner ships as a single distributable artifact;
//!   the host running it doesn't need cargo or the source tree.
//!
//! - Test output is captured into [`TestResult`] records so the
//!   TRIPLE SIMS gate can hash a deterministic projection of the
//!   results (skipping per-test elapsed_ms which varies by
//!   machine load) and assert byte-identical hashes across runs.
//!
//! Cargo-style `#[cfg(test)] mod tests {}` blocks are being
//! progressively migrated into this tree, one module per commit.
//! Until full migration, `cargo test` still runs the un-migrated
//! tests; once a module's tests live here, cargo test ignores it
//! (the block has been deleted).
//!
//! ## Layout
//!
//! ```text
//! src/tests/
//!   mod.rs          this file — TestResult, check, run, run_all
//!   connector.rs    pub fn run() -> Vec<TestResult>  (Phase 1)
//!   …               one file per crate module being covered
//! ```
//!
//! ## Hand-rolled equivalents we still need
//!
//! - `proptest` fuzzing for the URL parser (5 cases × 500 reps
//!   in `src/browser/url.rs`'s current `#[cfg(test)] mod tests`).
//!   Will land as a hand-rolled property runner in this tree.
//! - `insta` snapshot comparisons for the 6 internal-page fixtures
//!   in `src/browser/snapshots/`. Will land as a string-equality
//!   checker that loads the same `.snap` files (or a parallel
//!   atsisbroken-tests/ snapshot tree).
//!
//! Both stay cargo-test-side temporarily; conversion happens in
//! later phases.

#![allow(dead_code)]

use std::time::Instant;

pub mod bridge;
pub mod browser_detect;
pub mod cdp;
pub mod config;
pub mod connector;
pub mod engine;
pub mod fingerprint;
pub mod github;
pub mod input;
pub mod internal;
pub mod learning;
pub mod lib;
pub mod paths;
pub mod products;
pub mod resume;
pub mod run_loop;
pub mod shell;
pub mod strategy;
pub mod text;
#[cfg(feature = "tui")]
pub mod tui;
pub mod window;

/// Result of running one test. The TRIPLE SIMS gate hashes a
/// deterministic projection of `Vec<TestResult>` (name + passed +
/// error, NOT elapsed_ms) and compares across three runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub elapsed_ms: u64,
    /// Failure reason for FAIL outcomes; None on PASS. Captures
    /// either the explicit `Err(msg)` from a `?`-shaped test
    /// body, or "PANIC: <payload>" when the test panicked.
    pub error: Option<String>,
}

impl TestResult {
    /// Print one PASS/FAIL line to stderr in the cochranblock-style
    /// format. Used by the live runner; the gate hashes the
    /// deterministic-bytes projection separately.
    pub fn print(&self) {
        let tag = if self.passed { "PASS" } else { "FAIL" };
        match &self.error {
            Some(e) => eprintln!("{tag}  {}  {}ms  {e}", self.name, self.elapsed_ms),
            None => eprintln!("{tag}  {}  {}ms", self.name, self.elapsed_ms),
        }
    }

    /// Bytes the gate hashes. Excludes elapsed_ms (machine-load
    /// dependent) so the same test results produce the same hash
    /// across runs and across machines. Format:
    ///
    /// ```text
    /// <name>\tP\n
    /// <name>\tF\t<error>\n
    /// ```
    pub fn deterministic_bytes(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self.name.as_bytes());
        out.push(b'\t');
        out.push(if self.passed { b'P' } else { b'F' });
        if let Some(e) = &self.error {
            out.push(b'\t');
            out.extend_from_slice(e.as_bytes());
        }
        out.push(b'\n');
    }
}

/// Assertion primitive for tests in this tree. Returns `Ok(())` on
/// pass; `Err(msg)` on fail. Use with `?`:
///
/// ```ignore
/// fn my_test() -> Result<(), String> {
///     let url: Url = "https://example.com".parse().map_err(|e| format!("{e}"))?;
///     check(url.is_network(), "expected network URL")?;
///     check(url.host() == "example.com", "host wrong")?;
///     Ok(())
/// }
///
/// pub fn run() -> Vec<TestResult> {
///     vec![case("module::my_test", my_test)]
/// }
/// ```
pub fn check(condition: bool, msg: impl Into<String>) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(msg.into())
    }
}

/// Equivalent of `assert_eq!` for this tree. Returns Err with a
/// formatted "left != right" message on mismatch. Both sides must
/// be `PartialEq + Debug`.
pub fn check_eq<T: PartialEq + std::fmt::Debug>(
    left: T,
    right: T,
    label: &str,
) -> Result<(), String> {
    if left == right {
        Ok(())
    } else {
        Err(format!("{label}: {left:?} != {right:?}"))
    }
}

/// Hand-rolled equivalent of `insta::assert_snapshot!`. Reads the
/// fixture file at the documented path and compares the raw body
/// (everything after the second `---` line of the YAML header) to
/// `actual` — string-equality, no fancy diffing.
///
/// Format of the .snap file (matches `insta`'s text format):
///
/// ```text
/// ---
/// source: <path>
/// expression: <expr>
/// ---
/// <body>
/// ```
///
/// `name` is the bare snapshot name (e.g. `"internal_page_home"`);
/// the file path is constructed as
/// `<crate_root>/<dir>/atsisbroken__browser__engine__tests__<name>.snap`.
/// `dir` lets a converted module point at its own snapshots
/// directory; today only `src/browser/snapshots` exists.
pub fn check_snapshot(dir: &str, name: &str, actual: &str) -> Result<(), String> {
    let crate_root = env!("CARGO_MANIFEST_DIR");
    let path = std::path::PathBuf::from(crate_root)
        .join(dir)
        .join(format!("atsisbroken__browser__engine__tests__{name}.snap"));
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("read {}: {e}", path.display()))?;
    let body = strip_insta_header(&raw)
        .ok_or_else(|| format!("malformed snapshot header in {}", path.display()))?;
    if body == actual {
        return Ok(());
    }
    let (line_no, exp_line, got_line) = first_diff_line(body, actual);
    Err(format!(
        "snapshot {name} differs at line {line_no}\n  expected: {exp_line:?}\n  got:      {got_line:?}\n  fixture:  {}",
        path.display()
    ))
}

/// Strip insta's YAML header (`---\n…\n---\n`) and return the body.
/// Strips the trailing newline insta appends so `actual` (which
/// typically doesn't end with `\n`) compares cleanly.
/// Returns None if the header isn't well-formed.
fn strip_insta_header(raw: &str) -> Option<&str> {
    if !raw.starts_with("---\n") {
        return None;
    }
    let after_first = &raw[4..];
    let second = after_first.find("\n---\n")?;
    let body_start = second + 5; // skip "\n---\n"
    let body = &after_first[body_start..];
    // insta writes a trailing `\n` to the file; the test's `actual`
    // string typically doesn't carry that. Strip exactly one.
    Some(body.strip_suffix('\n').unwrap_or(body))
}

/// Return (line_no, expected_line, got_line) for the first differing
/// line between `expected` and `got`. line_no is 1-indexed.
fn first_diff_line<'a>(expected: &'a str, got: &'a str) -> (usize, &'a str, &'a str) {
    let mut e = expected.lines();
    let mut g = got.lines();
    let mut n = 1;
    loop {
        match (e.next(), g.next()) {
            (Some(a), Some(b)) if a == b => {
                n += 1;
                continue;
            }
            (Some(a), Some(b)) => return (n, a, b),
            (Some(a), None) => return (n, a, ""),
            (None, Some(b)) => return (n, "", b),
            (None, None) => return (n, "", ""),
        }
    }
}

/// Run one test by name. Catches panics so a single panicking
/// test reports as FAIL and the rest of the suite continues.
/// Times the body and returns a [`TestResult`].
///
/// Named `case` (not `run`) so it doesn't collide with each
/// module's `pub fn run() -> Vec<TestResult>` dispatcher when
/// the module imports both via `use super::{check, case, …}`.
pub fn case(name: &str, body: impl FnOnce() -> Result<(), String>) -> TestResult {
    let start = Instant::now();
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(body));
    let elapsed_ms = start.elapsed().as_millis() as u64;
    match outcome {
        Ok(Ok(())) => TestResult {
            name: name.into(),
            passed: true,
            elapsed_ms,
            error: None,
        },
        Ok(Err(e)) => TestResult {
            name: name.into(),
            passed: false,
            elapsed_ms,
            error: Some(e),
        },
        Err(panic_payload) => {
            let msg = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                (*s).to_string()
            } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                s.clone()
            } else {
                "<panic with non-string payload>".to_string()
            };
            TestResult {
                name: name.into(),
                passed: false,
                elapsed_ms,
                error: Some(format!("PANIC: {msg}")),
            }
        }
    }
}

/// Run every converted module's tests in source-file order.
/// Returns the full result vector; the caller decides what to do
/// with it (print, hash, exit).
pub fn run_all() -> Vec<TestResult> {
    let mut all = Vec::new();
    all.extend(bridge::run());
    all.extend(browser_detect::run());
    all.extend(cdp::run());
    all.extend(config::run());
    all.extend(connector::run());
    all.extend(engine::run());
    all.extend(fingerprint::run());
    all.extend(github::run());
    all.extend(input::run());
    all.extend(internal::run());
    all.extend(learning::run());
    all.extend(lib::run());
    all.extend(paths::run());
    all.extend(products::run());
    all.extend(resume::run());
    all.extend(run_loop::run());
    all.extend(shell::run());
    all.extend(strategy::run());
    all.extend(text::run());
    #[cfg(feature = "tui")]
    all.extend(tui::run());
    all.extend(window::run());
    all
}

/// Convenience for the `atsisbroken-test` bin: run, print every
/// result, return whether all passed.
pub fn run_all_and_print() -> (Vec<TestResult>, bool) {
    let results = run_all();
    for r in &results {
        r.print();
    }
    let total = results.len();
    let passed = results.iter().filter(|r| r.passed).count();
    eprintln!("\n{passed}/{total} tests passed");
    (results, passed == total)
}
