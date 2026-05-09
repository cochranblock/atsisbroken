// SPDX-License-Identifier: Unlicense

//! Tests for `crate::config` (Phase 4).

use crate::config::{Config, GraduationError};
use crate::{ConfidenceThreshold, FeedbackDelivery, Mode};

use super::{case, check, check_eq, TestResult};

pub fn run() -> Vec<TestResult> {
    vec![
        case("config::missing_file_yields_default_not_error", missing_file_yields_default_not_error),
        case("config::round_trip_through_disk", round_trip_through_disk),
        case("config::save_atomically_via_tmp", save_atomically_via_tmp),
        case("config::graduate_walks_training_wheels_to_chaos",
             graduate_walks_training_wheels_to_chaos),
        case("config::graduate_at_chaos_errors_does_not_mutate", graduate_at_chaos_errors_does_not_mutate),
        case("config::step_back_walks_chaos_to_training_wheels",
             step_back_walks_chaos_to_training_wheels),
        case("config::step_back_at_training_wheels_errors_does_not_mutate",
             step_back_at_training_wheels_errors_does_not_mutate),
        case("config::config_with_only_mode_field_loads_with_defaults_for_rest",
             config_with_only_mode_field_loads_with_defaults_for_rest),
        case("config::graduation_error_messages_are_actionable",
             graduation_error_messages_are_actionable),
    ]
}

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

fn missing_file_yields_default_not_error() -> Result<(), String> {
    let path = tmp_path("missing");
    let cfg = Config::load_from(&path).map_err(|e| format!("{e}"))?;
    check_eq(cfg, Config::default(), "missing file → default")
}

fn round_trip_through_disk() -> Result<(), String> {
    let dir = tmp_path("roundtrip");
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {e}"))?;
    let path = dir.join("config.toml");
    let cfg = Config {
        mode: Mode::Shadow,
        confidence_threshold: ConfidenceThreshold(0.92),
        feedback_delivery: FeedbackDelivery::SendWhenOnline {
            destination: "mailto:me@example.com".into(),
        },
    };
    cfg.save_to(&path).map_err(|e| format!("save: {e}"))?;
    let back = Config::load_from(&path).map_err(|e| format!("load: {e}"))?;
    check_eq(cfg, back, "config round-trip")?;
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

fn save_atomically_via_tmp() -> Result<(), String> {
    let dir = tmp_path("atomic");
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {e}"))?;
    let path = dir.join("config.toml");
    Config::default().save_to(&path).map_err(|e| format!("save: {e}"))?;
    let tmp = path.with_extension("toml.tmp");
    check(!tmp.exists(), ".tmp must be renamed away")?;
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

fn graduate_walks_training_wheels_to_chaos() -> Result<(), String> {
    let mut cfg = Config::default();
    check_eq(cfg.mode, Mode::TrainingWheels, "starts at TrainingWheels")?;
    check_eq(cfg.graduate().map_err(|e| format!("{e}"))?, Mode::Shadow, "→ Shadow")?;
    check_eq(cfg.mode, Mode::Shadow, "now Shadow")?;
    check_eq(cfg.graduate().map_err(|e| format!("{e}"))?, Mode::Chaos, "→ Chaos")?;
    check_eq(cfg.mode, Mode::Chaos, "now Chaos")
}

fn graduate_at_chaos_errors_does_not_mutate() -> Result<(), String> {
    let mut cfg = Config {
        mode: Mode::Chaos,
        ..Default::default()
    };
    let err = cfg.graduate().err().ok_or_else(|| "expected error".to_string())?;
    check_eq(err, GraduationError::AlreadyAtTop, "expected AlreadyAtTop")?;
    check_eq(cfg.mode, Mode::Chaos, "mode unchanged after error")
}

fn step_back_walks_chaos_to_training_wheels() -> Result<(), String> {
    let mut cfg = Config {
        mode: Mode::Chaos,
        ..Default::default()
    };
    check_eq(cfg.step_back().map_err(|e| format!("{e}"))?, Mode::Shadow, "→ Shadow")?;
    check_eq(
        cfg.step_back().map_err(|e| format!("{e}"))?,
        Mode::TrainingWheels,
        "→ TrainingWheels",
    )
}

fn step_back_at_training_wheels_errors_does_not_mutate() -> Result<(), String> {
    let mut cfg = Config {
        mode: Mode::TrainingWheels,
        ..Default::default()
    };
    let err = cfg.step_back().err().ok_or_else(|| "expected error".to_string())?;
    check_eq(err, GraduationError::AlreadyAtBottom, "expected AlreadyAtBottom")?;
    check_eq(cfg.mode, Mode::TrainingWheels, "mode unchanged after error")
}

fn config_with_only_mode_field_loads_with_defaults_for_rest() -> Result<(), String> {
    // Forward-compat: config.toml with only `mode = ...` (because
    // user wrote it before we added other fields) must still load.
    let dir = tmp_path("partial");
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {e}"))?;
    let path = dir.join("config.toml");
    std::fs::write(&path, "mode = \"shadow\"\n").map_err(|e| format!("write: {e}"))?;
    let cfg = Config::load_from(&path).map_err(|e| format!("load: {e}"))?;
    check_eq(cfg.mode, Mode::Shadow, "partial-config mode")?;
    check_eq(
        cfg.confidence_threshold,
        ConfidenceThreshold::default(),
        "partial-config threshold",
    )?;
    check_eq(
        cfg.feedback_delivery,
        FeedbackDelivery::default(),
        "partial-config delivery",
    )?;
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

fn graduation_error_messages_are_actionable() -> Result<(), String> {
    // The error string must hint at the recovery action.
    check(
        GraduationError::AlreadyAtTop.to_string().contains("--back"),
        "AlreadyAtTop message should mention --back",
    )?;
    check(
        GraduationError::AlreadyAtBottom
            .to_string()
            .contains("most supervised"),
        "AlreadyAtBottom message should mention 'most supervised'",
    )
}
