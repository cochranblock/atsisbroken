// SPDX-License-Identifier: Unlicense

//! Tests for `crate::browser::shell` — converted from
//! `#[cfg(test)] mod tests {}` (Phase 3).

use crate::browser::shell::compose_page_text;
use crate::browser::{BrowserConfig, EngineError, PageSnapshot, Url};

use super::{case, check, check_eq, TestResult};

pub fn run() -> Vec<TestResult> {
    vec![
        case("shell::default_config_starts_on_network_landing_page",
             default_config_starts_on_network_landing_page),
        case("shell::default_config_title_includes_crate_version",
             default_config_title_includes_crate_version),
        case("shell::default_config_pins_webdriver_field_false",
             default_config_pins_webdriver_field_false),
        case("shell::compose_page_text_uses_snapshot_title_when_present",
             compose_page_text_uses_snapshot_title_when_present),
        case("shell::compose_page_text_falls_back_to_host_when_title_empty",
             compose_page_text_falls_back_to_host_when_title_empty),
        case("shell::compose_page_text_renders_navigate_error_into_body",
             compose_page_text_renders_navigate_error_into_body),
        case("shell::internal_url_takes_synchronous_path",
             internal_url_takes_synchronous_path),
        case("shell::network_url_takes_async_worker_path",
             network_url_takes_async_worker_path),
    ]
}

fn url_for_test(s: &str) -> Url {
    s.parse().expect("test URL must parse")
}

fn default_config_starts_on_network_landing_page() -> Result<(), String> {
    // The default home is the live landing page. Network
    // navigation goes through the audit-fix-#11 worker thread,
    // so the OS event loop never blocks on the initial fetch.
    let c = BrowserConfig::default();
    check(c.start_url.is_network(), "start_url should be network")?;
    check_eq(c.start_url.host(), "atsisbroken.cochranblock.org", "start_url host")
}

fn default_config_title_includes_crate_version() -> Result<(), String> {
    let c = BrowserConfig::default();
    check(c.title.contains("atsisbroken"), "title contains 'atsisbroken'")?;
    check(
        c.title.contains(env!("CARGO_PKG_VERSION")),
        format!("title should contain version: {}", c.title),
    )
}

fn default_config_pins_webdriver_field_false() -> Result<(), String> {
    // This pins the struct default — the FingerprintProfile we
    // hand the rest of the system says webdriver=false. Not a
    // runtime guarantee against a live page; the JS engine
    // (mozjs) hasn't landed.
    let c = BrowserConfig::default();
    check(!c.fingerprint.navigator_webdriver, "navigator_webdriver default")
}

fn compose_page_text_uses_snapshot_title_when_present() -> Result<(), String> {
    let url = url_for_test("https://example.com/");
    let snap = PageSnapshot {
        url: url.to_string(),
        title: "Example Domain".into(),
        body: "This domain is for use in illustrative examples.".into(),
        fields: Vec::new(),
    };
    let (title, body) = compose_page_text(&url, Ok(()), Ok(snap));
    check_eq(title, "Example Domain".to_string(), "title")?;
    check(body.contains("illustrative"), "body contains illustrative")
}

fn compose_page_text_falls_back_to_host_when_title_empty() -> Result<(), String> {
    // Network pages without a <title> tag use the URL host as
    // the title — matches the address-bar idiom of "you're at
    // example.com" when the page hasn't named itself.
    let url = url_for_test("https://example.com/");
    let snap = PageSnapshot {
        url: url.to_string(),
        title: String::new(),
        body: "body text".into(),
        fields: Vec::new(),
    };
    let (title, body) = compose_page_text(&url, Ok(()), Ok(snap));
    check_eq(title, "example.com".to_string(), "title falls back to host")?;
    check_eq(body, "body text".to_string(), "body")
}

fn compose_page_text_renders_navigate_error_into_body() -> Result<(), String> {
    // Failed navigation renders the error inline so the user sees
    // it in the window — not just on stderr.
    let url = url_for_test("https://example.com/");
    let nav_err = Err(EngineError::Network("dns: no route to host".into()));
    let snap_err = Err(EngineError::Parse("no page loaded".into()));
    let (title, body) = compose_page_text(&url, nav_err, snap_err);
    check_eq(title, "atsisbroken".to_string(), "error title")?;
    check(body.contains("Couldn't load"), "body says Couldn't load")?;
    check(body.contains("https://example.com/"), "body cites URL")?;
    check(body.contains("dns: no route to host"), "body cites error")
}

fn internal_url_takes_synchronous_path() -> Result<(), String> {
    // The dispatch in `resumed` keys off `is_internal()`. Pin
    // the predicate so a URL refactor doesn't silently route an
    // internal page through the network worker.
    let url = url_for_test("atsisbroken://home");
    check(url.is_internal(), "should be internal")?;
    check(!url.is_network(), "should not be network")
}

fn network_url_takes_async_worker_path() -> Result<(), String> {
    let url = url_for_test("https://boards.greenhouse.io/example/jobs/123");
    check(!url.is_internal(), "should not be internal")?;
    check(url.is_network(), "should be network")
}
