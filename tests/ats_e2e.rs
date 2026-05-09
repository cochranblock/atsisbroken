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
//!
//! Gated on `legacy_cdp_e2e` — the in-tree kova `ats_fixtures` module
//! is only available when the workspace's kova-engine has it bundled.
//! When building against an older kova shape (or the new Servo-backed
//! engine path), this entire test file becomes a no-op.

#![cfg(feature = "legacy_cdp_e2e")]

use atsisbroken::{predict_field_key, profile_value_for_key, FieldDescriptor, Profile};
use chromiumoxide::page::ScreenshotParams;
use chromiumoxide::Browser;
use futures::StreamExt;
use atsisbroken::ats_fixtures::{self, AtsVendor, FixtureOpts};
use std::time::Duration;

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

    let profile = sample_profile();
    let opts = FixtureOpts::default();
    let cwd = std::env::current_dir().expect("cwd");
    let screenshots_dir = cwd.join("docs/screenshots/ats");
    std::fs::create_dir_all(&screenshots_dir).expect("mkdir screenshots/ats");

    let vendors = [
        AtsVendor::Greenhouse,
        AtsVendor::Lever,
        AtsVendor::Workday,
        AtsVendor::Icims,
        AtsVendor::Ashby,
    ];

    for vendor in vendors {
        eprintln!("--- vendor: {}", vendor.label());
        let html = ats_fixtures::render(vendor, &opts);
        // Use a data: URL so the browser parses inline HTML — no
        // file I/O required, and the fixture stays a pure string.
        let data_url = format!(
            "data:text/html;base64,{}",
            data_url_base64_encode(&html)
        );
        let page = browser.new_page(&data_url).await.expect("new_page");
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

        // Assert each (id, expected_key) pair from the kova capability.
        let expected = ats_fixtures::expected_keys(vendor, &opts);
        for (id, expected_key) in &expected {
            let got = predicted_by_id
                .get(id)
                .cloned()
                .unwrap_or_else(|| "<missing>".into());
            assert_eq!(
                got, *expected_key,
                "vendor {} field id={}: predicted {got:?}, expected {:?}",
                vendor.label(),
                id,
                expected_key
            );
        }

        // Fill every field with a non-empty matching profile value.
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

        let out = screenshots_dir.join(format!("{}.png", vendor.label()));
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

fn data_url_base64_encode(s: &str) -> String {
    // Tiny base64 encoder so we don't pull in a dep just for tests.
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = s.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i + 3 <= bytes.len() {
        let n = ((bytes[i] as u32) << 16) | ((bytes[i + 1] as u32) << 8) | bytes[i + 2] as u32;
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        out.push(TABLE[((n >> 6) & 63) as usize] as char);
        out.push(TABLE[(n & 63) as usize] as char);
        i += 3;
    }
    let rem = bytes.len() - i;
    if rem == 1 {
        let n = (bytes[i] as u32) << 16;
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        out.push_str("==");
    } else if rem == 2 {
        let n = ((bytes[i] as u32) << 16) | ((bytes[i + 1] as u32) << 8);
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        out.push(TABLE[((n >> 6) & 63) as usize] as char);
        out.push('=');
    }
    out
}
