// SPDX-License-Identifier: Unlicense
//! Real CDP-driven run loop. Lifted from the proven `tests/ats_e2e.rs`
//! pattern into a production code path so `atsisbroken run --url ...`
//! actually drives a browser, not a stub.
//!
//! The flow:
//!   1. Load profile from disk.
//!   2. Launch chromium via chromiumoxide (BrowserConfig auto-detect).
//!   3. Navigate to the user-supplied URL.
//!   4. Wait for hydration (1.5 s settle is good enough for static
//!      forms; multi-page wizards need explicit wait_for_navigation
//!      hooks added later).
//!   5. Snapshot every <input>/<textarea>/<select> via Runtime.evaluate.
//!   6. Classify each via `predict_field_key`.
//!   7. For each confident field with a profile value, fill via
//!      Runtime.evaluate (.value=..., dispatch input+change events).
//!   8. Save a screenshot of the filled state.
//!   9. Append `Observation` events to the on-disk feedback queue.
//!  10. Print a summary. **Never submit.** User clicks Submit.
//!
//! Constraint C1 (no auto-submit) is enforced by simply never calling
//! the submit button — the function exits after fill+screenshot+report.

use crate::{
    config::Config, paths, predict_field_key_with_confidence, profile_value_for_key, version,
    ConfidenceThreshold, Feedback, FeedbackQueue, FieldDescriptor, Mode, Profile,
};
use anyhow::{Context, Result};
use chromiumoxide::page::ScreenshotParams;
use chromiumoxide::Browser;
use futures::StreamExt;
use std::path::Path;
use std::time::Duration;

