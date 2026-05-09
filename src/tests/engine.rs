// SPDX-License-Identifier: Unlicense

//! Tests for `crate::browser::engine` — Phase 9. Includes a
//! hand-rolled equivalent for insta snapshot tests via
//! [`super::check_snapshot`], reading the same `.snap` files
//! cargo's `cargo insta accept` produces.

use crate::browser::engine::{
    default_user_agent, parse_html, Engine, EngineError, StaticHtmlEngine,
};
use crate::browser::Url;

use super::{case, check, check_eq, check_snapshot, TestResult};

pub fn run() -> Vec<TestResult> {
    vec![
        case("engine::parse_html_extracts_fields", parse_html_extracts_fields),
        case("engine::snapshot_fields_before_navigate_errors_cleanly",
             snapshot_fields_before_navigate_errors_cleanly),
        case("engine::fill_field_unimplemented_on_static_engine",
             fill_field_unimplemented_on_static_engine),
        case("engine::navigate_to_internal_url_renders_in_process",
             navigate_to_internal_url_renders_in_process),
        case("engine::navigate_internal_unknown_returns_not_found_page",
             navigate_internal_unknown_returns_not_found_page),
        case("engine::engine_state_persists_across_repeated_internal_navigates",
             engine_state_persists_across_repeated_internal_navigates),
        case("engine::engine_fields_are_empty_for_internal_pages",
             engine_fields_are_empty_for_internal_pages),
        case("engine::engine_navigate_outcome_internal_shape",
             engine_navigate_outcome_internal_shape),
        case("engine::engine_html_extracts_kind_aria_label_id_name_placeholder",
             engine_html_extracts_kind_aria_label_id_name_placeholder),
        case("engine::engine_input_kind_defaults_to_text_when_omitted",
             engine_input_kind_defaults_to_text_when_omitted),
        case("engine::engine_skips_non_form_elements", engine_skips_non_form_elements),
        case("engine::engine_handles_unicode_in_title", engine_handles_unicode_in_title),
        case("engine::engine_title_empty_when_no_title_tag", engine_title_empty_when_no_title_tag),
        case("engine::engine_handles_malformed_html_without_panic",
             engine_handles_malformed_html_without_panic),
        case("engine::snapshot_internal_page_home", snapshot_internal_page_home),
        case("engine::snapshot_internal_page_connections", snapshot_internal_page_connections),
        case("engine::snapshot_internal_page_summary", snapshot_internal_page_summary),
        case("engine::snapshot_internal_page_fingerprint", snapshot_internal_page_fingerprint),
        case("engine::snapshot_internal_page_shield", snapshot_internal_page_shield),
        case("engine::snapshot_internal_page_not_found", snapshot_internal_page_not_found),
        case("engine::navigate_unsupported_scheme_errors", navigate_unsupported_scheme_errors),
        case("engine::user_agent_includes_crate_version", user_agent_includes_crate_version),
    ]
}

fn parse_html_extracts_fields() -> Result<(), String> {
    let html = r#"
<!DOCTYPE html>
<html>
<head><title>Apply</title></head>
<body>
<form>
<input type="email" name="email" id="email-id" placeholder="you@example.com" aria-label="Email">
<input type="tel" name="phone" id="phone-id">
<textarea name="why" id="why-id" placeholder="Why this role?"></textarea>
<select name="country" id="country-id">
<option>USA</option>
</select>
</form>
</body>
</html>"#;
    let dom = parse_html(html).map_err(|e| format!("{e}"))?;
    check_eq(StaticHtmlEngine::extract_title(&dom), "Apply".to_string(), "title")?;
    let fields = StaticHtmlEngine::extract_fields(&dom);
    check_eq(fields.len(), 4usize, "fields count")?;
    let email = fields.iter().find(|f| f.name == "email").ok_or_else(|| "no email".to_string())?;
    check_eq(email.kind.as_str(), "email", "email kind")?;
    check_eq(email.placeholder.as_str(), "you@example.com", "email placeholder")?;
    check_eq(email.aria_label.as_str(), "Email", "email aria_label")?;
    let textarea = fields.iter().find(|f| f.name == "why").ok_or_else(|| "no textarea".to_string())?;
    check_eq(textarea.kind.as_str(), "textarea", "textarea kind")?;
    let select = fields.iter().find(|f| f.name == "country").ok_or_else(|| "no select".to_string())?;
    check_eq(select.kind.as_str(), "select", "select kind")?;
    let phone = fields.iter().find(|f| f.name == "phone").ok_or_else(|| "no phone".to_string())?;
    check_eq(phone.kind.as_str(), "tel", "phone kind")
}

