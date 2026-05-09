// SPDX-License-Identifier: Unlicense

//! Tests for `crate::paths` (Phase 4).

use crate::paths;

use super::{case, check, check_eq, TestResult};

pub fn run() -> Vec<TestResult> {
    vec![
        case("paths::paths_are_under_atsisbroken_dir", paths_are_under_atsisbroken_dir),
        case("paths::native_host_dir_known_on_supported_os", native_host_dir_known_on_supported_os),
        case("paths::paths_have_expected_filenames", paths_have_expected_filenames),
        case("paths::dotted_dirname_is_atsisbroken", dotted_dirname_is_atsisbroken),
        case("paths::ensure_dir_is_idempotent", ensure_dir_is_idempotent),
        case("paths::native_host_dir_when_set_is_absolute", native_host_dir_when_set_is_absolute),
        case("paths::native_host_dir_ends_with_native_messaging_hosts",
             native_host_dir_ends_with_native_messaging_hosts),
        case("paths::profile_path_override_used_when_provided",
             profile_path_override_used_when_provided),
        case("paths::profile_path_override_none_falls_back_to_default",
             profile_path_override_none_falls_back_to_default),
    ]
}

fn paths_are_under_atsisbroken_dir() -> Result<(), String> {
    let root = paths::atsisbroken_dir();
    check(paths::profile_path().starts_with(&root), "profile under root")?;
    check(paths::config_path().starts_with(&root), "config under root")?;
    check(paths::feedback_jsonl_path().starts_with(&root), "feedback under root")?;
    check(paths::model_path().starts_with(&root), "model under root")
}

fn native_host_dir_known_on_supported_os() -> Result<(), String> {
    if cfg!(any(target_os = "macos", target_os = "linux")) {
        check(paths::chrome_native_host_dir().is_some(), "should have native host dir")
    } else {
        check(paths::chrome_native_host_dir().is_none(), "should not have native host dir")
    }
}

fn paths_have_expected_filenames() -> Result<(), String> {
    // File names are part of the user's mental model — pin them.
    check_eq(
        paths::profile_path().file_name().and_then(|s| s.to_str()),
        Some("profile.toml"),
        "profile filename",
    )?;
    check_eq(
        paths::feedback_jsonl_path().file_name().and_then(|s| s.to_str()),
        Some("feedback.jsonl"),
        "feedback filename",
    )?;
    check_eq(
        paths::model_path().file_name().and_then(|s| s.to_str()),
        Some("model.json"),
        "model filename",
    )?;
    check_eq(
        paths::config_path().file_name().and_then(|s| s.to_str()),
        Some("config.toml"),
        "config filename",
    )
}

fn dotted_dirname_is_atsisbroken() -> Result<(), String> {
    // The leading-dot convention means it doesn't pollute `ls`.
    check_eq(
        paths::atsisbroken_dir().file_name().and_then(|s| s.to_str()),
        Some(".atsisbroken"),
        "dotted dirname",
    )
}

fn ensure_dir_is_idempotent() -> Result<(), String> {
    // Calling ensure_dir twice must succeed both times.
    let d1 = paths::ensure_dir().map_err(|e| format!("first: {e}"))?;
    let d2 = paths::ensure_dir().map_err(|e| format!("second: {e}"))?;
    check_eq(&d1, &d2, "ensure_dir returns same path twice")?;
    check(d1.exists(), "directory should exist after ensure")
}

fn native_host_dir_when_set_is_absolute() -> Result<(), String> {
    if let Some(d) = paths::chrome_native_host_dir() {
        check(d.is_absolute(), format!("native host dir not absolute: {}", d.display()))?;
    }
    Ok(())
}

fn native_host_dir_ends_with_native_messaging_hosts() -> Result<(), String> {
    if let Some(d) = paths::chrome_native_host_dir() {
        check_eq(
            d.file_name().and_then(|s| s.to_str()),
            Some("NativeMessagingHosts"),
            "native host dir name",
        )?;
    }
    Ok(())
}

fn profile_path_override_used_when_provided() -> Result<(), String> {
    let custom = std::path::PathBuf::from("/tmp/clients/jane.toml");
    check_eq(
        paths::profile_path_with_override(Some(&custom)),
        custom,
        "override should be returned",
    )
}

fn profile_path_override_none_falls_back_to_default() -> Result<(), String> {
    check_eq(
        paths::profile_path_with_override(None),
        paths::profile_path(),
        "None should fall back to default",
    )
}
