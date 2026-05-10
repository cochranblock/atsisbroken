// SPDX-License-Identifier: Unlicense

//! In-process CLI smoke tests — Phase 11 migration of
//! `tests/cli_smoke.rs`. Drives [`crate::cli::run`] directly with
//! a tempdir home + `Vec<u8>` stdout/stderr writers + an `&[u8]`
//! stdin. No subprocess. No CARGO_BIN_EXE. The test binary IS
//! the test runner and the test harness — same code as the
//! production binary minus the `main()` entry shell.

use std::path::PathBuf;

use crate::cli::{run, Cli, CliCtx};
use clap::Parser;

use super::{case, check, check_eq, TestResult};

pub fn run_tests() -> Vec<TestResult> {
    vec![
        case("cli_smoke::help_flag_prints_subcommands", help_flag_prints_subcommands),
        case("cli_smoke::version_flag_prints_crate_version", version_flag_prints_crate_version),
        case("cli_smoke::status_before_init_says_not_initialized",
             status_before_init_says_not_initialized),
        case("cli_smoke::init_then_status_reports_initialized_yes",
             init_then_status_reports_initialized_yes),
        case("cli_smoke::speak_after_init_prints_every_field", speak_after_init_prints_every_field),
        case("cli_smoke::bookmarklet_after_init_is_javascript_url",
             bookmarklet_after_init_is_javascript_url),
        case("cli_smoke::run_without_init_errors_with_helpful_hint",
             run_without_init_errors_with_helpful_hint),
        case("cli_smoke::copy_with_unknown_key_errors", copy_with_unknown_key_errors),
        case("cli_smoke::cdp_probe_without_running_chromium_exits_cleanly",
             cdp_probe_without_running_chromium_exits_cleanly),
        case("cli_smoke::inspect_internal_home_renders", inspect_internal_home_renders),
        case("cli_smoke::inspect_connections_lists_services", inspect_connections_lists_services),
        case("cli_smoke::inspect_unknown_returns_not_found", inspect_unknown_returns_not_found),
        case("cli_smoke::inspect_invalid_url_errors", inspect_invalid_url_errors),
        case("cli_smoke::inspect_fingerprint_documents_webdriver_false",
             inspect_fingerprint_documents_webdriver_false),
        case("cli_smoke::inspect_summary_directs_to_connections", inspect_summary_directs_to_connections),
    ]
}

// ─── helpers ─────────────────────────────────────────────────────────

fn tmp_home(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "atsisbroken_cli_smoke_{label}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&dir).expect("mkdir tmp_home");
    dir
}

/// Build a Cli + CliCtx and call run. Returns
/// (result, stdout_bytes, stderr_bytes). Each test owns its own
/// tempdir home and gets a fresh pair of writers.
fn run_args(
    home: &std::path::Path,
    stdin: &[u8],
    args: &[&str],
) -> (anyhow::Result<()>, Vec<u8>, Vec<u8>) {
    // clap expects argv[0] to be the binary name. Prepend it.
    let mut argv: Vec<String> = vec!["atsisbroken".to_string()];
    argv.extend(args.iter().map(|s| s.to_string()));
    let cli = match Cli::try_parse_from(&argv) {
        Ok(c) => c,
        Err(e) => {
            // For DisplayHelp / DisplayVersion, clap returns Err with
            // the rendered text in `e`. The two tests that exercise
            // those paths use try_parse_from directly; this helper is
            // for the run-the-command path where parse failure means
            // the test setup is wrong.
            return (Err(anyhow::anyhow!("clap parse: {e}")), Vec::new(), Vec::new());
        }
    };
    let mut stdout = Vec::<u8>::new();
    let mut stderr = Vec::<u8>::new();
    let mut ctx = CliCtx::new(home.to_path_buf(), stdin, &mut stdout, &mut stderr);
    let result = run(cli, &mut ctx);
    (result, stdout, stderr)
}

// ─── tests ───────────────────────────────────────────────────────────

fn help_flag_prints_subcommands() -> Result<(), String> {
    // clap's --help returns Err(ErrorKind::DisplayHelp) with the
    // rendered help text. That's what `--help` exits 0 with on the
    // command line — clap prints the message and process_exit(0)s
    // when called from main; in-process we read the rendered text.
    let err = Cli::try_parse_from(["atsisbroken", "--help"]).err()
        .ok_or_else(|| "expected --help to return Err".to_string())?;
    check_eq(err.kind(), clap::error::ErrorKind::DisplayHelp, "help kind")?;
    let rendered = err.render().to_string();
    for sub in [
        "init",
        "run",
        "graduate",
        "sync",
        "export",
        "bridge",
        "status",
        "userscript",
        "bookmarklet",
        "copy",
        "speak",
        "cdp-probe",
        "install-bridge",
    ] {
        check(
            rendered.contains(sub),
            format!("help missing subcommand {sub}"),
        )?;
    }
    Ok(())
}