fn snapshot_fields_before_navigate_errors_cleanly() -> Result<(), String> {
    let engine = StaticHtmlEngine::new().map_err(|e| format!("{e}"))?;
    let r = engine.snapshot_fields();
    check(r.is_err(), "snapshot before navigate must error")
}

fn fill_field_unimplemented_on_static_engine() -> Result<(), String> {
    let mut e = StaticHtmlEngine::new().map_err(|e| format!("{e}"))?;
    let mut t = crate::browser::input::InputTiming::new(crate::browser::HumanInputProfile::default());
    match e.fill_field("anything", "value", &mut t) {
        Err(EngineError::Unimplemented(_)) => Ok(()),
        other => Err(format!("expected Unimplemented, got {other:?}")),
    }
}

fn navigate_to_internal_url_renders_in_process() -> Result<(), String> {
    let mut e = StaticHtmlEngine::new().map_err(|e| format!("{e}"))?;
    let url: Url = "atsisbroken://home".parse().map_err(|e| format!("{e}"))?;
    let outcome = e.navigate(&url).map_err(|e| format!("{e}"))?;
    check_eq(outcome.status, 0u16, "status")?;
    check_eq(outcome.content_type.as_str(), "text/atsisbroken-internal", "content_type")?;
    check_eq(outcome.form_field_count, 0usize, "form_field_count")?;
    let snap = e.snapshot_fields().map_err(|e| format!("{e}"))?;
    check_eq(snap.title.as_str(), "atsisbroken", "title")?;
    check(!snap.body.is_empty(), "body not empty")?;
    check(snap.fields.is_empty(), "no fields")
}

fn navigate_internal_unknown_returns_not_found_page() -> Result<(), String> {
    let mut e = StaticHtmlEngine::new().map_err(|e| format!("{e}"))?;
    let url: Url = "atsisbroken://does-not-exist".parse().map_err(|e| format!("{e}"))?;
    e.navigate(&url).map_err(|e| format!("{e}"))?;
    let snap = e.snapshot_fields().map_err(|e| format!("{e}"))?;
    check_eq(snap.title.as_str(), "Page not found", "404 title")?;
    check(snap.body.contains("atsisbroken://does-not-exist"), "404 body cites URL")
}

fn engine_state_persists_across_repeated_internal_navigates() -> Result<(), String> {
    let mut e = StaticHtmlEngine::new().map_err(|e| format!("{e}"))?;
    e.navigate(&"atsisbroken://home".parse().map_err(|e: _| format!("{e}"))?)
        .map_err(|e| format!("{e}"))?;
    let snap1 = e.snapshot_fields().map_err(|e| format!("{e}"))?;
    check_eq(snap1.title.as_str(), "atsisbroken", "first title")?;

    e.navigate(&"atsisbroken://connections".parse().map_err(|e: _| format!("{e}"))?)
        .map_err(|e| format!("{e}"))?;
    let snap2 = e.snapshot_fields().map_err(|e| format!("{e}"))?;
    check(snap2.title.contains("connections"), "second title contains 'connections'")?;
    check(snap2.body.contains("GitHub"), "second body has GitHub")?;
    check(!snap2.body.contains("atsisbroken: the browser"), "no leakage from prior page")
}

