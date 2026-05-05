// SPDX-License-Identifier: Unlicense
//! ATS fixture end-to-end. Launches chromium (kova's BrowserConfig +
//! BrowserFetcher pattern), navigates to each fixture via `file://`,
//! snapshots form fields via `page.evaluate`, runs them through the
//! Rust classifier, and asserts predictions match `expected.toml`.
//!
//! Then fills the matched fields with a sample profile's values and
//! saves a PNG screenshot per fixture to `docs/screenshots/ats/<vendor>.png`
//! so the receipts of "we filled it" are inspectable.
//!
//! When chromium isn't installed, the BrowserFetcher would download
//! ~150 MB. CI without internet skips that path; the test marks itself
//! as skipped rather than failing.

use atsisbroken::{predict_field_key, profile_value_for_key, FieldDescriptor, Profile};
use chromiumoxide::page::ScreenshotParams;
use chromiumoxide::Browser;
use futures::StreamExt;
use serde::Deserialize;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct Expected {
    fixture: Vec<FixtureExpectation>,
}

#[derive(Debug, Deserialize)]
struct FixtureExpectation {
    file: String,
    expectations: Vec<FieldExpectation>,
}

#[derive(Debug, Deserialize)]
struct FieldExpectation {
    id: String,
    expected: String,
}

const SNAPSHOT_JS: &str = r#"
(() => {
    const out = [];
    const all = document.querySelectorAll('input, textarea, select');
    for (const el of all) {
        const labelEl = (el.id && document.querySelector('label[for="' + el.id + '"]'))
            || el.closest('label')
            || (el.parentElement && el.parentElement.querySelector('span.wd-label'));
        out.push({
            label: labelEl ? labelEl.innerText.trim() : '',
            placeholder: el.placeholder || '',
            aria_label: el.getAttribute('aria-label') || '',
            name: el.name || '',
            id: el.id || '',
            kind: (el.tagName.toLowerCase() === 'textarea')
                  ? 'textarea'
                  : (el.tagName.toLowerCase() === 'select')
                  ? 'select'
                  : (el.type || 'text'),
        });
    }
    return out;
})()
"#;

fn fill_js(id: &str, value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('\'', "\\'");
    format!(
        r#"
(() => {{
    const el = document.getElementById('{id}');
    if (!el) return false;
    el.value = '{escaped}';
    el.dispatchEvent(new Event('input', {{ bubbles: true }}));
    el.dispatchEvent(new Event('change', {{ bubbles: true }}));
    return true;
}})()
"#
    )
}

async fn browser_config() -> Result<chromiumoxide::BrowserConfig, String> {
    // Try auto-detect. If the user has no Chromium-family browser installed,
    // skip rather than auto-downloading 150MB — that would be a bad CI default.
    chromiumoxide::BrowserConfig::builder()
        .build()
        .map_err(|e| format!("config: {e}"))
}

fn sample_profile() -> Profile {
    Profile {
        full_name: "Jane Q. Doe".into(),
        email: "jane.doe@example.com".into(),
        phone: "+1-555-010-2030".into(),
        linkedin: "https://linkedin.com/in/janedoe".into(),
        github: "https://github.com/janedoe".into(),
        website: "https://janedoe.dev".into(),
        address: "1 Main St, Anywhere, CA 94000".into(),
        work_authorization: "US Citizen".into(),
        years_experience: 7,
        ..Default::default()
    }
}

#[tokio::test]
async fn ats_fixtures_classify_and_fill_correctly() {
    let cfg = match browser_config().await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("skipping ATS e2e: {e}");
            return;
        }
    };

    let (browser, mut handler) = match Browser::launch(cfg).await {
        Ok(b) => b,
        Err(e) => {
            eprintln!("skipping ATS e2e: launch failed: {e}");
            return;
        }
    };
    let driver = tokio::spawn(async move { while handler.next().await.is_some() {} });

    let expected_text = std::fs::read_to_string("tests/fixtures/ats/expected.toml")
        .expect("read expected.toml");
    let expected: Expected = toml::from_str(&expected_text).expect("parse expected.toml");
    let profile = sample_profile();

    let cwd = std::env::current_dir().expect("cwd");
    let screenshots_dir = cwd.join("docs/screenshots/ats");
    std::fs::create_dir_all(&screenshots_dir).expect("mkdir screenshots/ats");

    for fx in &expected.fixture {
        let url = format!(
            "file://{}",
            cwd.join("tests/fixtures/ats").join(&fx.file).display()
        );
        eprintln!("--- fixture: {}", fx.file);
        let page = browser.new_page(&url).await.expect("new_page");
        // Tiny settle for layout / inline DOM.
        tokio::time::sleep(Duration::from_millis(250)).await;

        let descriptors_json = page
            .evaluate(SNAPSHOT_JS)
            .await
            .expect("snapshot evaluate")
            .into_value::<serde_json::Value>()
            .expect("snapshot result is JSON");

        let descriptors: Vec<FieldDescriptor> = serde_json::from_value(descriptors_json)
            .expect("snapshot result deserialize");

        // Classify every snapshotted field. Build a map from id -> predicted.
        let mut predicted_by_id: std::collections::BTreeMap<String, String> = Default::default();
        for d in &descriptors {
            let key = predict_field_key(d).to_string();
            if !d.id.is_empty() {
                predicted_by_id.insert(d.id.clone(), key);
            }
        }

        // Assert each expected (id, expected_key) matches predicted.
        for ex in &fx.expectations {
            let got = predicted_by_id
                .get(&ex.id)
                .cloned()
                .unwrap_or_else(|| "<missing>".into());
            assert_eq!(
                got, ex.expected,
                "fixture {} field id={}: predicted {got:?}, expected {:?}",
                fx.file, ex.id, ex.expected
            );
        }

        // Fill every field that has a non-empty predicted key with a
        // matching profile value. (Skip "freetext" and "" — those don't
        // come from the structured profile.)
        let mut filled = 0;
        for d in &descriptors {
            let key = predict_field_key(d);
            if let Some(value) = profile_value_for_key(&profile, key) {
                if d.id.is_empty() {
                    continue;
                }
                let js = fill_js(&d.id, value);
                let _ = page.evaluate(js).await;
                filled += 1;
            }
        }

        // Save screenshot of the filled form.
        let vendor = fx.file.trim_end_matches(".html");
        let out = screenshots_dir.join(format!("{vendor}.png"));
        page.save_screenshot(
            ScreenshotParams::builder()
                .format(chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat::Png)
                .full_page(true)
                .build(),
            &out,
        )
        .await
        .expect("screenshot");
        eprintln!("  filled {filled} fields, screenshot: {}", out.display());
    }

    drop(browser);
    driver.abort();
}