/// JS executed in the page's main world to enumerate every form field.
/// Same shape as the seed corpus FieldDescriptor: label / placeholder
/// / aria_label / name / id / kind.
const SNAPSHOT_JS: &str = r#"
(() => {
    const out = [];
    const all = document.querySelectorAll('input, textarea, select');
    for (const el of all) {
        const labelEl = (el.id && document.querySelector('label[for="' + el.id + '"]'))
            || el.closest('label')
            || (el.parentElement && el.parentElement.querySelector('span.wd-label'))
            || (el.parentElement && el.parentElement.querySelector('label.ashby-application-form-question-title'));
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

/// Per-field decision — pure function, no I/O. The run loop applies
/// it to every snapshotted field and dispatches accordingly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FillDecision {
    /// Skip — either the classifier returned unknown, or there's no
    /// matching profile value, or (in Shadow) confidence was below
    /// threshold.
    Skip { reason: SkipReason },
    /// Fill the field outright (Chaos mode, or Shadow above threshold).
    Fill,
    /// Ask the user yes/no per field (TrainingWheels mode).
    Prompt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipReason {
    /// Classifier returned `unknown` or empty key.
    NotClassified,
    /// Field key was classified but profile has no value for it
    /// (e.g. user didn't fill in `linkedin`).
    NoProfileValue,
    /// Field has no DOM id, so we can't reliably target it.
    NoDomId,
    /// Shadow mode: classifier confidence below threshold.
    BelowConfidenceThreshold,
}

/// Apply the autonomy-mode policy to one field. Pure; no I/O.
pub fn decide(
    mode: Mode,
    key: &str,
    confidence: f32,
    threshold: ConfidenceThreshold,
    has_profile_value: bool,
    has_dom_id: bool,
) -> FillDecision {
    if key.is_empty() {
        return FillDecision::Skip {
            reason: SkipReason::NotClassified,
        };
    }
    if !has_profile_value {
        return FillDecision::Skip {
            reason: SkipReason::NoProfileValue,
        };
    }
    if !has_dom_id {
        return FillDecision::Skip {
            reason: SkipReason::NoDomId,
        };
    }
    match mode {
        Mode::TrainingWheels => FillDecision::Prompt,
        Mode::Shadow => {
            if threshold.passes(confidence) {
                FillDecision::Fill
            } else {
                FillDecision::Skip {
                    reason: SkipReason::BelowConfidenceThreshold,
                }
            }
        }
        Mode::Chaos => FillDecision::Fill,
    }
}

/// JS template — fill one field by id and dispatch React-friendly events.
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

/// Result of one run_against_url call. Returned to the CLI for printing
/// or to the TUI for status panel updates.
#[derive(Debug, Clone)]
pub struct RunSummary {
    pub url: String,
    pub fields_seen: usize,
    pub fields_filled: usize,
    pub fields_skipped: usize,
    pub screenshot_path: Option<std::path::PathBuf>,
    /// Feedback events appended during this run.
    pub feedback_events: Vec<Feedback>,
}

/// End-to-end: launch → navigate → snapshot → classify → fill →
/// screenshot → persist → return summary. Never submits.
///
/// `screenshot_dir` may be None to skip screenshotting (TUI uses None;
/// the `run` subcommand defaults to ~/.atsisbroken/last-run.png).
pub async fn run_against_url(
    profile: &Profile,
    url: &str,
    screenshot_dir: Option<&Path>,
) -> Result<RunSummary> {
    let cfg = chromiumoxide::BrowserConfig::builder()
        .build()
        .map_err(|e| anyhow::anyhow!("browser config: {e}"))?;

    let (browser, mut handler) = Browser::launch(cfg)
        .await
        .map_err(|e| anyhow::anyhow!("browser launch: {e}"))?;
    let driver = tokio::spawn(async move { while handler.next().await.is_some() {} });

    // Wrap the actual work so we always shut down cleanly even on Err.
    let result = run_inner(&browser, profile, url, screenshot_dir).await;

    drop(browser);
    driver.abort();
    result
}

async fn run_inner(
    browser: &Browser,
    profile: &Profile,
    url: &str,
    screenshot_dir: Option<&Path>,
) -> Result<RunSummary> {
    let page = browser.new_page(url).await.context("new_page")?;
    tokio::time::sleep(Duration::from_millis(1500)).await;

    let snap = page
        .evaluate(SNAPSHOT_JS)
        .await
        .context("snapshot evaluate")?
        .into_value::<serde_json::Value>()
        .context("snapshot value")?;
    let descriptors: Vec<FieldDescriptor> =
        serde_json::from_value(snap).context("snapshot deserialize")?;

    let cfg = Config::load().unwrap_or_default();
    eprintln!(
        "  mode: {:?}{}",
        cfg.mode,
        if cfg.mode == Mode::Shadow {
            format!(" (confidence threshold: {:.2})", cfg.confidence_threshold.0)
        } else {
            String::new()
        }
    );

    let mut filled = 0usize;
    let mut skipped = 0usize;
    let mut prompted = 0usize;
    let mut feedback_events: Vec<Feedback> = Vec::new();

    for d in &descriptors {
        let (key, confidence) = predict_field_key_with_confidence(d);
        let value = profile_value_for_key(profile, key);
        let decision = decide(
            cfg.mode,
            key,
            confidence,
            cfg.confidence_threshold,
            value.is_some(),
            !d.id.is_empty(),
        );
        match decision {
            FillDecision::Skip { .. } => {
                skipped += 1;
            }
            FillDecision::Fill => {
                let v = value.expect("decide returned Fill ⇒ value present");
                let _ = page.evaluate(fill_js(&d.id, v)).await;
                filled += 1;
                feedback_events.push(Feedback {
                    field: d.clone(),
                    predicted: key.to_string(),
                    actual: key.to_string(),
                    accepted: true,
                });
            }
            FillDecision::Prompt => {
                let v = value.expect("decide returned Prompt ⇒ value present");
                prompted += 1;
                if prompt_user(&d.label, key, v)? {
                    let _ = page.evaluate(fill_js(&d.id, v)).await;
                    filled += 1;
                    feedback_events.push(Feedback {
                        field: d.clone(),
                        predicted: key.to_string(),
                        actual: key.to_string(),
                        accepted: true,
                    });
                } else {
                    skipped += 1;
                    feedback_events.push(Feedback {
                        field: d.clone(),
                        predicted: key.to_string(),
                        actual: String::new(), // user rejected
                        accepted: false,
                    });
                }
            }
        }
    }
    let _ = prompted; // retained for future summary surface

    let screenshot_path = if let Some(dir) = screenshot_dir {
        std::fs::create_dir_all(dir).context("mkdir screenshot dir")?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let out = dir.join(format!("run-{stamp}.png"));
        page.save_screenshot(
            ScreenshotParams::builder()
                .format(
                    chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat::Png,
                )
                .full_page(true)
                .build(),
            &out,
        )
        .await
        .context("save_screenshot")?;
        Some(out)
    } else {
        None
    };

    Ok(RunSummary {
        url: url.to_string(),
        fields_seen: descriptors.len(),
        fields_filled: filled,
        fields_skipped: skipped,
        screenshot_path,
        feedback_events,
    })
}

/// TrainingWheels prompt: ask the user yes/no per field on stderr +
/// stdin. Returns true on yes, false on no/empty/garbage. Designed
/// for terminal use; TUI integration will route through a different
/// surface later.
fn prompt_user(label: &str, key: &str, value: &str) -> Result<bool> {
    use std::io::{BufRead, Write};
    eprint!(
        "  field {label:?} → {key} = {value:?} — fill? [y/N] ",
        label = label,
        key = key,
        value = value
    );
    std::io::stderr().flush().ok();
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line)?;
    let answer = line.trim().to_ascii_lowercase();
    Ok(answer == "y" || answer == "yes")
}

/// Append the run's feedback events to the persistent queue at the
/// canonical path. Returns the new queue length.
pub fn persist_feedback(events: Vec<Feedback>) -> Result<usize> {
    let path = paths::feedback_jsonl_path();
    let mut q = FeedbackQueue::load_from(&path).context("load feedback queue")?;
    for ev in events {
        q.append(ev);
    }
    paths::ensure_dir()?;
    q.save_to(&path).context("save feedback queue")?;
    Ok(q.len())
}

/// User-friendly one-line summary suitable for stderr.
pub fn format_summary(s: &RunSummary, queue_depth: usize) -> String {
    let shot = match &s.screenshot_path {
        Some(p) => format!("\n  screenshot: {}", p.display()),
        None => String::new(),
    };
    format!(
        "atsisbroken {ver}\n  url: {url}\n  fields seen: {seen}\n  filled: {filled}\n  skipped (unknown / freetext / empty): {skipped}\n  feedback queue: {q} events on disk{shot}\n  ⚠ NOT submitted — review in browser, click Submit yourself.",
        ver = version(),
        url = s.url,
        seen = s.fields_seen,
        filled = s.fields_filled,
        skipped = s.fields_skipped,
        q = queue_depth,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_js_escapes_single_quotes() {
        let js = fill_js("foo", "Jane's resume");
        // `'` must appear escaped inside the literal so the JS string
        // doesn't terminate early.
        assert!(js.contains("Jane\\'s resume"));
    }

    #[test]
    fn fill_js_escapes_backslash() {
        let js = fill_js("foo", r"C:\path");
        // Backslash must be doubled so the JS literal stays valid.
        assert!(js.contains(r"C:\\path"));
    }

    #[test]
    fn fill_js_uses_id_in_getelementbyid() {
        let js = fill_js("legalNameSection_firstName", "Jane");
        assert!(js.contains("getElementById('legalNameSection_firstName')"));
    }

    #[test]
    fn fill_js_dispatches_input_and_change_events() {
        // React listens on `input` for controlled inputs; some legacy
        // form code listens only on `change`. Both must fire or the
        // form's internal state desyncs from the DOM value.
        let js = fill_js("foo", "bar");
        assert!(js.contains("new Event('input'"));
        assert!(js.contains("new Event('change'"));
    }

    #[test]
    fn format_summary_warns_about_not_submitting() {
        let s = RunSummary {
            url: "https://example.com".into(),
            fields_seen: 10,
            fields_filled: 7,
            fields_skipped: 3,
            screenshot_path: None,
            feedback_events: vec![],
        };
        let out = format_summary(&s, 0);
        assert!(out.contains("NOT submitted"));
        assert!(out.contains("review"));
    }

    // ─── decide ───────────────────────────────────────────────────────

    fn t() -> ConfidenceThreshold {
        ConfidenceThreshold(0.85)
    }

    #[test]
    fn decide_skip_when_classifier_returns_unknown() {
        let d = decide(Mode::Chaos, "", 0.0, t(), true, true);
        assert_eq!(
            d,
            FillDecision::Skip {
                reason: SkipReason::NotClassified
            }
        );
    }

    #[test]
    fn decide_skip_when_no_profile_value() {
        let d = decide(Mode::Chaos, "linkedin", 1.0, t(), false, true);
        assert_eq!(
            d,
            FillDecision::Skip {
                reason: SkipReason::NoProfileValue
            }
        );
    }

    #[test]
    fn decide_skip_when_no_dom_id() {
        let d = decide(Mode::Chaos, "email", 1.0, t(), true, false);
        assert_eq!(
            d,
            FillDecision::Skip {
                reason: SkipReason::NoDomId
            }
        );
    }

    #[test]
    fn decide_chaos_fills_classified_field() {
        let d = decide(Mode::Chaos, "email", 1.0, t(), true, true);
        assert_eq!(d, FillDecision::Fill);
    }

    #[test]
    fn decide_training_wheels_always_prompts_classified_fields() {
        let d = decide(Mode::TrainingWheels, "email", 1.0, t(), true, true);
        assert_eq!(d, FillDecision::Prompt);
    }

    #[test]
    fn decide_shadow_fills_above_threshold() {
        let d = decide(Mode::Shadow, "email", 0.90, t(), true, true);
        assert_eq!(d, FillDecision::Fill);
    }

    #[test]
    fn decide_shadow_fills_at_threshold() {
        // Property: threshold is inclusive (>=). Catches accidental
        // > → >= flip in `ConfidenceThreshold::passes`.
        let d = decide(Mode::Shadow, "email", 0.85, t(), true, true);
        assert_eq!(d, FillDecision::Fill);
    }

    #[test]
    fn decide_shadow_skips_below_threshold() {
        let d = decide(Mode::Shadow, "email", 0.84, t(), true, true);
        assert_eq!(
            d,
            FillDecision::Skip {
                reason: SkipReason::BelowConfidenceThreshold
            }
        );
    }

    #[test]
    fn decide_skip_precedence_unknown_beats_threshold() {
        // If the field can't be classified at all, it doesn't matter
        // what mode/threshold says. NotClassified wins.
        let d = decide(Mode::Shadow, "", 1.0, t(), true, true);
        assert_eq!(
            d,
            FillDecision::Skip {
                reason: SkipReason::NotClassified
            }
        );
    }

    #[test]
    fn decide_skip_precedence_no_value_beats_prompt() {
        // TrainingWheels would normally Prompt, but if the user has
        // no value to fill we skip rather than prompt for nothing.
        let d = decide(Mode::TrainingWheels, "linkedin", 1.0, t(), false, true);
        assert_eq!(
            d,
            FillDecision::Skip {
                reason: SkipReason::NoProfileValue
            }
        );
    }

    #[test]
    fn format_summary_shows_screenshot_path_when_set() {
        let s = RunSummary {
            url: "u".into(),
            fields_seen: 0,
            fields_filled: 0,
            fields_skipped: 0,
            screenshot_path: Some(std::path::PathBuf::from("/tmp/run-1234.png")),
            feedback_events: vec![],
        };
        let out = format_summary(&s, 0);
        assert!(out.contains("/tmp/run-1234.png"));
    }
}
