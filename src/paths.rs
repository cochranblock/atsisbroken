// SPDX-License-Identifier: Unlicense
//! On-disk locations atsisbroken owns under the user's home dir.
//!
//! Single canonical resolver so the rest of the code never duplicates
//! `~/.atsisbroken/...` strings. Every path is per-user, never world-
//! readable by default.

use std::path::PathBuf;

/// `~/.atsisbroken/`. Created on demand by [`ensure_dir`].
pub fn atsisbroken_dir() -> PathBuf {
    let mut p = home_dir();
    p.push(".atsisbroken");
    p
}

pub fn profile_path() -> PathBuf {
    let mut p = atsisbroken_dir();
    p.push("profile.toml");
    p
}

/// If `override_path` is `Some`, use that path as the profile location
/// (the `--profile` CLI flag). Otherwise fall back to the canonical
/// `~/.atsisbroken/profile.toml`. Lets a user (e.g. P4 career counselor)
/// keep multiple profiles side by side without env-var juggling.
pub fn profile_path_with_override(override_path: Option<&std::path::Path>) -> PathBuf {
    match override_path {
        Some(p) => p.to_path_buf(),
        None => profile_path(),
    }
}

pub fn config_path() -> PathBuf {
    let mut p = atsisbroken_dir();
    p.push("config.toml");
    p
}

pub fn feedback_jsonl_path() -> PathBuf {
    let mut p = atsisbroken_dir();
    p.push("feedback.jsonl");
    p
}

pub fn model_path() -> PathBuf {
    let mut p = atsisbroken_dir();
    p.push("model.json");
    p
}

/// `~/.atsisbroken/github_inventory.json` — Phase I sync output.
/// Public-repo metadata + README excerpts + recent commit messages.
/// Never ships the user's source code, only structural metadata.
pub fn github_inventory_path() -> PathBuf {
    let mut p = atsisbroken_dir();
    p.push("github_inventory.json");
    p
}

/// `~/.atsisbroken/github_token` — Phase I auth.
/// Plain-text token file, chmod 600 on Unix, owner-only on Windows
/// via best-effort ACL. Never logged, never crosses the Native
/// Messaging bridge, never appears in any TUI tab. The token is
/// optional; without it sync is rate-limited at 60 req/h.
pub fn github_token_path() -> PathBuf {
    let mut p = atsisbroken_dir();
    p.push("github_token");
    p
}

/// Per-Chrome-family Native Messaging host manifest target. Returns the
/// directory the host JSON belongs in, per OS. None ⇒ unsupported on this
/// platform (Windows uses the registry; not implemented yet).
pub fn chrome_native_host_dir() -> Option<PathBuf> {
    let mut p = home_dir();
    if cfg!(target_os = "macos") {
        p.push("Library");
        p.push("Application Support");
        p.push("Google");
        p.push("Chrome");
        p.push("NativeMessagingHosts");
        Some(p)
    } else if cfg!(target_os = "linux") {
        p.push(".config");
        p.push("google-chrome");
        p.push("NativeMessagingHosts");
        Some(p)
    } else {
        None
    }
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Create `~/.atsisbroken/` if it doesn't exist.
pub fn ensure_dir() -> std::io::Result<PathBuf> {
    let dir = atsisbroken_dir();
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

