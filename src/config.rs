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

