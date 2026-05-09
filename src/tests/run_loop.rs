// SPDX-License-Identifier: Unlicense

//! Tests for `crate::run_loop` (Phase 7).

use crate::learning::UserDecision;
use crate::run_loop::{
    decide, fill_js, format_summary, read_back_js, FillDecision, RunSummary, SkipReason,
    VerifyOutcome,
};
use crate::{ConfidenceThreshold, Mode};

use super::{case, check, check_eq, TestResult};

pub fn run() -> Vec<TestResult> {
    vec![
        case("run_loop::fill_js_escapes_single_quotes", fill_js_escapes_single_quotes),
        case("run_loop::fill_js_escapes_backslash", fill_js_escapes_backslash),
        case("run_loop::fill_js_uses_id_in_getelementbyid", fill_js_uses_id_in_getelementbyid),
        case("run_loop::read_back_js_targets_correct_id_and_returns_value",
             read_back_js_targets_correct_id_and_returns_value),
        case("run_loop::verify_outcome_default_is_zeroed", verify_outcome_default_is_zeroed),
        case("run_loop::fill_js_dispatches_input_and_change_events",
             fill_js_dispatches_input_and_change_events),
        case("run_loop::format_summary_warns_about_not_submitting",
             format_summary_warns_about_not_submitting),
        case("run_loop::decide_skip_when_classifier_returns_unknown",
             decide_skip_when_classifier_returns_unknown),
        case("run_loop::decide_skip_when_no_profile_value", decide_skip_when_no_profile_value),
        case("run_loop::decide_skip_when_no_dom_id", decide_skip_when_no_dom_id),
        case("run_loop::decide_chaos_fills_classified_field", decide_chaos_fills_classified_field),
        case("run_loop::decide_training_wheels_always_prompts_classified_fields",
             decide_training_wheels_always_prompts_classified_fields),
        case("run_loop::decide_shadow_fills_above_threshold", decide_shadow_fills_above_threshold),
        case("run_loop::decide_shadow_fills_at_threshold", decide_shadow_fills_at_threshold),
        case("run_loop::decide_shadow_skips_below_threshold", decide_shadow_skips_below_threshold),
        case("run_loop::decide_skip_precedence_unknown_beats_threshold",
             decide_skip_precedence_unknown_beats_threshold),
        case("run_loop::decide_skip_precedence_no_value_beats_prompt",
             decide_skip_precedence_no_value_beats_prompt),
        case("run_loop::decide_overlay_rejected_short_circuits_in_any_mode",
             decide_overlay_rejected_short_circuits_in_any_mode),
        case("run_loop::decide_overlay_accepted_skips_prompt_in_training_wheels",
             decide_overlay_accepted_skips_prompt_in_training_wheels),
        case("run_loop::decide_overlay_accepted_overrides_shadow_threshold",
             decide_overlay_accepted_overrides_shadow_threshold),
        case("run_loop::decide_overlay_does_not_short_circuit_skip_precedence",
             decide_overlay_does_not_short_circuit_skip_precedence),
        case("run_loop::format_summary_shows_screenshot_path_when_set",
             format_summary_shows_screenshot_path_when_set),
    ]
}

fn t() -> ConfidenceThreshold {
    ConfidenceThreshold(0.85)
}

fn fill_js_escapes_single_quotes() -> Result<(), String> {
    let js = fill_js("foo", "Jane's resume");
    check(js.contains("Jane\\'s resume"), "single quote escaped")
}

fn fill_js_escapes_backslash() -> Result<(), String> {
    let js = fill_js("foo", r"C:\path");
    check(js.contains(r"C:\\path"), "backslash doubled")
}

fn fill_js_uses_id_in_getelementbyid() -> Result<(), String> {
    let js = fill_js("legalNameSection_firstName", "Jane");
    check(
        js.contains("getElementById('legalNameSection_firstName')"),
        "uses getElementById with id",
    )
}

fn read_back_js_targets_correct_id_and_returns_value() -> Result<(), String> {
    let js = read_back_js("legalNameSection_firstName");
    check(
        js.contains("getElementById('legalNameSection_firstName')"),
        "uses getElementById with id",
    )?;
    check(js.contains("el.value"), "reads el.value")?;
    check(js.contains("null"), "returns null for missing")
}

fn verify_outcome_default_is_zeroed() -> Result<(), String> {
    let o = VerifyOutcome::default();
    check_eq(o.retried_total, 0usize, "retried_total")?;
    check_eq(o.retried_filled, 0usize, "retried_filled")?;
    check_eq(o.failed, 0usize, "failed")
}

fn fill_js_dispatches_input_and_change_events() -> Result<(), String> {
    let js = fill_js("foo", "bar");
    check(js.contains("new Event('input'"), "input event")?;
    check(js.contains("new Event('change'"), "change event")
}

fn format_summary_warns_about_not_submitting() -> Result<(), String> {
    let s = RunSummary {
        url: "https://example.com".into(),
        fields_seen: 10,
        fields_filled: 7,
        fields_skipped: 3,
        screenshot_path: None,
        feedback_events: vec![],
    };
    let out = format_summary(&s, 0);
    check(out.contains("NOT submitted"), "warns NOT submitted")?;
    check(out.contains("review"), "mentions review")
}