fn version_flag_prints_crate_version() -> Result<(), String> {
    let err = Cli::try_parse_from(["atsisbroken", "--version"])
        .err()
        .ok_or_else(|| "expected --version to return Err".to_string())?;
    check_eq(err.kind(), clap::error::ErrorKind::DisplayVersion, "version kind")?;
    let rendered = err.render().to_string();
    check(rendered.contains("0.0.0"), format!("version output: {rendered}"))
}

fn status_before_init_says_not_initialized() -> Result<(), String> {
    let home = tmp_home("status_before");
    let (result, stdout, _stderr) = run_args(&home, b"", &["status"]);
    result.map_err(|e| format!("{e}"))?;
    let s = String::from_utf8_lossy(&stdout);
    check(
        s.contains("initialized: no"),
        format!("status before init must say 'no': {s}"),
    )?;
    let _ = std::fs::remove_dir_all(&home);
    Ok(())
}

fn init_then_status_reports_initialized_yes() -> Result<(), String> {
    let home = tmp_home("init_then_status");
    let resume = b"Jane Q. Doe\njane@example.com\n+1-555-010-2030\n";
    let (init_result, _stdout, _stderr) = run_args(&home, resume, &["init"]);
    init_result.map_err(|e| format!("init: {e}"))?;

    let profile_path = home.join(".atsisbroken/profile.toml");
    check(profile_path.exists(), "profile.toml not written")?;
    let profile_text = std::fs::read_to_string(&profile_path).map_err(|e| format!("{e}"))?;
    check(
        profile_text.contains("jane@example.com"),
        "profile.toml missing email",
    )?;

    let (status_result, stdout, _stderr) = run_args(&home, b"", &["status"]);
    status_result.map_err(|e| format!("status: {e}"))?;
    let s = String::from_utf8_lossy(&stdout);
    check(s.contains("initialized: yes"), format!("status: {s}"))?;
    check(s.contains("feedback queue: 0 events"), format!("status: {s}"))?;
    let _ = std::fs::remove_dir_all(&home);
    Ok(())
}

fn speak_after_init_prints_every_field() -> Result<(), String> {
    let home = tmp_home("speak_after_init");
    let resume = b"Jane Q. Doe\njane@example.com\n+1-555-010-2030\nlinkedin.com/in/janedoe\ngithub.com/janedoe\n";
    let (init_result, _, _) = run_args(&home, resume, &["init"]);
    init_result.map_err(|e| format!("init: {e}"))?;

    let (speak_result, stdout, _) = run_args(&home, b"", &["speak"]);
    speak_result.map_err(|e| format!("speak: {e}"))?;
    let s = String::from_utf8_lossy(&stdout);
    check(s.contains("full_name: Jane Q. Doe"), format!("missing full_name: {s}"))?;
    check(s.contains("email: jane@example.com"), format!("missing email: {s}"))?;
    check(s.contains("phone:"), format!("missing phone: {s}"))?;
    check(
        s.contains("linkedin: linkedin.com/in/janedoe"),
        format!("missing linkedin: {s}"),
    )?;
    check(
        s.contains("github: github.com/janedoe"),
        format!("missing github: {s}"),
    )?;
    let _ = std::fs::remove_dir_all(&home);
    Ok(())
}

fn bookmarklet_after_init_is_javascript_url() -> Result<(), String> {
    let home = tmp_home("bookmarklet_after_init");
    let resume = b"Jane Q. Doe\njane@example.com\n";
    let (init_result, _, _) = run_args(&home, resume, &["init"]);
    init_result.map_err(|e| format!("init: {e}"))?;

    let (bm_result, stdout, _) = run_args(&home, b"", &["bookmarklet"]);
    bm_result.map_err(|e| format!("bookmarklet: {e}"))?;
    let s = String::from_utf8_lossy(&stdout);
    check(s.starts_with("javascript:"), format!("bookmarklet: {s}"))?;
    check(s.contains("jane@example.com"), format!("missing email: {s}"))?;
    let _ = std::fs::remove_dir_all(&home);
    Ok(())
}

fn run_without_init_errors_with_helpful_hint() -> Result<(), String> {
    let home = tmp_home("run_without_init");
    let (result, _stdout, _stderr) = run_args(&home, b"", &["run"]);
    let err = result.err().ok_or_else(|| "expected run to fail".to_string())?;
    let msg = format!("{err:#}");
    check(
        msg.contains("atsisbroken init"),
        format!("error must hint at init: {msg}"),
    )?;
    let _ = std::fs::remove_dir_all(&home);
    Ok(())
}

fn copy_with_unknown_key_errors() -> Result<(), String> {
    let home = tmp_home("copy_unknown");
    let resume = b"Jane Doe\njane@example.com\n";
    let (init_result, _, _) = run_args(&home, resume, &["init"]);
    init_result.map_err(|e| format!("init: {e}"))?;

    let (result, _, _) = run_args(&home, b"", &["copy", "not_a_real_key"]);
    let err = result.err().ok_or_else(|| "expected copy unknown to fail".to_string())?;
    let msg = format!("{err:#}");
    check(msg.contains("unknown"), format!("error msg: {msg}"))?;
    let _ = std::fs::remove_dir_all(&home);
    Ok(())
}

