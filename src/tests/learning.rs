// SPDX-License-Identifier: Unlicense

//! Tests for `crate::learning` (Phase 5).

use crate::learning::{fingerprint, CorrectionOverlay, FeedbackStats, UserDecision};
use crate::{Feedback, FieldDescriptor};

use super::{case, check, check_eq, TestResult};

pub fn run() -> Vec<TestResult> {
    vec![
        case("learning::fingerprint_ignores_id_for_stability", fingerprint_ignores_id_for_stability),
        case("learning::fingerprint_normalizes_whitespace_and_case",
             fingerprint_normalizes_whitespace_and_case),
        case("learning::overlay_records_rejection", overlay_records_rejection),
        case("learning::overlay_records_every_accepted_event", overlay_records_every_accepted_event),
        case("learning::overlay_records_user_correction_as_accepted",
             overlay_records_user_correction_as_accepted),
        case("learning::overlay_most_recent_wins", overlay_most_recent_wins),
        case("learning::overlay_decision_for_unknown_field_returns_none",
             overlay_decision_for_unknown_field_returns_none),
        case("learning::stats_count_accepted_rejected", stats_count_accepted_rejected),
        case("learning::stats_top_rejected_keys_sorted_desc", stats_top_rejected_keys_sorted_desc),
        case("learning::stats_top_rejected_capped_at_five", stats_top_rejected_capped_at_five),
    ]
}

fn fb(label: &str, predicted: &str, actual: &str, accepted: bool) -> Feedback {
    Feedback {
        field: FieldDescriptor {
            label: label.into(),
            placeholder: "".into(),
            aria_label: "".into(),
            name: "".into(),
            id: "".into(),
            kind: "text".into(),
        },
        predicted: predicted.into(),
        actual: actual.into(),
        accepted,
    }
}

fn fingerprint_ignores_id_for_stability() -> Result<(), String> {
    // Workday-style dynamic id MUST NOT affect the fingerprint.
    let d1 = FieldDescriptor {
        label: "Email".into(),
        placeholder: "".into(),
        aria_label: "".into(),
        name: "primaryEmail".into(),
        id: "input-1234567890-abc".into(),
        kind: "email".into(),
    };
    let d2 = FieldDescriptor {
        id: "input-9999999-xyz".into(),
        ..d1.clone()
    };
    check_eq(fingerprint(&d1), fingerprint(&d2), "fingerprint should ignore id")
}

fn fingerprint_normalizes_whitespace_and_case() -> Result<(), String> {
    let d1 = FieldDescriptor {
        label: "  Email  ".into(),
        placeholder: "".into(),
        aria_label: "".into(),
        name: "EMAIL".into(),
        id: "".into(),
        kind: "Email".into(),
    };
    let d2 = FieldDescriptor {
        label: "email".into(),
        placeholder: "".into(),
        aria_label: "".into(),
        name: "email".into(),
        id: "".into(),
        kind: "email".into(),
    };
    check_eq(fingerprint(&d1), fingerprint(&d2), "fingerprint should normalize")
}

fn overlay_records_rejection() -> Result<(), String> {
    let q = vec![fb("Salary expectation", "freetext", "", false)];
    let ov = CorrectionOverlay::from_queue(&q);
    let d = FieldDescriptor {
        label: "Salary expectation".into(),
        placeholder: "".into(),
        aria_label: "".into(),
        name: "".into(),
        id: "anything".into(),
        kind: "text".into(),
    };
    check_eq(ov.decision_for(&d), Some(UserDecision::Rejected), "rejection recorded")
}

fn overlay_records_every_accepted_event() -> Result<(), String> {
    let q = vec![fb("Email", "email", "email", true)];
    let ov = CorrectionOverlay::from_queue(&q);
    let d = FieldDescriptor {
        label: "Email".into(),
        placeholder: "".into(),
        aria_label: "".into(),
        name: "".into(),
        id: "".into(),
        kind: "text".into(),
    };
    check_eq(ov.decision_for(&d), Some(UserDecision::Accepted), "accepted recorded")
}

fn overlay_records_user_correction_as_accepted() -> Result<(), String> {
    // User corrected the key (predicted ≠ actual, not empty) →
    // still accepted=true, still Accepted in overlay.
    let q = vec![fb("Email", "email", "first_name", true)];
    let ov = CorrectionOverlay::from_queue(&q);
    let d = FieldDescriptor {
        label: "Email".into(),
        placeholder: "".into(),
        aria_label: "".into(),
        name: "".into(),
        id: "".into(),
        kind: "text".into(),
    };
    check_eq(
        ov.decision_for(&d),
        Some(UserDecision::Accepted),
        "user correction is Accepted",
    )
}

fn overlay_most_recent_wins() -> Result<(), String> {
    // User said no last week, yes today on the same field shape.
    let q = vec![
        fb("Email", "email", "", false),
        fb("Email", "email", "first_name", true),
    ];
    let ov = CorrectionOverlay::from_queue(&q);
    let d = FieldDescriptor {
        label: "Email".into(),
        placeholder: "".into(),
        aria_label: "".into(),
        name: "".into(),
        id: "".into(),
        kind: "text".into(),
    };
    check_eq(ov.decision_for(&d), Some(UserDecision::Accepted), "most recent wins")
}

fn overlay_decision_for_unknown_field_returns_none() -> Result<(), String> {
    let ov = CorrectionOverlay::default();
    let d = FieldDescriptor {
        label: "Anything".into(),
        placeholder: "".into(),
        aria_label: "".into(),
        name: "".into(),
        id: "".into(),
        kind: "text".into(),
    };
    check(ov.decision_for(&d).is_none(), "unknown field → None")
}

fn stats_count_accepted_rejected() -> Result<(), String> {
    let q = vec![
        fb("a", "email", "email", true),
        fb("b", "phone", "", false),
        fb("c", "linkedin", "linkedin", true),
        fb("d", "freetext", "", false),
        fb("e", "freetext", "", false),
    ];
    let s = FeedbackStats::from_queue(&q);
    check_eq(s.total, 5usize, "total")?;
    check_eq(s.accepted, 2usize, "accepted")?;
    check_eq(s.rejected, 3usize, "rejected")
}

fn stats_top_rejected_keys_sorted_desc() -> Result<(), String> {
    let q = vec![
        fb("a", "freetext", "", false),
        fb("b", "freetext", "", false),
        fb("c", "phone", "", false),
        fb("d", "freetext", "", false),
    ];
    let s = FeedbackStats::from_queue(&q);
    // freetext (3) before phone (1)
    check_eq(s.top_rejected_keys[0].clone(), ("freetext".to_string(), 3usize), "first")?;
    check_eq(s.top_rejected_keys[1].clone(), ("phone".to_string(), 1usize), "second")
}

fn stats_top_rejected_capped_at_five() -> Result<(), String> {
    let q: Vec<Feedback> = (0..10)
        .map(|i| fb("x", &format!("k{i}"), "", false))
        .collect();
    let s = FeedbackStats::from_queue(&q);
    check_eq(s.top_rejected_keys.len(), 5usize, "top rejected capped")
}
