// SPDX-License-Identifier: Unlicense
//! End-to-end CLI smoke tests. These run the actual built binary against
//! a temporary HOME so the user's real ~/.atsisbroken/ stays untouched.
//!
//! Catches regressions in: clap subcommand wiring, init→profile.toml
//! flow, status's first-run detection, export/sync queue path.

use std::io::Write;
use std::process::{Command, Stdio};

fn bin_path() -> std::path::PathBuf {
    // Cargo sets CARGO_BIN_EXE_<name> for binaries when running tests.
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_atsisbroken"))
}

fn tmp_home(label: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "atsisbroken_smoke_{label}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn help_flag_prints_subcommands_and_exits_zero() {
    let out = Command::new(bin_path())
        .arg("--help")
        .output()
        .expect("run binary");
    assert!(out.status.success(), "--help must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    for sub in ["init", "run", "graduate", "sync", "export", "bridge", "status",
                "userscript", "bookmarklet", "copy", "speak", "cdp-probe",
                "install-bridge"] {
        assert!(
            stdout.contains(sub),
            "--help missing subcommand {sub}: {stdout}"
        );
    }
}

#[test]
fn version_flag_prints_crate_version() {
    let out = Command::new(bin_path())
        .arg("--version")
        .output()
        .expect("run binary");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Cargo.toml version is 0.0.0 right now.
    assert!(stdout.contains("0.0.0"), "version output: {stdout}");
}

#[test]
fn status_before_init_says_not_initialized() {
    let home = tmp_home("status_before");
    let out = Command::new(bin_path())
        .arg("status")
        .env("HOME", &home)
        .output()
        .expect("run status");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("initialized: no"),
        "status before init must say 'no': {stdout}"
    );
    let _ = std::fs::remove_dir_all(home);
}

#[test]
fn init_then_status_reports_initialized_yes() {
    let home = tmp_home("init_then_status");
    let resume = "Jane Q. Doe\njane@example.com\n+1-555-010-2030\n";
    let mut child = Command::new(bin_path())
        .arg("init")
        .env("HOME", &home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn init");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(resume.as_bytes())
        .unwrap();
    let out = child.wait_with_output().expect("wait init");
    assert!(out.status.success(), "init failed: {:?}", out);
    let profile_path = home.join(".atsisbroken/profile.toml");
    assert!(profile_path.exists(), "profile.toml not written");
    let profile_text = std::fs::read_to_string(&profile_path).unwrap();
    assert!(profile_text.contains("jane@example.com"));
    let status = Command::new(bin_path())
        .arg("status")
        .env("HOME", &home)
        .output()
        .expect("run status");
    let stdout = String::from_utf8_lossy(&status.stdout);
    assert!(stdout.contains("initialized: yes"), "status: {stdout}");
    assert!(stdout.contains("feedback queue: 0 events"));
    let _ = std::fs::remove_dir_all(home);
}

#[test]
fn speak_after_init_prints_every_field_user_provided() {
    let home = tmp_home("speak_after_init");
    let resume = "Jane Q. Doe\njane@example.com\n+1-555-010-2030\nlinkedin.com/in/janedoe\ngithub.com/janedoe\n";
    let mut child = Command::new(bin_path())
        .arg("init")
        .env("HOME", &home)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    child.stdin.as_mut().unwrap().write_all(resume.as_bytes()).unwrap();
    child.wait().unwrap();

    let out = Command::new(bin_path())
        .arg("speak")
        .env("HOME", &home)
        .output()
        .expect("run speak");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("full_name: Jane Q. Doe"));
    assert!(stdout.contains("email: jane@example.com"));
    assert!(stdout.contains("phone:"));
    assert!(stdout.contains("linkedin: linkedin.com/in/janedoe"));
    assert!(stdout.contains("github: github.com/janedoe"));
    let _ = std::fs::remove_dir_all(home);
}

#[test]
fn bookmarklet_after_init_is_javascript_url() {
    let home = tmp_home("bookmarklet_after_init");
    let resume = "Jane Q. Doe\njane@example.com\n";
    let mut child = Command::new(bin_path())
        .arg("init")
        .env("HOME", &home)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    child.stdin.as_mut().unwrap().write_all(resume.as_bytes()).unwrap();
    child.wait().unwrap();

    let out = Command::new(bin_path())
        .arg("bookmarklet")
        .env("HOME", &home)
        .output()
        .expect("run bookmarklet");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.starts_with("javascript:"), "bookmarklet: {stdout}");
    assert!(stdout.contains("jane@example.com"));
    let _ = std::fs::remove_dir_all(home);
}

#[test]
fn run_without_init_errors_with_helpful_hint() {
    let home = tmp_home("run_without_init");
    let out = Command::new(bin_path())
        .arg("run")
        .env("HOME", &home)
        .output()
        .expect("run before init");
    assert!(!out.status.success(), "run without init should fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("atsisbroken init"),
        "error must hint at init: {stderr}"
    );
    let _ = std::fs::remove_dir_all(home);
}