fn decide_skip_when_classifier_returns_unknown() -> Result<(), String> {
    let d = decide(Mode::Chaos, "", 0.0, t(), Some("anything"), true, None);
    check_eq(d, FillDecision::Skip(SkipReason::NotClassified), "NotClassified")
}

fn decide_skip_when_no_profile_value() -> Result<(), String> {
    let d = decide(Mode::Chaos, "linkedin", 1.0, t(), None, true, None);
    check_eq(d, FillDecision::Skip(SkipReason::NoProfileValue), "NoProfileValue")
}

fn decide_skip_when_no_dom_id() -> Result<(), String> {
    let d = decide(Mode::Chaos, "email", 1.0, t(), Some("j@e.com"), false, None);
    check_eq(d, FillDecision::Skip(SkipReason::NoDomId), "NoDomId")
}

fn decide_chaos_fills_classified_field() -> Result<(), String> {
    let d = decide(Mode::Chaos, "email", 1.0, t(), Some("j@e.com"), true, None);
    check_eq(d, FillDecision::Fill { value: "j@e.com" }, "Chaos fills")
}

fn decide_training_wheels_always_prompts_classified_fields() -> Result<(), String> {
    let d = decide(Mode::TrainingWheels, "email", 1.0, t(), Some("j@e.com"), true, None);
    check_eq(
        d,
        FillDecision::Prompt {
            value: "j@e.com",
            key: "email",
        },
        "TrainingWheels prompts",
    )
}

fn decide_shadow_fills_above_threshold() -> Result<(), String> {
    let d = decide(Mode::Shadow, "email", 0.90, t(), Some("j@e.com"), true, None);
    check_eq(d, FillDecision::Fill { value: "j@e.com" }, "Shadow fills above threshold")
}

fn decide_shadow_fills_at_threshold() -> Result<(), String> {
    let d = decide(Mode::Shadow, "email", 0.85, t(), Some("j@e.com"), true, None);
    check_eq(d, FillDecision::Fill { value: "j@e.com" }, "Shadow fills at threshold (>=)")
}

fn decide_shadow_skips_below_threshold() -> Result<(), String> {
    let d = decide(Mode::Shadow, "email", 0.84, t(), Some("j@e.com"), true, None);
    check_eq(
        d,
        FillDecision::Skip(SkipReason::BelowConfidenceThreshold),
        "Shadow skips below threshold",
    )
}

fn decide_skip_precedence_unknown_beats_threshold() -> Result<(), String> {
    let d = decide(Mode::Shadow, "", 1.0, t(), Some("anything"), true, None);
    check_eq(
        d,
        FillDecision::Skip(SkipReason::NotClassified),
        "NotClassified beats threshold",
    )
}

fn decide_skip_precedence_no_value_beats_prompt() -> Result<(), String> {
    let d = decide(Mode::TrainingWheels, "linkedin", 1.0, t(), None, true, None);
    check_eq(
        d,
        FillDecision::Skip(SkipReason::NoProfileValue),
        "NoProfileValue beats Prompt",
    )
}

fn decide_overlay_rejected_short_circuits_in_any_mode() -> Result<(), String> {
    for mode in [Mode::TrainingWheels, Mode::Shadow, Mode::Chaos] {
        let d = decide(
            mode,
            "email",
            1.0,
            t(),
            Some("j@e.com"),
            true,
            Some(UserDecision::Rejected),
        );
        check_eq(
            d,
            FillDecision::Skip(SkipReason::UserPreviouslyRejected),
            &format!("mode {mode:?} did not honor prior Rejected"),
        )?;
    }
    Ok(())
}

fn decide_overlay_accepted_skips_prompt_in_training_wheels() -> Result<(), String> {
    let d = decide(
        Mode::TrainingWheels,
        "email",
        1.0,
        t(),
        Some("j@e.com"),
        true,
        Some(UserDecision::Accepted),
    );
    check_eq(
        d,
        FillDecision::Fill { value: "j@e.com" },
        "TrainingWheels skips prompt when Accepted",
    )
}

fn decide_overlay_accepted_overrides_shadow_threshold() -> Result<(), String> {
    let d = decide(
        Mode::Shadow,
        "email",
        0.10,
        t(),
        Some("j@e.com"),
        true,
        Some(UserDecision::Accepted),
    );
    check_eq(
        d,
        FillDecision::Fill { value: "j@e.com" },
        "Shadow Accepted overrides threshold",
    )
}

fn decide_overlay_does_not_short_circuit_skip_precedence() -> Result<(), String> {
    let d = decide(
        Mode::Chaos,
        "linkedin",
        1.0,
        t(),
        None,
        true,
        Some(UserDecision::Accepted),
    );
    check_eq(
        d,
        FillDecision::Skip(SkipReason::NoProfileValue),
        "Accepted does not manufacture value",
    )
}

fn format_summary_shows_screenshot_path_when_set() -> Result<(), String> {
    let s = RunSummary {
        url: "u".into(),
        fields_seen: 0,
        fields_filled: 0,
        fields_skipped: 0,
        screenshot_path: Some(std::path::PathBuf::from("/tmp/run-1234.png")),
        feedback_events: vec![],
    };
    let out = format_summary(&s, 0);
    check(out.contains("/tmp/run-1234.png"), "screenshot path included")
}
