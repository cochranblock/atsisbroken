// SPDX-License-Identifier: Unlicense

//! Tests for `crate` (`lib.rs`) — Phase 8.
//!
//! Largest single test surface in the crate (~126 tests). Covers:
//! - Profile + sub-struct serde round-trips and on-disk JSON shape
//! - predict_field_key classifier rules
//! - profile_value_for_key resolution
//! - Mode / ConfidenceThreshold / Observation
//! - FeedbackQueue jsonl I/O
//! - Seed corpus parse + coverage + fingerprint determinism
//! - In-process triple_sims determinism check

use crate::{
    parse_seed_corpus, predict_field_key, predict_field_key_with_confidence,
    profile_value_for_key, seed_corpus_fingerprint, seed_corpus_size, Application,
    ApplicationStatus, Award, Certification, Compensation, ConfidenceThreshold, Demographics,
    Education, Experience, Feedback, FeedbackDelivery, FeedbackQueue, FieldDescriptor,
    FreeFormAnswers, Language, LanguageProficiency, Mode, Observation, OnlinePresence, Patent,
    Profile, Project, Publication, Reference, RemotePreference, RepoRef, SecurityClearance,
    Skill, SkillLevel, TrainingPair, WorkAuth, WorkAuthStatus, SEED_CORPUS_JSONL,
};

use super::{case, check, check_eq, TestResult};

pub fn run() -> Vec<TestResult> {
    vec![
        // Seed corpus
        case("lib::seed_corpus_is_nonempty", seed_corpus_is_nonempty),
        case("lib::seed_corpus_parses", seed_corpus_parses),
        case("lib::seed_corpus_fingerprint_is_deterministic", seed_corpus_fingerprint_is_deterministic),
        case("lib::seed_corpus_covers_email", seed_corpus_covers_email),
        case("lib::seed_corpus_covers_phone", seed_corpus_covers_phone),
        case("lib::seed_corpus_covers_full_name", seed_corpus_covers_full_name),
        case("lib::seed_corpus_covers_linkedin", seed_corpus_covers_linkedin),
        case("lib::seed_corpus_covers_github", seed_corpus_covers_github),
        case("lib::seed_corpus_covers_website", seed_corpus_covers_website),
        case("lib::seed_corpus_covers_address", seed_corpus_covers_address),
        case("lib::seed_corpus_covers_work_authorization", seed_corpus_covers_work_authorization),
        case("lib::seed_corpus_covers_years_experience", seed_corpus_covers_years_experience),
        case("lib::seed_corpus_covers_freetext_sentinel", seed_corpus_covers_freetext_sentinel),
        case("lib::seed_corpus_covers_unknown_sentinel", seed_corpus_covers_unknown_sentinel),
        case("lib::seed_corpus_has_no_duplicate_descriptors", seed_corpus_has_no_duplicate_descriptors),
        case("lib::seed_corpus_jsonl_has_no_trailing_or_leading_whitespace_per_line",
             seed_corpus_jsonl_has_no_trailing_or_leading_whitespace_per_line),
        case("lib::parse_seed_corpus_propagates_malformed_line_error",
             parse_seed_corpus_propagates_malformed_line_error),
        case("lib::seed_corpus_covers_at_least_two_kinds", seed_corpus_covers_at_least_two_kinds),
        case("lib::seed_corpus_no_empty_label_and_name", seed_corpus_no_empty_label_and_name),
        case("lib::seed_corpus_expected_keys_are_known_vocab", seed_corpus_expected_keys_are_known_vocab),
        // Profile basics
        case("lib::profile_round_trip", profile_round_trip),
        case("lib::profile_default_is_empty", profile_default_is_empty),
        case("lib::is_meaningfully_populated_returns_false_for_empty",
             is_meaningfully_populated_returns_false_for_empty),
        case("lib::is_meaningfully_populated_false_with_only_one_field",
             is_meaningfully_populated_false_with_only_one_field),
        case("lib::is_meaningfully_populated_true_with_two_fields",
             is_meaningfully_populated_true_with_two_fields),
        case("lib::is_meaningfully_populated_counts_only_identity_signals",
             is_meaningfully_populated_counts_only_identity_signals),
        case("lib::profile_toml_accepts_partial_input", profile_toml_accepts_partial_input),
        // Mode + threshold
        case("lib::mode_default_is_training_wheels", mode_default_is_training_wheels),
        case("lib::mode_serializes_snake_case", mode_serializes_snake_case),
        case("lib::mode_from_cli_str_accepts_all_documented_spellings",
             mode_from_cli_str_accepts_all_documented_spellings),
        case("lib::mode_from_cli_str_rejects_unknown_and_typos",
             mode_from_cli_str_rejects_unknown_and_typos),
        case("lib::confidence_threshold_default_is_conservative", confidence_threshold_default_is_conservative),
        case("lib::confidence_threshold_gate", confidence_threshold_gate),
        case("lib::shadow_mode_gate_only_promotes_when_confident",
             shadow_mode_gate_only_promotes_when_confident),
        // FeedbackDelivery
        case("lib::feedback_delivery_default_is_local_only", feedback_delivery_default_is_local_only),
        case("lib::feedback_delivery_json_shapes_are_stable", feedback_delivery_json_shapes_are_stable),
        // Feedback / Observation / FieldDescriptor wire shape
        case("lib::feedback_queue_jsonl_round_trip", feedback_queue_jsonl_round_trip),
        case("lib::feedback_json_shape_is_stable", feedback_json_shape_is_stable),
        case("lib::observation_json_shape_is_stable", observation_json_shape_is_stable),
        case("lib::field_descriptor_json_shape_is_stable", field_descriptor_json_shape_is_stable),
        // FeedbackQueue
        case("lib::feedback_queue_jsonl_skips_blank_lines", feedback_queue_jsonl_skips_blank_lines),
        case("lib::feedback_queue_empty_jsonl_parses", feedback_queue_empty_jsonl_parses),
        case("lib::feedback_queue_jsonl_is_one_line_per_event", feedback_queue_jsonl_is_one_line_per_event),
        case("lib::feedback_queue_load_missing_file_is_empty", feedback_queue_load_missing_file_is_empty),
        case("lib::feedback_queue_save_then_load_round_trip", feedback_queue_save_then_load_round_trip),
        case("lib::feedback_queue_preserves_event_order_across_save_load",
             feedback_queue_preserves_event_order_across_save_load),
        case("lib::feedback_queue_save_idempotent_overwrites_prior_content",
             feedback_queue_save_idempotent_overwrites_prior_content),
        case("lib::feedback_queue_save_atomically_via_tmp", feedback_queue_save_atomically_via_tmp),
        case("lib::feedback_queue_corrupt_line_fails_loudly", feedback_queue_corrupt_line_fails_loudly),
        // predict_field_key
        case("lib::predict_email_via_label", predict_email_via_label),
        case("lib::predict_email_via_placeholder", predict_email_via_placeholder),
        case("lib::predict_phone_variants", predict_phone_variants),
        case("lib::predict_linkedin_github_website", predict_linkedin_github_website),
        case("lib::predict_address_variants", predict_address_variants),
        case("lib::predict_address_does_not_match_address_inside_unrelated_words",
             predict_address_does_not_match_address_inside_unrelated_words),
        case("lib::predict_work_authorization_variants", predict_work_authorization_variants),
        case("lib::predict_years_experience", predict_years_experience),
        case("lib::predict_name_disambiguation", predict_name_disambiguation),
        case("lib::predict_textarea_routes_to_freetext", predict_textarea_routes_to_freetext),
        case("lib::predict_unknown_returns_empty_not_a_guess", predict_unknown_returns_empty_not_a_guess),
        case("lib::predict_does_not_match_tel_inside_unrelated_words",
             predict_does_not_match_tel_inside_unrelated_words),
        case("lib::predict_phone_via_html_kind_tel", predict_phone_via_html_kind_tel),
        case("lib::predict_email_via_html_kind_email", predict_email_via_html_kind_email),
        case("lib::predict_email_outranks_name_when_both_present",
             predict_email_outranks_name_when_both_present),
        case("lib::predict_with_confidence_returns_one_for_known", predict_with_confidence_returns_one_for_known),
        case("lib::predict_with_confidence_returns_zero_for_unknown",
             predict_with_confidence_returns_zero_for_unknown),
        case("lib::predict_gitlab_routes_to_gitlab", predict_gitlab_routes_to_gitlab),
        case("lib::predict_bitbucket_routes_to_bitbucket", predict_bitbucket_routes_to_bitbucket),
        case("lib::predict_twitter_routes_to_twitter", predict_twitter_routes_to_twitter),
        case("lib::predict_bluesky_routes_to_bluesky", predict_bluesky_routes_to_bluesky),
        case("lib::predict_mastodon_routes_to_mastodon", predict_mastodon_routes_to_mastodon),
        case("lib::predict_stackoverflow_routes_to_stackoverflow", predict_stackoverflow_routes_to_stackoverflow),
        case("lib::predict_blog_routes_to_blog", predict_blog_routes_to_blog),
        case("lib::predict_blog_does_not_match_inside_unrelated_words",
             predict_blog_does_not_match_inside_unrelated_words),
        case("lib::predict_salary_expectation_routes_to_salary_expectation",
             predict_salary_expectation_routes_to_salary_expectation),
        case("lib::predict_bare_compensation_does_not_match_salary",
             predict_bare_compensation_does_not_match_salary),
        // profile_value_for_key
        case("lib::profile_value_for_key_resolves_new_presence_keys",
             profile_value_for_key_resolves_new_presence_keys),
        case("lib::profile_value_for_key_portfolio_falls_back_to_website",
             profile_value_for_key_portfolio_falls_back_to_website),
        case("lib::profile_value_for_key_empty_presence_returns_none",
             profile_value_for_key_empty_presence_returns_none),
        case("lib::profile_value_for_key_salary_expectation_returns_none",
             profile_value_for_key_salary_expectation_returns_none),
        case("lib::profile_value_for_key_empty_field_returns_none",
             profile_value_for_key_empty_field_returns_none),
        case("lib::profile_value_for_key_known_keys", profile_value_for_key_known_keys),
        case("lib::profile_value_for_key_unknown_keys_return_none",
             profile_value_for_key_unknown_keys_return_none),
        // Phase G schema round-trips
        case("lib::experience_json_shape_is_stable", experience_json_shape_is_stable),
        case("lib::education_json_shape_is_stable", education_json_shape_is_stable),
        case("lib::profile_v0_toml_loads_with_phase_g_defaults",
             profile_v0_toml_loads_with_phase_g_defaults),
        case("lib::profile_v1_full_round_trip_preserves_all_phase_g_fields",
             profile_v1_full_round_trip_preserves_all_phase_g_fields),
        case("lib::online_presence_json_shape_is_stable", online_presence_json_shape_is_stable),
        case("lib::skill_json_shape_is_stable", skill_json_shape_is_stable),
        case("lib::skill_level_serializes_snake_case", skill_level_serializes_snake_case),
        case("lib::skill_default_level_is_intermediate", skill_default_level_is_intermediate),
        case("lib::language_json_shape_is_stable", language_json_shape_is_stable),
        case("lib::language_proficiency_serializes_snake_case", language_proficiency_serializes_snake_case),
        case("lib::certification_json_shape_is_stable", certification_json_shape_is_stable),
        case("lib::project_with_github_repo_json_shape_is_stable",
             project_with_github_repo_json_shape_is_stable),
        case("lib::project_without_github_repo_serializes_repo_as_null",
             project_without_github_repo_serializes_repo_as_null),
        case("lib::compensation_json_shape_is_stable", compensation_json_shape_is_stable),
        case("lib::demographics_default_is_all_empty", demographics_default_is_all_empty),
        case("lib::profile_demographics_default_is_none_not_empty_struct",
             profile_demographics_default_is_none_not_empty_struct),
        case("lib::profile_compensation_default_is_none", profile_compensation_default_is_none),
        case("lib::repo_ref_round_trip", repo_ref_round_trip),
        case("lib::experience_v0_toml_loads_with_phase_g_defaults",
             experience_v0_toml_loads_with_phase_g_defaults),
        case("lib::education_v0_toml_loads_with_phase_g_defaults",
             education_v0_toml_loads_with_phase_g_defaults),
        // Phase G Tier 2
        case("lib::profile_tier2_defaults_are_inert", profile_tier2_defaults_are_inert),
        case("lib::profile_v0_toml_loads_with_tier2_defaults", profile_v0_toml_loads_with_tier2_defaults),
        case("lib::work_auth_status_serializes_snake_case", work_auth_status_serializes_snake_case),
        case("lib::work_auth_status_default_is_empty_other_escape_hatch",
             work_auth_status_default_is_empty_other_escape_hatch),
        case("lib::security_clearance_none_distinct_from_unspecified",
             security_clearance_none_distinct_from_unspecified),
        case("lib::remote_preference_default_is_flexible", remote_preference_default_is_flexible),
        case("lib::remote_preference_serializes_snake_case", remote_preference_serializes_snake_case),
        case("lib::application_status_default_is_submitted", application_status_default_is_submitted),
        case("lib::application_status_serializes_snake_case", application_status_serializes_snake_case),
        case("lib::work_auth_json_shape_is_stable", work_auth_json_shape_is_stable),
        case("lib::publication_json_shape_is_stable", publication_json_shape_is_stable),
        case("lib::patent_json_shape_is_stable", patent_json_shape_is_stable),
        case("lib::award_json_shape_is_stable", award_json_shape_is_stable),
        case("lib::reference_json_shape_is_stable", reference_json_shape_is_stable),
        case("lib::application_json_shape_is_stable", application_json_shape_is_stable),
        case("lib::free_form_answers_json_shape_is_stable", free_form_answers_json_shape_is_stable),
        case("lib::free_form_none_vs_empty_some_are_distinct_signals",
             free_form_none_vs_empty_some_are_distinct_signals),
        case("lib::application_status_round_trip", application_status_round_trip),
        case("lib::work_auth_status_round_trip_including_other", work_auth_status_round_trip_including_other),
        case("lib::profile_v1_tier2_full_round_trip", profile_v1_tier2_full_round_trip),
        // Triple sims determinism
        case("lib::triple_sims_determinism", triple_sims_determinism),
    ]
}