fn engine_fields_are_empty_for_internal_pages() -> Result<(), String> {
    let mut e = StaticHtmlEngine::new().map_err(|e| format!("{e}"))?;
    for host in ["home", "connections", "summary", "applications"] {
        let url: Url = format!("atsisbroken://{host}").parse().map_err(|e: _| format!("{e}"))?;
        e.navigate(&url).map_err(|e| format!("{e}"))?;
        let snap = e.snapshot_fields().map_err(|e| format!("{e}"))?;
        check(
            snap.fields.is_empty(),
            format!("internal page {host} produced form fields: {:?}", snap.fields),
        )?;
    }
    Ok(())
}

fn engine_navigate_outcome_internal_shape() -> Result<(), String> {
    let mut e = StaticHtmlEngine::new().map_err(|e| format!("{e}"))?;
    let url: Url = "atsisbroken://summary".parse().map_err(|e| format!("{e}"))?;
    let outcome = e.navigate(&url).map_err(|e| format!("{e}"))?;
    check_eq(outcome.status, 0u16, "status")?;
    check_eq(outcome.content_type.as_str(), "text/atsisbroken-internal", "content_type")?;
    check_eq(outcome.form_field_count, 0usize, "form_field_count")?;
    check_eq(outcome.final_url, url, "final_url")
}

fn engine_html_extracts_kind_aria_label_id_name_placeholder() -> Result<(), String> {
    let html = r#"<!DOCTYPE html>
<form>
  <input type="email" name="contact-email" id="email-input"
         placeholder="you@example.com" aria-label="Email address">
  <textarea name="bio" id="bio-text" placeholder="Tell us"></textarea>
  <select name="country" id="country-select">
    <option>USA</option>
  </select>
</form>"#;
    let dom = parse_html(html).map_err(|e| format!("{e}"))?;
    let fields = StaticHtmlEngine::extract_fields(&dom);
    check_eq(fields.len(), 3usize, "fields count")?;

    let email = fields.iter().find(|f| f.name == "contact-email").ok_or_else(|| "no email".to_string())?;
    check_eq(email.kind.as_str(), "email", "email kind")?;
    check_eq(email.id.as_str(), "email-input", "email id")?;
    check_eq(email.placeholder.as_str(), "you@example.com", "email placeholder")?;
    check_eq(email.aria_label.as_str(), "Email address", "email aria")?;

    let bio = fields.iter().find(|f| f.name == "bio").ok_or_else(|| "no bio".to_string())?;
    check_eq(bio.kind.as_str(), "textarea", "bio kind")?;
    check_eq(bio.id.as_str(), "bio-text", "bio id")?;
    check_eq(bio.placeholder.as_str(), "Tell us", "bio placeholder")?;

    let country = fields.iter().find(|f| f.name == "country").ok_or_else(|| "no country".to_string())?;
    check_eq(country.kind.as_str(), "select", "country kind")
}

fn engine_input_kind_defaults_to_text_when_omitted() -> Result<(), String> {
    let html = r#"<input name="username" id="u">"#;
    let dom = parse_html(html).map_err(|e| format!("{e}"))?;
    let fields = StaticHtmlEngine::extract_fields(&dom);
    check_eq(fields.len(), 1usize, "fields count")?;
    check_eq(fields[0].kind.as_str(), "text", "default kind")
}

fn engine_skips_non_form_elements() -> Result<(), String> {
    let html = r##"<!DOCTYPE html>
<html><body>
<button>Click me</button>
<a href="#">Link</a>
<div><p>Not a form field</p></div>
<input name="real-field" type="text">
</body></html>"##;
    let dom = parse_html(html).map_err(|e| format!("{e}"))?;
    let fields = StaticHtmlEngine::extract_fields(&dom);
    check_eq(fields.len(), 1usize, "fields count")?;
    check_eq(fields[0].name.as_str(), "real-field", "field name")
}

