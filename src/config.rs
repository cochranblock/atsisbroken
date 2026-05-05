// SPDX-License-Identifier: Unlicense
//! Persistent user configuration at `~/.atsisbroken/config.toml`.
//!
//! Holds the autonomy `Mode`, the `ConfidenceThreshold` for Shadow
//! mode, and the `FeedbackDelivery` opt-in. Read on every CLI
//! invocation that needs current state; written by `graduate` and
//! the TUI's mode toggle.
//!
//! Atomic writes via `<path>.tmp + rename`. A missing file yields a
//! default config — first-run is not an error.

use crate::{paths, ConfidenceThreshold, FeedbackDelivery, Mode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    #[serde(default)]
    pub mode: Mode,
    #[serde(default)]
    pub confidence_threshold: ConfidenceThreshold,
    #[serde(default)]
    pub feedback_delivery: FeedbackDelivery,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: Mode::default(),
            confidence_threshold: ConfidenceThreshold::default(),
            feedback_delivery: FeedbackDelivery::default(),
        }
    }
}

impl Config {
    /// Load from `path`. Missing file → default. Corrupt file → error.
    pub fn load_from(path: &std::path::Path) -> std::io::Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(text) => toml::from_str(&text)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e),
        }
    }

    /// Atomic write to `path` via `.tmp + rename`.
    pub fn save_to(&self, path: &std::path::Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let body = toml::to_string_pretty(self).map_err(std::io::Error::other)?;
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, body)?;
        std::fs::rename(&tmp, path)
    }

    /// Load from the canonical `~/.atsisbroken/config.toml`.
    pub fn load() -> std::io::Result<Self> {
        Self::load_from(&paths::config_path())
    }

    /// Save to the canonical `~/.atsisbroken/config.toml`.
    pub fn save(&self) -> std::io::Result<()> {
        self.save_to(&paths::config_path())
    }

    /// Advance one rung up the autonomy ladder. Returns the new mode
    /// or `Err` if already at Chaos.
    pub fn graduate(&mut self) -> Result<Mode, GraduationError> {
        let next = match self.mode {
            Mode::TrainingWheels => Mode::Shadow,
            Mode::Shadow => Mode::Chaos,
            Mode::Chaos => return Err(GraduationError::AlreadyAtTop),
        };
        self.mode = next;
        Ok(next)
    }

    /// Step one rung back down. Returns the new mode or `Err` if
    /// already at TrainingWheels.
    pub fn step_back(&mut self) -> Result<Mode, GraduationError> {
        let prev = match self.mode {
            Mode::Chaos => Mode::Shadow,
            Mode::Shadow => Mode::TrainingWheels,
            Mode::TrainingWheels => return Err(GraduationError::AlreadyAtBottom),
        };
        self.mode = prev;
        Ok(prev)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraduationError {
    AlreadyAtTop,
    AlreadyAtBottom,
}

impl std::fmt::Display for GraduationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GraduationError::AlreadyAtTop => {
                write!(f, "already at Chaos — pass --back to step down")
            }
            GraduationError::AlreadyAtBottom => {
                write!(f, "already at TrainingWheels — already the most supervised mode")
            }
        }
    }
}

impl std::error::Error for GraduationError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_path(label: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "atsisbroken_config_{label}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        p
    }

    #[test]
    fn missing_file_yields_default_not_error() {
        let path = tmp_path("missing");
        let cfg = Config::load_from(&path).unwrap();
        assert_eq!(cfg, Config::default());
    }

    #[test]
    fn round_trip_through_disk() {
        let dir = tmp_path("roundtrip");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        let cfg = Config {
            mode: Mode::Shadow,
            confidence_threshold: ConfidenceThreshold(0.92),
            feedback_delivery: FeedbackDelivery::SendWhenOnline {
                destination: "mailto:me@example.com".into(),
            },
        };
        cfg.save_to(&path).unwrap();
        let back = Config::load_from(&path).unwrap();
        assert_eq!(cfg, back);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_atomically_via_tmp() {
        let dir = tmp_path("atomic");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        Config::default().save_to(&path).unwrap();
        let tmp = path.with_extension("toml.tmp");
        assert!(!tmp.exists(), ".tmp must be renamed away");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn graduate_walks_training_wheels_to_chaos() {
        let mut cfg = Config::default();
        assert_eq!(cfg.mode, Mode::TrainingWheels);
        assert_eq!(cfg.graduate().unwrap(), Mode::Shadow);
        assert_eq!(cfg.mode, Mode::Shadow);
        assert_eq!(cfg.graduate().unwrap(), Mode::Chaos);
        assert_eq!(cfg.mode, Mode::Chaos);
    }

    #[test]
    fn graduate_at_chaos_errors_does_not_mutate() {
        let mut cfg = Config {
            mode: Mode::Chaos,
            ..Default::default()
        };
        let err = cfg.graduate().unwrap_err();
        assert_eq!(err, GraduationError::AlreadyAtTop);
        assert_eq!(cfg.mode, Mode::Chaos);
    }

    #[test]
    fn step_back_walks_chaos_to_training_wheels() {
        let mut cfg = Config {
            mode: Mode::Chaos,
            ..Default::default()
        };
        assert_eq!(cfg.step_back().unwrap(), Mode::Shadow);
        assert_eq!(cfg.step_back().unwrap(), Mode::TrainingWheels);
    }

    #[test]
    fn step_back_at_training_wheels_errors_does_not_mutate() {
        let mut cfg = Config {
            mode: Mode::TrainingWheels,
            ..Default::default()
        };
        let err = cfg.step_back().unwrap_err();
        assert_eq!(err, GraduationError::AlreadyAtBottom);
        assert_eq!(cfg.mode, Mode::TrainingWheels);
    }

    #[test]
    fn config_with_only_mode_field_loads_with_defaults_for_rest() {
        // Forward-compat: a config.toml that has only `mode = ...`
        // (because the user wrote it before we added other fields)
        // must still load. `#[serde(default)]` on each field handles
        // this; pin it.
        let dir = tmp_path("partial");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        std::fs::write(&path, "mode = \"shadow\"\n").unwrap();
        let cfg = Config::load_from(&path).unwrap();
        assert_eq!(cfg.mode, Mode::Shadow);
        assert_eq!(cfg.confidence_threshold, ConfidenceThreshold::default());
        assert_eq!(cfg.feedback_delivery, FeedbackDelivery::default());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn graduation_error_messages_are_actionable() {
        // The error string must hint at the recovery action; otherwise
        // the user sees "error: already at top" with no idea what to do.
        assert!(GraduationError::AlreadyAtTop
            .to_string()
            .contains("--back"));
        assert!(GraduationError::AlreadyAtBottom
            .to_string()
            .contains("most supervised"));
    }
}