// ─── helpers ─────────────────────────────────────────────────────────

fn fd(label: &str, placeholder: &str, aria: &str, name: &str, id: &str, kind: &str) -> FieldDescriptor {
    FieldDescriptor {
        label: label.into(),
        placeholder: placeholder.into(),
        aria_label: aria.into(),
        name: name.into(),
        id: id.into(),
        kind: kind.into(),
    }
}

fn sample_fb(label: &str, predicted: &str, actual: &str, accepted: bool) -> Feedback {
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

fn unique_tmp(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "atsisbroken_lib_{tag}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ))
}

// ─── seed corpus tests ───────────────────────────────────────────────

fn seed_corpus_is_nonempty() -> Result<(), String> {
    check(seed_corpus_size() > 0, "seed corpus size > 0")
}

fn seed_corpus_parses() -> Result<(), String> {
    let pairs = parse_seed_corpus().map_err(|e| format!("{e}"))?;
    check(pairs.len() >= 20, format!("seed corpus too small: {}", pairs.len()))?;
    for p in &pairs {
        check(!p.expected.is_empty(), "every pair has non-empty expected")?;
    }
    Ok(())
}

fn seed_corpus_fingerprint_is_deterministic() -> Result<(), String> {
    let h1 = seed_corpus_fingerprint();
    let h2 = seed_corpus_fingerprint();
    let h3 = seed_corpus_fingerprint();
    check_eq(h1, h2, "fingerprint stable 1→2")?;
    check_eq(h2, h3, "fingerprint stable 2→3")
}

fn corpus_has(expected: &str) -> Result<(), String> {
    let pairs = parse_seed_corpus().map_err(|e| format!("{e}"))?;
    check(
        pairs.iter().any(|p| p.expected == expected),
        format!("corpus should cover {expected:?}"),
    )
}

fn seed_corpus_covers_email() -> Result<(), String> { corpus_has("email") }
fn seed_corpus_covers_phone() -> Result<(), String> { corpus_has("phone") }
fn seed_corpus_covers_full_name() -> Result<(), String> { corpus_has("full_name") }
fn seed_corpus_covers_linkedin() -> Result<(), String> { corpus_has("linkedin") }
fn seed_corpus_covers_github() -> Result<(), String> { corpus_has("github") }
fn seed_corpus_covers_website() -> Result<(), String> { corpus_has("website") }
fn seed_corpus_covers_address() -> Result<(), String> { corpus_has("address") }
fn seed_corpus_covers_work_authorization() -> Result<(), String> { corpus_has("work_authorization") }
fn seed_corpus_covers_years_experience() -> Result<(), String> { corpus_has("years_experience") }
fn seed_corpus_covers_freetext_sentinel() -> Result<(), String> { corpus_has("freetext") }
fn seed_corpus_covers_unknown_sentinel() -> Result<(), String> { corpus_has("unknown") }

fn seed_corpus_has_no_duplicate_descriptors() -> Result<(), String> {
    use std::collections::HashSet;
    let pairs = parse_seed_corpus().map_err(|e| format!("{e}"))?;
    let mut seen: HashSet<String> = HashSet::new();
    for p in &pairs {
        let key = format!(
            "{}|{}|{}|{}|{}",
            p.field.label, p.field.placeholder, p.field.aria_label, p.field.name, p.field.id
        );
        check(
            seen.insert(key.clone()),
            format!("duplicate descriptor in seed corpus: {key}"),
        )?;
    }
    Ok(())
}

fn seed_corpus_jsonl_has_no_trailing_or_leading_whitespace_per_line() -> Result<(), String> {
    for line in SEED_CORPUS_JSONL.lines() {
        if line.is_empty() {
            continue;
        }
        check_eq(line, line.trim(), &format!("seed corpus line has whitespace edges: {line:?}"))?;
    }
    Ok(())
}

fn parse_seed_corpus_propagates_malformed_line_error() -> Result<(), String> {
    let injected = format!("{SEED_CORPUS_JSONL}\n{{this is not json}}\n");
    let result: Result<Vec<TrainingPair>, _> = injected
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(serde_json::from_str)
        .collect();
    check(result.is_err(), "malformed line must error")
}

fn seed_corpus_covers_at_least_two_kinds() -> Result<(), String> {
    let pairs = parse_seed_corpus().map_err(|e| format!("{e}"))?;
    let mut kinds: std::collections::HashSet<String> = Default::default();
    for p in &pairs {
        kinds.insert(p.field.kind.clone());
    }
    check(kinds.len() >= 2, format!("kinds: {kinds:?}"))
}

fn seed_corpus_no_empty_label_and_name() -> Result<(), String> {
    let pairs = parse_seed_corpus().map_err(|e| format!("{e}"))?;
    for (i, p) in pairs.iter().enumerate() {
        let any = !p.field.label.is_empty()
            || !p.field.placeholder.is_empty()
            || !p.field.aria_label.is_empty()
            || !p.field.name.is_empty()
            || !p.field.id.is_empty();
        check(any, format!("row {i} has no descriptor signal"))?;
    }
    Ok(())
}

fn seed_corpus_expected_keys_are_known_vocab() -> Result<(), String> {
    let allowed = [
        "email",
        "phone",
        "full_name",
        "linkedin",
        "github",
        "website",
        "address",
        "work_authorization",
        "years_experience",
        "freetext",
        "unknown",
    ];
    let pairs = parse_seed_corpus().map_err(|e| format!("{e}"))?;
    for p in &pairs {
        check(
            allowed.contains(&p.expected.as_str()),
            format!("out-of-vocab key in corpus: {:?}", p.expected),
        )?;
    }
    Ok(())
}

// ─── Profile basics ──────────────────────────────────────────────────

fn profile_round_trip() -> Result<(), String> {
    let p = Profile {
        full_name: "Jane Doe".into(),
        email: "jane@example.com".into(),
        phone: "+1-555-0100".into(),
        years_experience: 7,
        skills: vec!["rust".into(), "ml".into()],
        ..Default::default()
    };
    let toml_text = toml::to_string(&p).map_err(|e| format!("{e}"))?;
    let back: Profile = toml::from_str(&toml_text).map_err(|e| format!("{e}"))?;
    check_eq(p, back, "profile round-trip")
}

fn profile_default_is_empty() -> Result<(), String> {
    let p = Profile::default();
    check(p.full_name.is_empty(), "full_name empty")?;
    check(p.email.is_empty(), "email empty")?;
    check_eq(p.years_experience, 0u8, "years_experience")?;
    check(p.skills.is_empty(), "skills empty")?;
    check(p.experience.is_empty(), "experience empty")
}

fn is_meaningfully_populated_returns_false_for_empty() -> Result<(), String> {
    check(!Profile::default().is_meaningfully_populated(), "default not populated")
}

fn is_meaningfully_populated_false_with_only_one_field() -> Result<(), String> {
    let p = Profile {
        full_name: "Jane".into(),
        ..Default::default()
    };
    check(!p.is_meaningfully_populated(), "one field is not enough")
}

fn is_meaningfully_populated_true_with_two_fields() -> Result<(), String> {
    let p = Profile {
        full_name: "Jane Doe".into(),
        email: "jane@example.com".into(),
        ..Default::default()
    };
    check(p.is_meaningfully_populated(), "two fields are enough")
}

fn is_meaningfully_populated_counts_only_identity_signals() -> Result<(), String> {
    let p = Profile {
        address: "1 Main St".into(),
        years_experience: 7,
        skills: vec!["rust".into(), "ml".into()],
        ..Default::default()
    };
    check(!p.is_meaningfully_populated(), "non-identity signals don't count")
}

fn profile_toml_accepts_partial_input() -> Result<(), String> {
    let toml_text = r#"
        full_name = "Jane Doe"
        email    = "jane@example.com"
        phone    = ""
        address  = ""
        linkedin = ""
        github   = ""
        website  = ""
        work_authorization = ""
        years_experience   = 0
        raw_resume_text    = ""
        experience = []
        education  = []
        skills     = []
    "#;
    let p: Profile = toml::from_str(toml_text).map_err(|e| format!("{e}"))?;
    check_eq(p.full_name, "Jane Doe".to_string(), "full_name")?;
    check_eq(p.email, "jane@example.com".to_string(), "email")
}

// ─── Mode + threshold ────────────────────────────────────────────────

fn mode_default_is_training_wheels() -> Result<(), String> {
    check_eq(Mode::default(), Mode::TrainingWheels, "default Mode")
}

fn mode_serializes_snake_case() -> Result<(), String> {
    check_eq(serde_json::to_string(&Mode::TrainingWheels).map_err(|e| format!("{e}"))?, "\"training_wheels\"".to_string(), "TrainingWheels")?;
    check_eq(serde_json::to_string(&Mode::Shadow).map_err(|e| format!("{e}"))?, "\"shadow\"".to_string(), "Shadow")?;
    check_eq(serde_json::to_string(&Mode::Chaos).map_err(|e| format!("{e}"))?, "\"chaos\"".to_string(), "Chaos")
}

fn mode_from_cli_str_accepts_all_documented_spellings() -> Result<(), String> {
    check_eq(Mode::from_cli_str("training-wheels"), Some(Mode::TrainingWheels), "training-wheels")?;
    check_eq(Mode::from_cli_str("training_wheels"), Some(Mode::TrainingWheels), "training_wheels")?;
    check_eq(Mode::from_cli_str("training"), Some(Mode::TrainingWheels), "training")?;
    check_eq(Mode::from_cli_str("shadow"), Some(Mode::Shadow), "shadow")?;
    check_eq(Mode::from_cli_str("chaos"), Some(Mode::Chaos), "chaos")
}

fn mode_from_cli_str_rejects_unknown_and_typos() -> Result<(), String> {
    check_eq(Mode::from_cli_str(""), None, "empty")?;
    check_eq(Mode::from_cli_str("Chaos"), None, "case-sensitive")?;
    check_eq(Mode::from_cli_str("training wheels"), None, "space")?;
    check_eq(Mode::from_cli_str("shadows"), None, "shadows")?;
    check_eq(Mode::from_cli_str("yolo"), None, "yolo")
}

fn confidence_threshold_default_is_conservative() -> Result<(), String> {
    let t = ConfidenceThreshold::default();
    check(t.0 > 0.5 && t.0 < 1.0, format!("threshold: {}", t.0))
}

fn confidence_threshold_gate() -> Result<(), String> {
    let t = ConfidenceThreshold(0.85);
    check(t.passes(0.85), "0.85 passes (>=)")?;
    check(t.passes(0.90), "0.90 passes")?;
    check(t.passes(1.0), "1.0 passes")?;
    check(!t.passes(0.84), "0.84 fails")?;
    check(!t.passes(0.0), "0.0 fails")
}

fn shadow_mode_gate_only_promotes_when_confident() -> Result<(), String> {
    let t = ConfidenceThreshold::default();
    let stream: Vec<f32> = vec![0.42, 0.58, 0.71, 0.79, 0.83, 0.86, 0.90];
    let mut promoted_at: Option<usize> = None;
    for (i, c) in stream.iter().enumerate() {
        if t.passes(*c) {
            promoted_at = Some(i);
            break;
        }
    }
    check_eq(promoted_at, Some(5), "promoted at index 5")
}

// ─── FeedbackDelivery ─────────────────────────────────────────────────

fn feedback_delivery_default_is_local_only() -> Result<(), String> {
    check_eq(FeedbackDelivery::default(), FeedbackDelivery::LocalOnly, "default delivery")
}

fn feedback_delivery_json_shapes_are_stable() -> Result<(), String> {
    let local = serde_json::to_string(&FeedbackDelivery::LocalOnly).map_err(|e| format!("{e}"))?;
    check_eq(local, r#"{"kind":"local_only"}"#.to_string(), "LocalOnly shape")?;
    let online = serde_json::to_string(&FeedbackDelivery::SendWhenOnline {
        destination: "mailto:me@example.com".into(),
    })
    .map_err(|e| format!("{e}"))?;
    check_eq(
        online,
        r#"{"kind":"send_when_online","destination":"mailto:me@example.com"}"#.to_string(),
        "SendWhenOnline shape",
    )
}

// ─── Feedback / Observation / FieldDescriptor wire shape ─────────────

fn feedback_queue_jsonl_round_trip() -> Result<(), String> {
    let mut q = FeedbackQueue::default();
    q.append(Feedback {
        field: FieldDescriptor {
            label: "Email".into(),
            placeholder: "".into(),
            aria_label: "".into(),
            name: "email".into(),
            id: "".into(),
            kind: "email".into(),
        },
        predicted: "email".into(),
        actual: "email".into(),
        accepted: true,
    });
    q.append(Feedback {
        field: FieldDescriptor {
            label: "Why this role?".into(),
            placeholder: "".into(),
            aria_label: "".into(),
            name: "why".into(),
            id: "".into(),
            kind: "textarea".into(),
        },
        predicted: "freetext".into(),
        actual: "skip".into(),
        accepted: false,
    });
    let jsonl = q.to_jsonl().map_err(|e| format!("{e}"))?;
    let back = FeedbackQueue::from_jsonl(&jsonl).map_err(|e| format!("{e}"))?;
    check_eq(q.clone(), back.clone(), "queue round-trip")?;
    check_eq(back.len(), 2usize, "queue length")
}

fn feedback_json_shape_is_stable() -> Result<(), String> {
    let fb = Feedback {
        field: FieldDescriptor {
            label: "Email".into(),
            placeholder: "".into(),
            aria_label: "".into(),
            name: "email".into(),
            id: "".into(),
            kind: "email".into(),
        },
        predicted: "email".into(),
        actual: "email".into(),
        accepted: true,
    };
    let got = serde_json::to_string(&fb).map_err(|e| format!("{e}"))?;
    let want = r#"{"field":{"label":"Email","placeholder":"","aria_label":"","name":"email","id":"","kind":"email"},"predicted":"email","actual":"email","accepted":true}"#;
    check_eq(got, want.to_string(), "feedback wire shape")
}

fn observation_json_shape_is_stable() -> Result<(), String> {
    let o = Observation {
        field: FieldDescriptor {
            label: "Email".into(),
            placeholder: "".into(),
            aria_label: "".into(),
            name: "email".into(),
            id: "".into(),
            kind: "email".into(),
        },
        predicted: "email".into(),
        observed: "email".into(),
        confidence: 0.92,
    };
    let got = serde_json::to_string(&o).map_err(|e| format!("{e}"))?;
    let want = r#"{"field":{"label":"Email","placeholder":"","aria_label":"","name":"email","id":"","kind":"email"},"predicted":"email","observed":"email","confidence":0.92}"#;
    check_eq(got, want.to_string(), "observation wire shape")
}

fn field_descriptor_json_shape_is_stable() -> Result<(), String> {
    let f = FieldDescriptor {
        label: "Email".into(),
        placeholder: "you@example.com".into(),
        aria_label: "Email address".into(),
        name: "email".into(),
        id: "input-email".into(),
        kind: "email".into(),
    };
    let got = serde_json::to_string(&f).map_err(|e| format!("{e}"))?;
    let want = r#"{"label":"Email","placeholder":"you@example.com","aria_label":"Email address","name":"email","id":"input-email","kind":"email"}"#;
    check_eq(got, want.to_string(), "field_descriptor wire shape")
}

// ─── FeedbackQueue I/O ───────────────────────────────────────────────

fn feedback_queue_jsonl_skips_blank_lines() -> Result<(), String> {
    let jsonl = "\n\n{\"field\":{\"label\":\"Email\",\"placeholder\":\"\",\"aria_label\":\"\",\"name\":\"\",\"id\":\"\",\"kind\":\"text\"},\"predicted\":\"email\",\"actual\":\"email\",\"accepted\":true}\n\n";
    let q = FeedbackQueue::from_jsonl(jsonl).map_err(|e| format!("{e}"))?;
    check_eq(q.len(), 1usize, "blank lines skipped")
}

fn feedback_queue_empty_jsonl_parses() -> Result<(), String> {
    let q = FeedbackQueue::from_jsonl("").map_err(|e| format!("{e}"))?;
    check(q.is_empty(), "empty jsonl")?;
    let q = FeedbackQueue::from_jsonl("\n\n\n").map_err(|e| format!("{e}"))?;
    check(q.is_empty(), "newlines-only jsonl")
}

fn feedback_queue_jsonl_is_one_line_per_event() -> Result<(), String> {
    let mut q = FeedbackQueue::default();
    q.append(sample_fb("a", "email", "email", true));
    q.append(sample_fb("b", "phone", "phone", true));
    q.append(sample_fb("c", "freetext", "skip", false));
    let jsonl = q.to_jsonl().map_err(|e| format!("{e}"))?;
    let lines: Vec<&str> = jsonl.lines().collect();
    check_eq(lines.len(), 3usize, "one line per event")
}

fn feedback_queue_load_missing_file_is_empty() -> Result<(), String> {
    let dir = unique_tmp("load_missing");
    let path = dir.join("nope.jsonl");
    let q = FeedbackQueue::load_from(&path).map_err(|e| format!("{e}"))?;
    check(q.is_empty(), "missing file → empty")
}

fn feedback_queue_save_then_load_round_trip() -> Result<(), String> {
    let dir = unique_tmp("save_load");
    std::fs::create_dir_all(&dir).map_err(|e| format!("{e}"))?;
    let path = dir.join("feedback.jsonl");
    let mut q = FeedbackQueue::default();
    q.append(sample_fb("Email", "email", "email", true));
    q.append(sample_fb("Phone", "phone", "phone", true));
    q.save_to(&path).map_err(|e| format!("{e}"))?;
    let back = FeedbackQueue::load_from(&path).map_err(|e| format!("{e}"))?;
    check_eq(q, back, "round-trip")?;
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

fn feedback_queue_preserves_event_order_across_save_load() -> Result<(), String> {
    let dir = unique_tmp("order");
    std::fs::create_dir_all(&dir).map_err(|e| format!("{e}"))?;
    let path = dir.join("feedback.jsonl");
    let mut q = FeedbackQueue::default();
    q.append(sample_fb("first", "email", "email", true));
    q.append(sample_fb("second", "phone", "phone", true));
    q.append(sample_fb("third", "freetext", "skip", false));
    q.save_to(&path).map_err(|e| format!("{e}"))?;
    let back = FeedbackQueue::load_from(&path).map_err(|e| format!("{e}"))?;
    check_eq(back.events[0].field.label.clone(), "first".to_string(), "first")?;
    check_eq(back.events[1].field.label.clone(), "second".to_string(), "second")?;
    check_eq(back.events[2].field.label.clone(), "third".to_string(), "third")?;
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

fn feedback_queue_save_idempotent_overwrites_prior_content() -> Result<(), String> {
    let dir = unique_tmp("overwrite");
    std::fs::create_dir_all(&dir).map_err(|e| format!("{e}"))?;
    let path = dir.join("feedback.jsonl");
    let mut q = FeedbackQueue::default();
    q.append(sample_fb("one", "email", "email", true));
    q.save_to(&path).map_err(|e| format!("{e}"))?;
    let mut q2 = FeedbackQueue::default();
    q2.append(sample_fb("two", "phone", "phone", true));
    q2.save_to(&path).map_err(|e| format!("{e}"))?;
    let back = FeedbackQueue::load_from(&path).map_err(|e| format!("{e}"))?;
    check_eq(back.len(), 1usize, "len after overwrite")?;
    check_eq(back.events[0].field.label.clone(), "two".to_string(), "second wins")?;
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

fn feedback_queue_save_atomically_via_tmp() -> Result<(), String> {
    let dir = unique_tmp("atomic");
    std::fs::create_dir_all(&dir).map_err(|e| format!("{e}"))?;
    let path = dir.join("feedback.jsonl");
    FeedbackQueue::default().save_to(&path).map_err(|e| format!("{e}"))?;
    let tmp = path.with_extension("jsonl.tmp");
    check(!tmp.exists(), ".tmp must be renamed away")?;
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

fn feedback_queue_corrupt_line_fails_loudly() -> Result<(), String> {
    let result = FeedbackQueue::from_jsonl("{this is not json}\n");
    check(result.is_err(), "corrupt line → Err")
}

// ─── predict_field_key ───────────────────────────────────────────────

fn predict_email_via_label() -> Result<(), String> {
    check_eq(predict_field_key(&fd("Email", "", "", "", "", "email")), "email", "email via label")
}

fn predict_email_via_placeholder() -> Result<(), String> {
    check_eq(
        predict_field_key(&fd("", "your.email@example.com", "", "", "", "text")),
        "email",
        "email via placeholder",
    )
}

fn predict_phone_variants() -> Result<(), String> {
    check_eq(predict_field_key(&fd("Mobile phone", "", "", "", "", "tel")), "phone", "Mobile phone")?;
    check_eq(predict_field_key(&fd("", "", "", "user_phone", "", "tel")), "phone", "user_phone")?;
    check_eq(predict_field_key(&fd("Mobile #", "", "", "", "", "tel")), "phone", "Mobile #")
}

fn predict_linkedin_github_website() -> Result<(), String> {
    check_eq(predict_field_key(&fd("LinkedIn URL", "", "", "", "", "url")), "linkedin", "LinkedIn")?;
    check_eq(predict_field_key(&fd("GitHub", "", "", "", "", "url")), "github", "GitHub")?;
    check_eq(predict_field_key(&fd("Personal website", "", "", "", "", "url")), "website", "website")?;
    check_eq(predict_field_key(&fd("Portfolio", "", "", "", "", "url")), "website", "portfolio")
}

fn predict_address_variants() -> Result<(), String> {
    check_eq(predict_field_key(&fd("Street address", "", "", "", "", "text")), "street1", "Street address")?;
    check_eq(predict_field_key(&fd("Street", "", "", "", "", "text")), "street1", "Street")?;
    check_eq(predict_field_key(&fd("Address Line 1", "", "", "", "", "text")), "street1", "Address Line 1")?;
    check_eq(predict_field_key(&fd("Address Line 2", "", "", "", "", "text")), "street2", "Address Line 2")?;
    check_eq(predict_field_key(&fd("Apartment / Suite", "", "", "", "", "text")), "street2", "Apartment / Suite")?;
    check_eq(predict_field_key(&fd("City", "", "", "", "", "text")), "city", "City")?;
    check_eq(predict_field_key(&fd("State", "", "", "", "", "text")), "state", "State")?;
    check_eq(predict_field_key(&fd("Province", "", "", "", "", "text")), "state", "Province")?;
    check_eq(predict_field_key(&fd("Country", "", "", "", "", "text")), "country", "Country")?;
    check_eq(predict_field_key(&fd("Zip code", "", "", "", "", "text")), "postal_code", "Zip code")?;
    check_eq(predict_field_key(&fd("Postal code", "", "", "", "", "text")), "postal_code", "Postal code")?;
    check_eq(predict_field_key(&fd("Address", "", "", "", "", "text")), "address", "bare Address")
}

fn predict_address_does_not_match_address_inside_unrelated_words() -> Result<(), String> {
    check_eq(
        predict_field_key(&fd("Email address", "", "", "user_email", "", "email")),
        "email",
        "Email address → email (not street1)",
    )
}

fn predict_work_authorization_variants() -> Result<(), String> {
    check_eq(
        predict_field_key(&fd("Are you authorized to work?", "", "", "", "", "select")),
        "work_authorization",
        "authorized to work",
    )?;
    check_eq(
        predict_field_key(&fd("Visa sponsorship required?", "", "", "", "", "select")),
        "work_authorization",
        "visa sponsorship",
    )
}

fn predict_years_experience() -> Result<(), String> {
    check_eq(
        predict_field_key(&fd("Years of relevant experience", "", "", "", "", "number")),
        "years_experience",
        "years of experience",
    )
}

fn predict_name_disambiguation() -> Result<(), String> {
    check_eq(predict_field_key(&fd("First name", "", "", "fname", "", "text")), "first_name", "First name")?;
    check_eq(predict_field_key(&fd("Last name", "", "", "", "", "text")), "last_name", "Last name")?;
    check_eq(predict_field_key(&fd("Surname", "", "", "", "", "text")), "last_name", "Surname")?;
    check_eq(predict_field_key(&fd("Given name", "", "", "", "", "text")), "first_name", "Given name")?;
    check_eq(predict_field_key(&fd("Family name", "", "", "", "", "text")), "last_name", "Family name")?;
    check_eq(predict_field_key(&fd("Full name", "", "", "", "", "text")), "full_name", "Full name")?;
    check_eq(predict_field_key(&fd("Name", "", "", "", "", "text")), "full_name", "Name")?;
    check_eq(
        predict_field_key(&fd("Legal Name (First)", "", "First Name", "legalNameFirst", "lf", "text")),
        "first_name",
        "Workday legal name (First)",
    )?;
    check_eq(
        predict_field_key(&fd("Legal Name (Last)", "", "Last Name", "legalNameLast", "ll", "text")),
        "last_name",
        "Workday legal name (Last)",
    )?;
    check_eq(
        predict_field_key(&fd("Full name", "", "", "name", "first", "text")),
        "full_name",
        "Lever combined-name (id=first, label=Full name)",
    )
}

fn predict_textarea_routes_to_freetext() -> Result<(), String> {
    check_eq(
        predict_field_key(&fd("Why do you want this role?", "", "", "", "", "textarea")),
        "freetext",
        "textarea → freetext",
    )
}

fn predict_unknown_returns_empty_not_a_guess() -> Result<(), String> {
    check_eq(predict_field_key(&fd("Hobbies", "", "", "", "", "text")), "", "Hobbies → empty")?;
    check_eq(predict_field_key(&fd("", "", "", "", "", "text")), "", "blank → empty")
}

fn predict_does_not_match_tel_inside_unrelated_words() -> Result<(), String> {
    check_eq(
        predict_field_key(&fd("LinkedIn (Optional)", "", "LinkedIn URL", "websiteLinkedIn", "wd_lk", "url")),
        "linkedin",
        "websiteLinkedIn → linkedin (not phone)",
    )
}

fn predict_phone_via_html_kind_tel() -> Result<(), String> {
    check_eq(predict_field_key(&fd("", "", "", "", "ph", "tel")), "phone", "type=tel → phone")
}

fn predict_email_via_html_kind_email() -> Result<(), String> {
    check_eq(predict_field_key(&fd("", "", "", "", "x", "email")), "email", "type=email → email")
}

fn predict_email_outranks_name_when_both_present() -> Result<(), String> {
    check_eq(
        predict_field_key(&fd("Contact name", "name@email.com", "", "", "", "text")),
        "email",
        "email evidence wins over name label",
    )
}

fn predict_with_confidence_returns_one_for_known() -> Result<(), String> {
    let (key, conf) = predict_field_key_with_confidence(&fd("Email", "", "", "", "", "email"));
    check_eq(key, "email", "key")?;
    check_eq(conf, 1.0, "confidence")
}

fn predict_with_confidence_returns_zero_for_unknown() -> Result<(), String> {
    let (key, conf) = predict_field_key_with_confidence(&fd("How many siblings?", "", "", "", "", "number"));
    check_eq(key, "", "key")?;
    check_eq(conf, 0.0, "confidence")
}

fn predict_gitlab_routes_to_gitlab() -> Result<(), String> {
    check_eq(predict_field_key(&fd("GitLab URL", "", "", "", "", "url")), "gitlab", "GitLab URL")?;
    check_eq(predict_field_key(&fd("", "", "", "gitlab_handle", "", "text")), "gitlab", "gitlab_handle")
}

fn predict_bitbucket_routes_to_bitbucket() -> Result<(), String> {
    check_eq(predict_field_key(&fd("Bitbucket", "", "", "", "", "url")), "bitbucket", "Bitbucket")
}

fn predict_twitter_routes_to_twitter() -> Result<(), String> {
    check_eq(predict_field_key(&fd("Twitter", "", "", "", "", "url")), "twitter", "Twitter")?;
    check_eq(
        predict_field_key(&fd("Twitter / X handle", "", "", "", "", "text")),
        "twitter",
        "Twitter / X handle",
    )
}

fn predict_bluesky_routes_to_bluesky() -> Result<(), String> {
    check_eq(predict_field_key(&fd("Bluesky", "", "", "", "", "url")), "bluesky", "Bluesky")?;
    check_eq(predict_field_key(&fd("Bluesky handle", "", "", "", "", "text")), "bluesky", "Bluesky handle")
}

fn predict_mastodon_routes_to_mastodon() -> Result<(), String> {
    check_eq(predict_field_key(&fd("Mastodon", "", "", "", "", "url")), "mastodon", "Mastodon")
}

fn predict_stackoverflow_routes_to_stackoverflow() -> Result<(), String> {
    check_eq(
        predict_field_key(&fd("StackOverflow URL", "", "", "", "", "url")),
        "stackoverflow",
        "StackOverflow URL",
    )?;
    check_eq(
        predict_field_key(&fd("Stack Overflow profile", "", "", "", "", "url")),
        "stackoverflow",
        "Stack Overflow profile",
    )
}

fn predict_blog_routes_to_blog() -> Result<(), String> {
    check_eq(predict_field_key(&fd("Blog URL", "", "", "", "", "url")), "blog", "Blog URL")?;
    check_eq(predict_field_key(&fd("Personal blog", "", "", "", "", "url")), "blog", "Personal blog")
}

fn predict_blog_does_not_match_inside_unrelated_words() -> Result<(), String> {
    let f = fd("Weblog server URL", "", "", "weblog_url", "", "url");
    let got = predict_field_key(&f);
    check(got != "blog", format!("Weblog must NOT classify as blog (got {got:?})"))
}

fn predict_salary_expectation_routes_to_salary_expectation() -> Result<(), String> {
    check_eq(
        predict_field_key(&fd("Salary expectation", "", "", "", "", "number")),
        "salary_expectation",
        "Salary expectation",
    )?;
    check_eq(
        predict_field_key(&fd("Expected salary", "", "", "", "", "number")),
        "salary_expectation",
        "Expected salary",
    )?;
    check_eq(
        predict_field_key(&fd("Compensation expectation", "", "", "", "", "text")),
        "salary_expectation",
        "Compensation expectation",
    )?;
    check_eq(
        predict_field_key(&fd("Expected pay", "", "", "", "", "number")),
        "salary_expectation",
        "Expected pay",
    )
}

fn predict_bare_compensation_does_not_match_salary() -> Result<(), String> {
    let got = predict_field_key(&fd("Compensation", "", "", "", "", "text"));
    check(
        got != "salary_expectation",
        format!("bare Compensation must NOT match salary (got {got:?})"),
    )
}

// ─── profile_value_for_key ───────────────────────────────────────────

fn profile_value_for_key_resolves_new_presence_keys() -> Result<(), String> {
    let p = Profile {
        presence: OnlinePresence {
            gitlab: "https://gitlab.com/janedoe".into(),
            bitbucket: "https://bitbucket.org/janedoe".into(),
            blog: "https://janedoe.dev/blog".into(),
            twitter: "@janedoe".into(),
            bluesky: "@janedoe.bsky.social".into(),
            mastodon: "@janedoe@mastodon.social".into(),
            stackoverflow: "https://stackoverflow.com/users/123".into(),
            portfolio: "https://janedoe.dev/portfolio".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    check_eq(profile_value_for_key(&p, "gitlab"), Some("https://gitlab.com/janedoe"), "gitlab")?;
    check_eq(profile_value_for_key(&p, "bitbucket"), Some("https://bitbucket.org/janedoe"), "bitbucket")?;
    check_eq(profile_value_for_key(&p, "blog"), Some("https://janedoe.dev/blog"), "blog")?;
    check_eq(profile_value_for_key(&p, "twitter"), Some("@janedoe"), "twitter")?;
    check_eq(profile_value_for_key(&p, "bluesky"), Some("@janedoe.bsky.social"), "bluesky")?;
    check_eq(profile_value_for_key(&p, "mastodon"), Some("@janedoe@mastodon.social"), "mastodon")?;
    check_eq(profile_value_for_key(&p, "stackoverflow"), Some("https://stackoverflow.com/users/123"), "stackoverflow")?;
    check_eq(profile_value_for_key(&p, "portfolio"), Some("https://janedoe.dev/portfolio"), "portfolio")
}

fn profile_value_for_key_portfolio_falls_back_to_website() -> Result<(), String> {
    let p = Profile {
        website: "https://janedoe.dev".into(),
        ..Default::default()
    };
    check_eq(
        profile_value_for_key(&p, "portfolio"),
        Some("https://janedoe.dev"),
        "portfolio falls back to website",
    )
}

fn profile_value_for_key_empty_presence_returns_none() -> Result<(), String> {
    let p = Profile::default();
    check(profile_value_for_key(&p, "gitlab").is_none(), "gitlab None")?;
    check(profile_value_for_key(&p, "twitter").is_none(), "twitter None")?;
    check(profile_value_for_key(&p, "mastodon").is_none(), "mastodon None")
}

fn profile_value_for_key_salary_expectation_returns_none() -> Result<(), String> {
    let p = Profile {
        compensation: Some(Compensation {
            salary_expectation_min: Some(150_000),
            salary_expectation_max: Some(220_000),
            salary_currency: "USD".into(),
            ..Default::default()
        }),
        ..Default::default()
    };
    check(
        profile_value_for_key(&p, "salary_expectation").is_none(),
        "salary_expectation not yet borrowable",
    )
}

fn profile_value_for_key_empty_field_returns_none() -> Result<(), String> {
    let p = Profile::default();
    check(profile_value_for_key(&p, "email").is_none(), "empty email → None")
}

fn profile_value_for_key_known_keys() -> Result<(), String> {
    let p = Profile {
        full_name: "Jane".into(),
        email: "j@e.com".into(),
        phone: "+1".into(),
        ..Default::default()
    };
    check_eq(profile_value_for_key(&p, "full_name"), Some("Jane"), "full_name")?;
    check_eq(profile_value_for_key(&p, "email"), Some("j@e.com"), "email")?;
    check_eq(profile_value_for_key(&p, "phone"), Some("+1"), "phone")
}

fn profile_value_for_key_unknown_keys_return_none() -> Result<(), String> {
    let p = Profile { full_name: "Jane".into(), ..Default::default() };
    check(profile_value_for_key(&p, "freetext").is_none(), "freetext None")?;
    check(profile_value_for_key(&p, "unknown").is_none(), "unknown None")?;
    check(profile_value_for_key(&p, "garbage").is_none(), "garbage None")
}

// ─── Phase G schema ───────────────────────────────────────────────────

fn experience_json_shape_is_stable() -> Result<(), String> {
    let e = Experience {
        company: "Acme Co".into(),
        title: "Engineer III".into(),
        start: "2020-01".into(),
        end: "2024-06".into(),
        bullets: vec!["shipped X".into(), "led Y".into()],
        ..Default::default()
    };
    let got = serde_json::to_string(&e).map_err(|e| format!("{e}"))?;
    let want = r#"{"company":"Acme Co","title":"Engineer III","start":"2020-01","end":"2024-06","bullets":["shipped X","led Y"],"location":"","employment_type":"","supervisor_name":"","supervisor_email":"","supervisor_phone":"","reason_for_leaving":"","can_we_contact":false}"#;
    check_eq(got, want.to_string(), "experience wire shape")
}

fn education_json_shape_is_stable() -> Result<(), String> {
    let ed = Education {
        school: "State U".into(),
        degree: "BS".into(),
        field: "CS".into(),
        start: "2016".into(),
        end: "2020".into(),
        gpa: None,
        ..Default::default()
    };
    let got = serde_json::to_string(&ed).map_err(|e| format!("{e}"))?;
    let want = r#"{"school":"State U","degree":"BS","field":"CS","start":"2016","end":"2020","gpa":null,"honors":[],"minor":"","relevant_coursework":[],"thesis_title":"","extracurriculars":[],"location":""}"#;
    check_eq(got, want.to_string(), "education wire shape")
}

fn profile_v0_toml_loads_with_phase_g_defaults() -> Result<(), String> {
    let v0 = r#"
        full_name = "Jane Doe"
        first_name = "Jane"
        last_name = "Doe"
        email = "jane@example.com"
        phone = "+1-555-0100"
        address = "123 Main St, Anywhere, CA 94000"
        street1 = ""
        street2 = ""
        city = ""
        state = ""
        postal_code = ""
        country = ""
        linkedin = ""
        github = ""
        website = ""
        work_authorization = "Citizen"
        years_experience = 7
        raw_resume_text = ""
        experience = []
        education = []
        skills = ["rust", "ml"]
    "#;
    let p: Profile = toml::from_str(v0).map_err(|e| format!("{e}"))?;
    check_eq(p.full_name.clone(), "Jane Doe".to_string(), "full_name")?;
    check_eq(p.first_name.clone(), "Jane".to_string(), "first_name")?;
    check_eq(p.last_name.clone(), "Doe".to_string(), "last_name")?;
    check_eq(p.years_experience, 7u8, "years_experience")?;
    check_eq(p.skills.clone(), vec!["rust".to_string(), "ml".to_string()], "skills")?;
    check_eq(p.preferred_name.clone(), "".to_string(), "preferred_name")?;
    check_eq(p.pronouns.clone(), "".to_string(), "pronouns")?;
    check_eq(p.presence, OnlinePresence::default(), "presence default")?;
    check(p.technical_skills.is_empty(), "technical_skills empty")?;
    check(p.languages.is_empty(), "languages empty")?;
    check(p.soft_skills.is_empty(), "soft_skills empty")?;
    check(p.certifications.is_empty(), "certifications empty")?;
    check(p.projects.is_empty(), "projects empty")?;
    check(p.compensation.is_none(), "compensation None")?;
    check(p.demographics.is_none(), "demographics None")
}

fn profile_v1_full_round_trip_preserves_all_phase_g_fields() -> Result<(), String> {
    let p = Profile {
        full_name: "Jane Q. Doe".into(),
        first_name: "Jane Q.".into(),
        last_name: "Doe".into(),
        preferred_name: "Janie".into(),
        pronouns: "she/her".into(),
        email: "jane@example.com".into(),
        phone: "+1-555-0100".into(),
        address: "".into(),
        street1: "123 Main St".into(),
        city: "Anywhere".into(),
        state: "CA".into(),
        postal_code: "94000".into(),
        country: "USA".into(),
        linkedin: "https://linkedin.com/in/janedoe".into(),
        github: "https://github.com/janedoe".into(),
        website: "https://janedoe.dev".into(),
        presence: OnlinePresence {
            gitlab: "https://gitlab.com/janedoe".into(),
            portfolio: "https://janedoe.dev/portfolio".into(),
            blog: "https://janedoe.dev/blog".into(),
            twitter: "@janedoe".into(),
            bluesky: "@janedoe.bsky.social".into(),
            mastodon: "@janedoe@mastodon.social".into(),
            stackoverflow: "https://stackoverflow.com/users/123/janedoe".into(),
            ..Default::default()
        },
        work_authorization: "Citizen".into(),
        years_experience: 7,
        technical_skills: vec![Skill {
            name: "Rust".into(),
            years: Some(5.5),
            level: SkillLevel::Advanced,
            last_used: "2026".into(),
        }],
        languages: vec![Language {
            name: "Spanish".into(),
            proficiency: LanguageProficiency::Conversational,
        }],
        soft_skills: vec!["written communication".into()],
        certifications: vec![Certification {
            name: "AWS Solutions Architect — Associate".into(),
            issuer: "Amazon Web Services".into(),
            issue_date: "2024-09".into(),
            expiry_date: "2027-09".into(),
            credential_id: "ABC-123".into(),
            credential_url: "https://credly.com/badges/abc-123".into(),
        }],
        projects: vec![Project {
            name: "atsisbroken".into(),
            description: "Browser autopilot for ATS forms".into(),
            url: "https://github.com/cochranblock/atsisbroken".into(),
            start_date: "2026-04".into(),
            end_date: "".into(),
            technologies: vec!["Rust".into(), "CDP".into()],
            role: "creator/maintainer".into(),
            highlights: vec!["263/263 tests".into()],
            github_repo: Some(RepoRef {
                owner: "cochranblock".into(),
                name: "atsisbroken".into(),
                url: "https://github.com/cochranblock/atsisbroken".into(),
            }),
        }],
        compensation: Some(Compensation {
            salary_expectation_min: Some(150_000),
            salary_expectation_max: Some(220_000),
            salary_currency: "USD".into(),
            compensation_notes: "open to equity-heavy".into(),
            desired_base: Some(170_000),
            desired_variable: Some(20_000),
            desired_equity: "0.1%".into(),
        }),
        demographics: Some(Demographics::default()),
        raw_resume_text: "".into(),
        ..Default::default()
    };
    let toml_text = toml::to_string(&p).map_err(|e| format!("{e}"))?;
    let back: Profile = toml::from_str(&toml_text).map_err(|e| format!("{e}"))?;
    check_eq(p, back, "full Phase G round-trip")
}

fn online_presence_json_shape_is_stable() -> Result<(), String> {
    let op = OnlinePresence::default();
    let got = serde_json::to_string(&op).map_err(|e| format!("{e}"))?;
    let want = r#"{"gitlab":"","bitbucket":"","portfolio":"","blog":"","twitter":"","bluesky":"","mastodon":"","stackoverflow":"","devto":"","medium":"","hashnode":"","youtube":"","dribbble":"","behance":"","artstation":""}"#;
    check_eq(got, want.to_string(), "online presence wire shape")
}

fn skill_json_shape_is_stable() -> Result<(), String> {
    let s = Skill {
        name: "Rust".into(),
        years: Some(5.0),
        level: SkillLevel::Advanced,
        last_used: "2026".into(),
    };
    let got = serde_json::to_string(&s).map_err(|e| format!("{e}"))?;
    let want = r#"{"name":"Rust","years":5.0,"level":"advanced","last_used":"2026"}"#;
    check_eq(got, want.to_string(), "skill wire shape")
}

fn skill_level_serializes_snake_case() -> Result<(), String> {
    check_eq(serde_json::to_string(&SkillLevel::Beginner).map_err(|e| format!("{e}"))?, "\"beginner\"".to_string(), "Beginner")?;
    check_eq(serde_json::to_string(&SkillLevel::Intermediate).map_err(|e| format!("{e}"))?, "\"intermediate\"".to_string(), "Intermediate")?;
    check_eq(serde_json::to_string(&SkillLevel::Advanced).map_err(|e| format!("{e}"))?, "\"advanced\"".to_string(), "Advanced")?;
    check_eq(serde_json::to_string(&SkillLevel::Expert).map_err(|e| format!("{e}"))?, "\"expert\"".to_string(), "Expert")
}

fn skill_default_level_is_intermediate() -> Result<(), String> {
    check_eq(SkillLevel::default(), SkillLevel::Intermediate, "default")
}

fn language_json_shape_is_stable() -> Result<(), String> {
    let l = Language {
        name: "Spanish".into(),
        proficiency: LanguageProficiency::Conversational,
    };
    let got = serde_json::to_string(&l).map_err(|e| format!("{e}"))?;
    let want = r#"{"name":"Spanish","proficiency":"conversational"}"#;
    check_eq(got, want.to_string(), "language wire shape")
}

fn language_proficiency_serializes_snake_case() -> Result<(), String> {
    check_eq(serde_json::to_string(&LanguageProficiency::Native).map_err(|e| format!("{e}"))?, "\"native\"".to_string(), "Native")?;
    check_eq(serde_json::to_string(&LanguageProficiency::Fluent).map_err(|e| format!("{e}"))?, "\"fluent\"".to_string(), "Fluent")?;
    check_eq(serde_json::to_string(&LanguageProficiency::Conversational).map_err(|e| format!("{e}"))?, "\"conversational\"".to_string(), "Conversational")?;
    check_eq(serde_json::to_string(&LanguageProficiency::Beginner).map_err(|e| format!("{e}"))?, "\"beginner\"".to_string(), "Beginner")
}

fn certification_json_shape_is_stable() -> Result<(), String> {
    let c = Certification {
        name: "AWS SAA".into(),
        issuer: "AWS".into(),
        issue_date: "2024-09".into(),
        expiry_date: "2027-09".into(),
        credential_id: "ABC-123".into(),
        credential_url: "https://credly.com/badges/abc-123".into(),
    };
    let got = serde_json::to_string(&c).map_err(|e| format!("{e}"))?;
    let want = r#"{"name":"AWS SAA","issuer":"AWS","issue_date":"2024-09","expiry_date":"2027-09","credential_id":"ABC-123","credential_url":"https://credly.com/badges/abc-123"}"#;
    check_eq(got, want.to_string(), "certification wire shape")
}

fn project_with_github_repo_json_shape_is_stable() -> Result<(), String> {
    let proj = Project {
        name: "atsisbroken".into(),
        description: "Browser autopilot".into(),
        url: "https://github.com/cochranblock/atsisbroken".into(),
        start_date: "2026-04".into(),
        end_date: "".into(),
        technologies: vec!["Rust".into()],
        role: "creator".into(),
        highlights: vec!["263/263".into()],
        github_repo: Some(RepoRef {
            owner: "cochranblock".into(),
            name: "atsisbroken".into(),
            url: "https://github.com/cochranblock/atsisbroken".into(),
        }),
    };
    let got = serde_json::to_string(&proj).map_err(|e| format!("{e}"))?;
    let want = r#"{"name":"atsisbroken","description":"Browser autopilot","url":"https://github.com/cochranblock/atsisbroken","start_date":"2026-04","end_date":"","technologies":["Rust"],"role":"creator","highlights":["263/263"],"github_repo":{"owner":"cochranblock","name":"atsisbroken","url":"https://github.com/cochranblock/atsisbroken"}}"#;
    check_eq(got, want.to_string(), "project with repo wire shape")
}

fn project_without_github_repo_serializes_repo_as_null() -> Result<(), String> {
    let proj = Project {
        name: "private project".into(),
        description: "personal".into(),
        ..Default::default()
    };
    let got = serde_json::to_string(&proj).map_err(|e| format!("{e}"))?;
    check(got.contains(r#""github_repo":null"#), format!("expected null repo, got: {got}"))
}

fn compensation_json_shape_is_stable() -> Result<(), String> {
    let c = Compensation {
        salary_expectation_min: Some(150_000),
        salary_expectation_max: Some(220_000),
        salary_currency: "USD".into(),
        compensation_notes: "open to equity-heavy".into(),
        desired_base: Some(170_000),
        desired_variable: Some(20_000),
        desired_equity: "0.1%".into(),
    };
    let got = serde_json::to_string(&c).map_err(|e| format!("{e}"))?;
    let want = r#"{"salary_expectation_min":150000,"salary_expectation_max":220000,"salary_currency":"USD","compensation_notes":"open to equity-heavy","desired_base":170000,"desired_variable":20000,"desired_equity":"0.1%"}"#;
    check_eq(got, want.to_string(), "compensation wire shape")
}

fn demographics_default_is_all_empty() -> Result<(), String> {
    let d = Demographics::default();
    check(d.gender.is_empty(), "gender empty")?;
    check(d.race_ethnicity.is_empty(), "race_ethnicity empty")?;
    check(d.veteran_status.is_empty(), "veteran_status empty")?;
    check(d.disability_status.is_empty(), "disability_status empty")?;
    check(d.lgbtq_self_id.is_empty(), "lgbtq_self_id empty")
}

fn profile_demographics_default_is_none_not_empty_struct() -> Result<(), String> {
    check(Profile::default().demographics.is_none(), "demographics default None")
}

fn profile_compensation_default_is_none() -> Result<(), String> {
    check(Profile::default().compensation.is_none(), "compensation default None")
}

fn repo_ref_round_trip() -> Result<(), String> {
    let r = RepoRef {
        owner: "cochranblock".into(),
        name: "atsisbroken".into(),
        url: "https://github.com/cochranblock/atsisbroken".into(),
    };
    let s = serde_json::to_string(&r).map_err(|e| format!("{e}"))?;
    let back: RepoRef = serde_json::from_str(&s).map_err(|e| format!("{e}"))?;
    check_eq(r, back, "RepoRef round-trip")
}

fn experience_v0_toml_loads_with_phase_g_defaults() -> Result<(), String> {
    let v0 = r#"
        company = "Acme Co"
        title   = "Engineer III"
        start   = "2020-01"
        end     = "2024-06"
        bullets = ["shipped X", "led Y"]
    "#;
    let e: Experience = toml::from_str(v0).map_err(|e| format!("{e}"))?;
    check_eq(e.company.clone(), "Acme Co".to_string(), "company")?;
    check_eq(e.location.clone(), "".to_string(), "location default")?;
    check_eq(e.employment_type.clone(), "".to_string(), "employment_type default")?;
    check(!e.can_we_contact, "can_we_contact false")
}

fn education_v0_toml_loads_with_phase_g_defaults() -> Result<(), String> {
    let v0 = r#"
        school = "State U"
        degree = "BS"
        field  = "CS"
        start  = "2016"
        end    = "2020"
    "#;
    let ed: Education = toml::from_str(v0).map_err(|e| format!("{e}"))?;
    check_eq(ed.school.clone(), "State U".to_string(), "school")?;
    check(ed.honors.is_empty(), "honors empty")?;
    check_eq(ed.minor.clone(), "".to_string(), "minor")?;
    check_eq(ed.thesis_title.clone(), "".to_string(), "thesis_title")
}

// ─── Phase G Tier 2 ──────────────────────────────────────────────────

fn profile_tier2_defaults_are_inert() -> Result<(), String> {
    let p = Profile::default();
    check(p.work_auth.is_none(), "work_auth None")?;
    check(p.publications.is_empty(), "publications empty")?;
    check(p.patents.is_empty(), "patents empty")?;
    check(p.awards.is_empty(), "awards empty")?;
    check(p.references.is_empty(), "references empty")?;
    check(p.applications.is_empty(), "applications empty")?;
    check(p.free_form.elevator_pitch.is_none(), "elevator_pitch None")?;
    check(p.free_form.proudest_project.is_none(), "proudest_project None")?;
    check(p.free_form.custom.is_empty(), "free_form.custom empty")
}

fn profile_v0_toml_loads_with_tier2_defaults() -> Result<(), String> {
    let v0 = r#"
        full_name = "Jane Doe"
        email = "jane@example.com"
        phone = ""
        address = ""
        linkedin = ""
        github = ""
        website = ""
        work_authorization = "Citizen"
        years_experience = 0
        raw_resume_text = ""
        experience = []
        education = []
        skills = []
    "#;
    let p: Profile = toml::from_str(v0).map_err(|e| format!("{e}"))?;
    check(p.work_auth.is_none(), "work_auth None")?;
    check(p.publications.is_empty(), "publications empty")?;
    check(p.patents.is_empty(), "patents empty")?;
    check(p.awards.is_empty(), "awards empty")?;
    check(p.references.is_empty(), "references empty")?;
    check(p.applications.is_empty(), "applications empty")?;
    check_eq(p.free_form.clone(), FreeFormAnswers::default(), "free_form default")?;
    check_eq(p.work_authorization.clone(), "Citizen".to_string(), "work_authorization preserved")
}

fn work_auth_status_serializes_snake_case() -> Result<(), String> {
    check_eq(serde_json::to_string(&WorkAuthStatus::Citizen).map_err(|e| format!("{e}"))?, "\"citizen\"".to_string(), "Citizen")?;
    check_eq(serde_json::to_string(&WorkAuthStatus::PermanentResident).map_err(|e| format!("{e}"))?, "\"permanent_resident\"".to_string(), "PermanentResident")?;
    check_eq(serde_json::to_string(&WorkAuthStatus::H1B).map_err(|e| format!("{e}"))?, "\"h1_b\"".to_string(), "H1B")?;
    check_eq(serde_json::to_string(&WorkAuthStatus::Opt).map_err(|e| format!("{e}"))?, "\"opt\"".to_string(), "Opt")?;
    check_eq(serde_json::to_string(&WorkAuthStatus::RequireSponsorship).map_err(|e| format!("{e}"))?, "\"require_sponsorship\"".to_string(), "RequireSponsorship")?;
    let s = serde_json::to_string(&WorkAuthStatus::Other("DACA".into())).map_err(|e| format!("{e}"))?;
    check_eq(s, r#"{"other":"DACA"}"#.to_string(), "Other(DACA)")
}

fn work_auth_status_default_is_empty_other_escape_hatch() -> Result<(), String> {
    check_eq(WorkAuthStatus::default(), WorkAuthStatus::Other(String::new()), "default")
}

fn security_clearance_none_distinct_from_unspecified() -> Result<(), String> {
    check(SecurityClearance::default() != SecurityClearance::None, "None ≠ default")?;
    check_eq(SecurityClearance::default(), SecurityClearance::Other(String::new()), "default")
}

fn remote_preference_default_is_flexible() -> Result<(), String> {
    check_eq(RemotePreference::default(), RemotePreference::Flexible, "default")
}

fn remote_preference_serializes_snake_case() -> Result<(), String> {
    check_eq(serde_json::to_string(&RemotePreference::Remote).map_err(|e| format!("{e}"))?, "\"remote\"".to_string(), "Remote")?;
    check_eq(serde_json::to_string(&RemotePreference::Hybrid).map_err(|e| format!("{e}"))?, "\"hybrid\"".to_string(), "Hybrid")?;
    check_eq(serde_json::to_string(&RemotePreference::OnSite).map_err(|e| format!("{e}"))?, "\"on_site\"".to_string(), "OnSite")?;
    check_eq(serde_json::to_string(&RemotePreference::Flexible).map_err(|e| format!("{e}"))?, "\"flexible\"".to_string(), "Flexible")
}

fn application_status_default_is_submitted() -> Result<(), String> {
    check_eq(ApplicationStatus::default(), ApplicationStatus::Submitted, "default")
}

fn application_status_serializes_snake_case() -> Result<(), String> {
    check_eq(serde_json::to_string(&ApplicationStatus::Submitted).map_err(|e| format!("{e}"))?, "\"submitted\"".to_string(), "Submitted")?;
    check_eq(serde_json::to_string(&ApplicationStatus::Interviewed).map_err(|e| format!("{e}"))?, "\"interviewed\"".to_string(), "Interviewed")?;
    check_eq(serde_json::to_string(&ApplicationStatus::Offer).map_err(|e| format!("{e}"))?, "\"offer\"".to_string(), "Offer")?;
    check_eq(serde_json::to_string(&ApplicationStatus::Rejected).map_err(|e| format!("{e}"))?, "\"rejected\"".to_string(), "Rejected")?;
    check_eq(serde_json::to_string(&ApplicationStatus::Ghosted).map_err(|e| format!("{e}"))?, "\"ghosted\"".to_string(), "Ghosted")?;
    check_eq(serde_json::to_string(&ApplicationStatus::Withdrawn).map_err(|e| format!("{e}"))?, "\"withdrawn\"".to_string(), "Withdrawn")
}

fn work_auth_json_shape_is_stable() -> Result<(), String> {
    let wa = WorkAuth {
        citizenship_country: "USA".into(),
        status: WorkAuthStatus::Citizen,
        visa_sponsorship_required: false,
        security_clearance: SecurityClearance::None,
        relocation_willingness: true,
        region_preferences: vec!["West Coast US".into()],
        remote_preference: RemotePreference::Hybrid,
    };
    let got = serde_json::to_string(&wa).map_err(|e| format!("{e}"))?;
    let want = r#"{"citizenship_country":"USA","status":"citizen","visa_sponsorship_required":false,"security_clearance":"none","relocation_willingness":true,"region_preferences":["West Coast US"],"remote_preference":"hybrid"}"#;
    check_eq(got, want.to_string(), "work_auth wire shape")
}

fn publication_json_shape_is_stable() -> Result<(), String> {
    let pub_ = Publication {
        title: "On the Verbatim Source Property".into(),
        authors: vec!["Cochran, M.".into()],
        venue: "USENIX Security".into(),
        date: "2026-08".into(),
        url: "https://example.org/paper".into(),
        doi: "10.1234/example.5678".into(),
    };
    let got = serde_json::to_string(&pub_).map_err(|e| format!("{e}"))?;
    let want = r#"{"title":"On the Verbatim Source Property","authors":["Cochran, M."],"venue":"USENIX Security","date":"2026-08","url":"https://example.org/paper","doi":"10.1234/example.5678"}"#;
    check_eq(got, want.to_string(), "publication wire shape")
}

fn patent_json_shape_is_stable() -> Result<(), String> {
    let p = Patent {
        title: "Method for verifiable autofill".into(),
        number: "US10,123,456 B2".into(),
        issued_date: "2024-03-12".into(),
        inventors: vec!["Cochran, M.".into()],
        url: "https://patents.google.com/patent/US10123456B2".into(),
    };
    let got = serde_json::to_string(&p).map_err(|e| format!("{e}"))?;
    let want = r#"{"title":"Method for verifiable autofill","number":"US10,123,456 B2","issued_date":"2024-03-12","inventors":["Cochran, M."],"url":"https://patents.google.com/patent/US10123456B2"}"#;
    check_eq(got, want.to_string(), "patent wire shape")
}

fn award_json_shape_is_stable() -> Result<(), String> {
    let a = Award {
        name: "Outstanding Contribution Award".into(),
        issuer: "Open Source Foundation".into(),
        date: "2025".into(),
        description: "For sustained Rust ecosystem maintenance".into(),
    };
    let got = serde_json::to_string(&a).map_err(|e| format!("{e}"))?;
    let want = r#"{"name":"Outstanding Contribution Award","issuer":"Open Source Foundation","date":"2025","description":"For sustained Rust ecosystem maintenance"}"#;
    check_eq(got, want.to_string(), "award wire shape")
}

fn reference_json_shape_is_stable() -> Result<(), String> {
    let r = Reference {
        name: "Pat Doe".into(),
        relationship: "Direct Manager".into(),
        company: "Acme Co".into(),
        title: "VP Engineering".into(),
        email: "pat.doe@acme.example".into(),
        phone: "+1-555-0188".into(),
    };
    let got = serde_json::to_string(&r).map_err(|e| format!("{e}"))?;
    let want = r#"{"name":"Pat Doe","relationship":"Direct Manager","company":"Acme Co","title":"VP Engineering","email":"pat.doe@acme.example","phone":"+1-555-0188"}"#;
    check_eq(got, want.to_string(), "reference wire shape")
}

fn application_json_shape_is_stable() -> Result<(), String> {
    let a = Application {
        company: "Example Corp".into(),
        role: "Senior Engineer".into(),
        submitted_at: "2026-05-06T10:11:12Z".into(),
        url: "https://example.com/jobs/123".into(),
        status: ApplicationStatus::Submitted,
        notes: "applied via Greenhouse".into(),
    };
    let got = serde_json::to_string(&a).map_err(|e| format!("{e}"))?;
    let want = r#"{"company":"Example Corp","role":"Senior Engineer","submitted_at":"2026-05-06T10:11:12Z","url":"https://example.com/jobs/123","status":"submitted","notes":"applied via Greenhouse"}"#;
    check_eq(got, want.to_string(), "application wire shape")
}

fn free_form_answers_json_shape_is_stable() -> Result<(), String> {
    let f = FreeFormAnswers::default();
    let got = serde_json::to_string(&f).map_err(|e| format!("{e}"))?;
    let want = r#"{"elevator_pitch":null,"why_this_role_template":null,"biggest_technical_challenge":null,"proudest_project":null,"biggest_failure_and_lesson":null,"five_year_plan":null,"strengths":null,"weaknesses":null,"management_style":null,"collaboration_example":null,"custom":{}}"#;
    check_eq(got, want.to_string(), "free_form_answers wire shape")
}

fn free_form_none_vs_empty_some_are_distinct_signals() -> Result<(), String> {
    let none_signal = FreeFormAnswers::default();
    let cleared = FreeFormAnswers {
        elevator_pitch: Some(String::new()),
        ..Default::default()
    };
    check(none_signal != cleared, "None ≠ Some(\"\")")?;
    check(none_signal.elevator_pitch.is_none(), "None signal has None")?;
    check_eq(cleared.elevator_pitch.as_deref(), Some(""), "cleared has Some(\"\")")
}

fn application_status_round_trip() -> Result<(), String> {
    for s in [
        ApplicationStatus::Submitted,
        ApplicationStatus::Interviewed,
        ApplicationStatus::Offer,
        ApplicationStatus::Rejected,
        ApplicationStatus::Ghosted,
        ApplicationStatus::Withdrawn,
    ] {
        let j = serde_json::to_string(&s).map_err(|e| format!("{e}"))?;
        let back: ApplicationStatus = serde_json::from_str(&j).map_err(|e| format!("{e}"))?;
        check_eq(s, back, &format!("ApplicationStatus round-trip"))?;
    }
    Ok(())
}

fn work_auth_status_round_trip_including_other() -> Result<(), String> {
    for s in [
        WorkAuthStatus::Citizen,
        WorkAuthStatus::PermanentResident,
        WorkAuthStatus::H1B,
        WorkAuthStatus::Opt,
        WorkAuthStatus::Ead,
        WorkAuthStatus::Tn,
        WorkAuthStatus::OptionalPracticalTraining,
        WorkAuthStatus::RequireSponsorship,
        WorkAuthStatus::Other("DACA".into()),
        WorkAuthStatus::Other(String::new()),
    ] {
        let j = serde_json::to_string(&s).map_err(|e| format!("{e}"))?;
        let back: WorkAuthStatus = serde_json::from_str(&j).map_err(|e| format!("{e}"))?;
        check_eq(s, back, "WorkAuthStatus round-trip")?;
    }
    Ok(())
}

fn profile_v1_tier2_full_round_trip() -> Result<(), String> {
    let p = Profile {
        full_name: "Jane Doe".into(),
        email: "jane@example.com".into(),
        phone: "+1-555-0100".into(),
        address: "123 Main St".into(),
        work_auth: Some(WorkAuth {
            citizenship_country: "USA".into(),
            status: WorkAuthStatus::Citizen,
            visa_sponsorship_required: false,
            security_clearance: SecurityClearance::Secret,
            relocation_willingness: true,
            region_preferences: vec!["West Coast US".into(), "EMEA".into()],
            remote_preference: RemotePreference::Hybrid,
        }),
        publications: vec![Publication {
            title: "Paper".into(),
            authors: vec!["Doe, J.".into()],
            venue: "Conf".into(),
            date: "2025".into(),
            url: String::new(),
            doi: String::new(),
        }],
        patents: vec![Patent {
            title: "Widget".into(),
            number: "US123".into(),
            issued_date: "2024".into(),
            inventors: vec!["Doe, J.".into()],
            url: String::new(),
        }],
        awards: vec![Award {
            name: "Award".into(),
            issuer: "Org".into(),
            date: "2023".into(),
            description: String::new(),
        }],
        references: vec![Reference {
            name: "Pat".into(),
            relationship: "Manager".into(),
            company: "Acme".into(),
            title: "VP".into(),
            email: "pat@acme.example".into(),
            phone: String::new(),
        }],
        applications: vec![Application {
            company: "Example".into(),
            role: "SWE".into(),
            submitted_at: "2026-05-06T10:11:12Z".into(),
            url: "https://example.com/jobs/1".into(),
            status: ApplicationStatus::Interviewed,
            notes: String::new(),
        }],
        free_form: FreeFormAnswers {
            elevator_pitch: Some("I build verifiable software.".into()),
            strengths: Some("Systems design.".into()),
            ..Default::default()
        },
        ..Default::default()
    };
    let toml_text = toml::to_string(&p).map_err(|e| format!("{e}"))?;
    let back: Profile = toml::from_str(&toml_text).map_err(|e| format!("{e}"))?;
    check_eq(p, back, "Tier 2 full round-trip")
}

// ─── Triple sims determinism ─────────────────────────────────────────

fn triple_sims_determinism() -> Result<(), String> {
    let run = || -> Result<(usize, u32, usize, String), String> {
        let pairs = parse_seed_corpus().map_err(|e| format!("{e}"))?;
        let p = Profile {
            full_name: "Jane Doe".into(),
            email: "jane@example.com".into(),
            ..Default::default()
        };
        Ok((
            seed_corpus_size(),
            seed_corpus_fingerprint(),
            pairs.len(),
            serde_json::to_string(&p).map_err(|e| format!("{e}"))?,
        ))
    };
    let s1 = run()?;
    let s2 = run()?;
    let s3 = run()?;
    check_eq(s1.clone(), s2.clone(), "sim 1→2")?;
    check_eq(s2, s3, "sim 2→3")
}