fn engine_handles_unicode_in_title() -> Result<(), String> {
    let html = "<!DOCTYPE html><title>café — application</title>";
    let dom = parse_html(html).map_err(|e| format!("{e}"))?;
    check_eq(
        StaticHtmlEngine::extract_title(&dom),
        "café — application".to_string(),
        "unicode title",
    )
}

fn engine_title_empty_when_no_title_tag() -> Result<(), String> {
    let html = "<!DOCTYPE html><body>no title</body>";
    let dom = parse_html(html).map_err(|e| format!("{e}"))?;
    check_eq(StaticHtmlEngine::extract_title(&dom), "".to_string(), "empty title")
}

fn engine_handles_malformed_html_without_panic() -> Result<(), String> {
    for bad_html in [
        "",
        "<<<<>>>",
        "<input <input <input",
        "<!-- never closing comment ",
        r#"<input type="email" name="<script>alert(1)</script>">"#,
    ] {
        let r = parse_html(bad_html);
        check(r.is_ok(), format!("parse-able: {bad_html:?}"))?;
        let dom = r.unwrap();
        let _ = StaticHtmlEngine::extract_fields(&dom);
    }
    Ok(())
}

// ─── Snapshot tests ──────────────────────────────────────────────────

fn inspect_for_test(url: &str) -> Result<String, String> {
    let mut engine = StaticHtmlEngine::new().map_err(|e| format!("{e}"))?;
    engine.navigate(&url.parse().map_err(|e: _| format!("{e}"))?).map_err(|e| format!("{e}"))?;
    let snap = engine.snapshot_fields().map_err(|e| format!("{e}"))?;
    Ok(format!("URL: {}\nTitle: {}\n\n{}", snap.url, snap.title, snap.body))
}

fn snapshot_internal_page_home() -> Result<(), String> {
    let snap = inspect_for_test("atsisbroken://home")?;
    check_snapshot("src/browser/snapshots", "internal_page_home", &snap)
}

fn snapshot_internal_page_connections() -> Result<(), String> {
    let snap = inspect_for_test("atsisbroken://connections")?;
    check_snapshot("src/browser/snapshots", "internal_page_connections", &snap)
}

fn snapshot_internal_page_summary() -> Result<(), String> {
    let snap = inspect_for_test("atsisbroken://summary")?;
    check_snapshot("src/browser/snapshots", "internal_page_summary", &snap)
}

fn snapshot_internal_page_fingerprint() -> Result<(), String> {
    let snap = inspect_for_test("atsisbroken://fingerprint")?;
    check_snapshot("src/browser/snapshots", "internal_page_fingerprint", &snap)
}

fn snapshot_internal_page_shield() -> Result<(), String> {
    let snap = inspect_for_test("atsisbroken://shield")?;
    check_snapshot("src/browser/snapshots", "internal_page_shield", &snap)
}

fn snapshot_internal_page_not_found() -> Result<(), String> {
    let snap = inspect_for_test("atsisbroken://no-such-page/with/path")?;
    check_snapshot("src/browser/snapshots", "internal_page_not_found", &snap)
}

fn navigate_unsupported_scheme_errors() -> Result<(), String> {
    let mut e = StaticHtmlEngine::new().map_err(|e| format!("{e}"))?;
    let url: Url = "ftp://example.com/x".parse().map_err(|e: _| format!("{e}"))?;
    match e.navigate(&url) {
        Err(EngineError::Network(_)) => Ok(()),
        other => Err(format!("expected Network err, got {other:?}")),
    }
}

fn user_agent_includes_crate_version() -> Result<(), String> {
    let ua = default_user_agent();
    check(ua.contains("atsisbroken/"), "has atsisbroken/")?;
    check(ua.contains(env!("CARGO_PKG_VERSION")), "has version")
}
