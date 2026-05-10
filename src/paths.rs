// SPDX-License-Identifier: Unlicense
//! On-disk locations atsisbroken owns under the user's home dir.
//!
//! Single canonical resolver so the rest of the code never duplicates
//! `~/.atsisbroken/...` strings. Every path is per-user, never world-
//! readable by default.
//!
//! ## Two flavors per path
//!
//! Most callers want the canonical path under the *real* user's
//! home directory and use the bare functions (`atsisbroken_dir()`,
//! `profile_path()`, etc.) which read `$HOME` from the process
//! environment. The cli_smoke tests in [`crate::tests::cli_smoke`]
//! redirect every path under a `tempdir` per-test by passing an
//! explicit home into the `_with_home` variants. The bare
//! functions are wrappers around the explicit ones.

use std::path::{Path, PathBuf};

// ─── canonical (read $HOME from process env) ──────────────────────────

/// `~/.atsisbroken/`. Created on demand by [`ensure_dir`].
pub fn atsisbroken_dir() -> PathBuf {
    atsisbroken_dir_with_home(&home_dir())
}

pub fn profile_path() -> PathBuf {
    profile_path_with_home(&home_dir())
}

/// If `override_path` is `Some`, use that path as the profile location
/// (the `--profile` CLI flag). Otherwise fall back to the canonical
/// `~/.atsisbroken/profile.toml`.
pub fn profile_path_with_override(override_path: Option<&Path>) -> PathBuf {
    match override_path {
        Some(p) => p.to_path_buf(),
        None => profile_path(),
    }
}

pub fn config_path() -> PathBuf {
    config_path_with_home(&home_dir())
}

pub fn feedback_jsonl_path() -> PathBuf {
    feedback_jsonl_path_with_home(&home_dir())
}

pub fn model_path() -> PathBuf {
    model_path_with_home(&home_dir())
}

/// `~/.atsisbroken/github_inventory.json` — Phase I sync output.
pub fn github_inventory_path() -> PathBuf {
    github_inventory_path_with_home(&home_dir())
}

/// `~/.atsisbroken/github_token` — Phase I auth.
pub fn github_token_path() -> PathBuf {
    github_token_path_with_home(&home_dir())
}

/// Per-Chrome-family Native Messaging host manifest target.
pub fn chrome_native_host_dir() -> Option<PathBuf> {
    chrome_native_host_dir_with_home(&home_dir())
}

/// Create `~/.atsisbroken/` if it doesn't exist.
pub fn ensure_dir() -> std::io::Result<PathBuf> {
    ensure_dir_with_home(&home_dir())
}

// ─── explicit-home variants (used by cli_smoke + any caller that
//     needs a non-real-user home, like a tempdir) ──────────────────────

pub fn atsisbroken_dir_with_home(home: &Path) -> PathBuf {
    home.join(".atsisbroken")
}

pub fn profile_path_with_home(home: &Path) -> PathBuf {
    atsisbroken_dir_with_home(home).join("profile.toml")
}

pub fn profile_path_with_override_and_home(
    override_path: Option<&Path>,
    home: &Path,
) -> PathBuf {
    match override_path {
        Some(p) => p.to_path_buf(),
        None => profile_path_with_home(home),
    }
}

pub fn config_path_with_home(home: &Path) -> PathBuf {
    atsisbroken_dir_with_home(home).join("config.toml")
}

pub fn feedback_jsonl_path_with_home(home: &Path) -> PathBuf {
    atsisbroken_dir_with_home(home).join("feedback.jsonl")
}

pub fn model_path_with_home(home: &Path) -> PathBuf {
    atsisbroken_dir_with_home(home).join("model.json")
}

pub fn github_inventory_path_with_home(home: &Path) -> PathBuf {
    atsisbroken_dir_with_home(home).join("github_inventory.json")
}

pub fn github_token_path_with_home(home: &Path) -> PathBuf {
    atsisbroken_dir_with_home(home).join("github_token")
}

pub fn chrome_native_host_dir_with_home(home: &Path) -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        Some(
            home.join("Library")
                .join("Application Support")
                .join("Google")
                .join("Chrome")
                .join("NativeMessagingHosts"),
        )
    } else if cfg!(target_os = "linux") {
        Some(
            home.join(".config")
                .join("google-chrome")
                .join("NativeMessagingHosts"),
        )
    } else {
        None
    }
}

pub fn ensure_dir_with_home(home: &Path) -> std::io::Result<PathBuf> {
    let dir = atsisbroken_dir_with_home(home);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

// ─── helpers ──────────────────────────────────────────────────────────

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}