fn cdp_probe_without_running_chromium_exits_cleanly() -> Result<(), String> {
    let home = tmp_home("cdp_probe_no_chrome");
    // Force the probe to look at port 1 (privileged, no daemon) so
    // the test result doesn't depend on whether the host happens to
    // have Chrome on 9222. set_var is unsafe in Rust 2024 because
    // it's racy under multi-threaded access; the gate runs all
    // tests on a single thread, and the var is set/read/cleared
    // entirely within this test. SAFE under those conditions.
    unsafe {
        std::env::set_var("CHROME_DEBUG_PORT", "1");
    }
    let (result, _stdout, _stderr) = run_args(&home, b"", &["cdp-probe"]);
    unsafe {
        std::env::remove_var("CHROME_DEBUG_PORT");
    }
    // The probe is "diagnostic" — exits 0 either way, with a message.
    result.map_err(|e| format!("cdp-probe: {e}"))?;
    let _ = std::fs::remove_dir_all(&home);
    Ok(())
}

// ─── inspect (gui-feature only — gated in run_all dispatch) ─────────

fn inspect_internal_home_renders() -> Result<(), String> {
    let home = tmp_home("inspect_home");
    let (result, stdout, _) = run_args(&home, b"", &["inspect", "--url", "atsisbroken://home"]);
    result.map_err(|e| format!("inspect: {e}"))?;
    let s = String::from_utf8_lossy(&stdout);
    check(s.contains("URL: atsisbroken://home"), format!("stdout: {s}"))?;
    check(s.contains("Title: atsisbroken"), format!("stdout: {s}"))?;
    check(s.contains("Fields: 0"), format!("stdout: {s}"))?;
    check(s.contains("atsisbroken://connections"), format!("stdout: {s}"))?;
    let _ = std::fs::remove_dir_all(&home);
    Ok(())
}

fn inspect_connections_lists_services() -> Result<(), String> {
    let home = tmp_home("inspect_conn");
    let (result, stdout, _) = run_args(&home, b"", &["inspect", "--url", "atsisbroken://connections"]);
    result.map_err(|e| format!("inspect: {e}"))?;
    let s = String::from_utf8_lossy(&stdout);
    check(s.contains("Title: Your connections"), format!("stdout: {s}"))?;
    for service in ["GitHub", "Stack Overflow", "Hacker News", "crates.io"] {
        check(s.contains(service), format!("missing {service}: {s}"))?;
    }
    check(s.contains("LinkedIn"), format!("missing LinkedIn: {s}"))?;
    check(s.contains("Substack"), format!("missing Substack: {s}"))?;
    check(
        s.contains("atsisbroken://connect/github"),
        format!("missing connect/github: {s}"),
    )?;
    let _ = std::fs::remove_dir_all(&home);
    Ok(())
}

fn inspect_unknown_returns_not_found() -> Result<(), String> {
    let home = tmp_home("inspect_404");
    let (result, stdout, _) = run_args(&home, b"", &["inspect", "--url", "atsisbroken://nope"]);
    result.map_err(|e| format!("inspect: {e}"))?;
    let s = String::from_utf8_lossy(&stdout);
    check(s.contains("Title: Page not found"), format!("stdout: {s}"))?;
    check(s.contains("atsisbroken://nope"), format!("stdout: {s}"))?;
    let _ = std::fs::remove_dir_all(&home);
    Ok(())
}

fn inspect_invalid_url_errors() -> Result<(), String> {
    let home = tmp_home("inspect_invalid");
    let (result, _, _) = run_args(&home, b"", &["inspect", "--url", ""]);
    let err = result.err().ok_or_else(|| "expected empty URL to error".to_string())?;
    let msg = format!("{err:#}");
    check(msg.contains("invalid"), format!("error msg: {msg}"))?;
    let _ = std::fs::remove_dir_all(&home);
    Ok(())
}

fn inspect_fingerprint_documents_webdriver_false() -> Result<(), String> {
    let home = tmp_home("inspect_fp");
    let (result, stdout, _) = run_args(&home, b"", &["inspect", "--url", "atsisbroken://fingerprint"]);
    result.map_err(|e| format!("inspect: {e}"))?;
    let s = String::from_utf8_lossy(&stdout);
    check(s.contains("navigator.webdriver"), format!("stdout: {s}"))?;
    check(s.contains("false"), format!("stdout: {s}"))?;
    let _ = std::fs::remove_dir_all(&home);
    Ok(())
}

fn inspect_summary_directs_to_connections() -> Result<(), String> {
    let home = tmp_home("inspect_summary");
    let (result, stdout, _) = run_args(&home, b"", &["inspect", "--url", "atsisbroken://summary"]);
    result.map_err(|e| format!("inspect: {e}"))?;
    let s = String::from_utf8_lossy(&stdout);
    check(
        s.contains("atsisbroken://connections"),
        format!("stdout: {s}"),
    )?;
    let _ = std::fs::remove_dir_all(&home);
    Ok(())
}
