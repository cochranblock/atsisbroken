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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_are_under_atsisbroken_dir() {
        let root = atsisbroken_dir();
        assert!(profile_path().starts_with(&root));
        assert!(config_path().starts_with(&root));
        assert!(feedback_jsonl_path().starts_with(&root));
        assert!(model_path().starts_with(&root));
    }

    #[test]
    fn native_host_dir_known_on_supported_os() {
        if cfg!(any(target_os = "macos", target_os = "linux")) {
            assert!(chrome_native_host_dir().is_some());
        } else {
            assert!(chrome_native_host_dir().is_none());
        }
    }

    #[test]
    fn paths_have_expected_filenames() {
        // File names are part of the user's mental model — pin them.
        assert_eq!(
            profile_path().file_name().and_then(|s| s.to_str()),
            Some("profile.toml")
        );
        assert_eq!(
            feedback_jsonl_path().file_name().and_then(|s| s.to_str()),
            Some("feedback.jsonl")
        );
        assert_eq!(
            model_path().file_name().and_then(|s| s.to_str()),
            Some("model.json")
        );
        assert_eq!(
            config_path().file_name().and_then(|s| s.to_str()),
            Some("config.toml")
        );
    }
}
