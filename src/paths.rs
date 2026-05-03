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

    #[test]
    fn dotted_dirname_is_atsisbroken() {
        // The leading-dot convention means it doesn't pollute `ls`.
        // If anyone "fixes" this to ~/atsisbroken/, the user's home
        // dir gets noisy and we lose the "config dir" mental category.
        assert_eq!(
            atsisbroken_dir().file_name().and_then(|s| s.to_str()),
            Some(".atsisbroken")
        );
    }

    #[test]
    fn ensure_dir_is_idempotent() {
        // Calling ensure_dir twice must succeed both times — once to
        // create, once to confirm it already exists. We do this in the
        // real ~/.atsisbroken/ since touching env vars in tests races
        // with parallel test threads on Rust 2024.
        let d1 = ensure_dir().unwrap();
        let d2 = ensure_dir().unwrap();
        assert_eq!(d1, d2);
        assert!(d1.exists());
    }

    #[test]
    fn native_host_dir_when_set_is_absolute() {
        if let Some(d) = chrome_native_host_dir() {
            assert!(
                d.is_absolute(),
                "native host dir must be absolute: {}",
                d.display()
            );
        }
    }

    #[test]
    fn native_host_dir_ends_with_native_messaging_hosts() {
        if let Some(d) = chrome_native_host_dir() {
            assert_eq!(
                d.file_name().and_then(|s| s.to_str()),
                Some("NativeMessagingHosts")
            );
        }
    }

    #[test]
    fn profile_path_override_used_when_provided() {
        let custom = std::path::PathBuf::from("/tmp/clients/jane.toml");
        assert_eq!(profile_path_with_override(Some(&custom)), custom);
    }

    #[test]
    fn profile_path_override_none_falls_back_to_default() {
        assert_eq!(profile_path_with_override(None), profile_path());
    }
}