#[test]
fn copy_with_unknown_key_errors() {
    let home = tmp_home("copy_unknown");
    let resume = "Jane Doe\njane@example.com\n";
    let mut child = Command::new(bin_path())
        .arg("init")
        .env("HOME", &home)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    child.stdin.as_mut().unwrap().write_all(resume.as_bytes()).unwrap();
    child.wait().unwrap();

    let out = Command::new(bin_path())
        .arg("copy")
        .arg("not_a_real_key")
        .env("HOME", &home)
        .output()
        .expect("run copy unknown");
    assert!(!out.status.success(), "unknown key must fail");
    let _ = std::fs::remove_dir_all(home);
}

#[test]
fn cdp_probe_without_running_chromium_exits_cleanly() {
    let home = tmp_home("cdp_probe_no_chrome");
    let out = Command::new(bin_path())
        .arg("cdp-probe")
        .env("HOME", &home)
        // Force the probe to look at a port nothing's on.
        .env("CHROME_DEBUG_PORT", "1") // privileged, no daemon
        .output()
        .expect("run cdp-probe");
    // The probe is "diagnostic" — exits 0 either way, with a message.
    assert!(out.status.success());
    let _ = std::fs::remove_dir_all(home);
}

// ─── Inspect subcommand — exercises the engine's internal-page
//     router via subprocess. Catches: clap wiring, internal::render
//     match-arm coverage, page body content drift. Each test runs
//     the actual built binary, parses stdout, asserts content.
//     These are integration tests by definition — they cross
//     process boundaries and exercise the full main fn.

#[cfg(feature = "gui")]
#[test]
fn inspect_internal_home_url_renders_home_page() {
    // The in-process atsisbroken://home page is still reachable
    // even after Url::home() flipped to the network landing
    // page. Pin the internal route explicitly so this test
    // doesn't depend on DNS for atsisbroken.cochranblock.org.
    let out = Command::new(bin_path())
        .arg("inspect")
        .arg("--url")
        .arg("atsisbroken://home")
        .output()
        .expect("run inspect");
    assert!(
        out.status.success(),
        "inspect atsisbroken://home must exit 0"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("URL: atsisbroken://home"), "stdout: {stdout}");
    assert!(stdout.contains("Title: atsisbroken"));
    assert!(stdout.contains("Fields: 0"));
    assert!(stdout.contains("atsisbroken://connections"));
}

#[cfg(feature = "gui")]
#[test]
fn inspect_connections_page_lists_services() {
    let out = Command::new(bin_path())
        .arg("inspect")
        .arg("--url")
        .arg("atsisbroken://connections")
        .output()
        .expect("run inspect connections");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Title: Your connections"));
    // Public-handle services
    for service in ["GitHub", "Stack Overflow", "Hacker News", "crates.io"] {
        assert!(stdout.contains(service), "missing {service}: {stdout}");
    }
    // OAuth services
    assert!(stdout.contains("LinkedIn"));
    // Session-cookie services
    assert!(stdout.contains("Substack"));
    // Connect targets
    assert!(stdout.contains("atsisbroken://connect/github"));
}

#[cfg(feature = "gui")]
#[test]
fn inspect_unknown_internal_url_returns_not_found() {
    let out = Command::new(bin_path())
        .arg("inspect")
        .arg("--url")
        .arg("atsisbroken://nope")
        .output()
        .expect("run inspect nope");
    assert!(out.status.success(), "not-found is still a successful render");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Title: Page not found"));
    assert!(stdout.contains("atsisbroken://nope"));
}

#[cfg(feature = "gui")]
#[test]
fn inspect_invalid_url_errors_with_helpful_message() {
    let out = Command::new(bin_path())
        .arg("inspect")
        .arg("--url")
        .arg("")
        .output()
        .expect("run inspect empty");
    assert!(!out.status.success(), "empty URL must error");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("invalid"), "stderr should explain: {stderr}");
}

#[cfg(feature = "gui")]
#[test]
fn inspect_fingerprint_page_documents_webdriver_false() {
    // Pin the user-visible contract: navigator.webdriver=false
    // is documented on the fingerprint page. A regression here
    // would mean the contract changed without updating the
    // public surface.
    let out = Command::new(bin_path())
        .arg("inspect")
        .arg("--url")
        .arg("atsisbroken://fingerprint")
        .output()
        .expect("run inspect fingerprint");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("navigator.webdriver"));
    assert!(stdout.contains("false"));
}

#[cfg(feature = "gui")]
#[test]
fn inspect_summary_page_directs_empty_graph_to_connections() {
    // Empty-graph state should tell the user where to go next
    // instead of silently rendering an empty bio. Pin that
    // user-experience contract.
    let out = Command::new(bin_path())
        .arg("inspect")
        .arg("--url")
        .arg("atsisbroken://summary")
        .output()
        .expect("run inspect summary");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("atsisbroken://connections"));
}
