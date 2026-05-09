// SPDX-License-Identifier: Unlicense

//! Tests for `crate::browser_detect` (Phase 5).

use crate::browser_detect::{
    bundle_id_to_kind, default_browser, parse_macos_launchservices_http_handler,
    parse_windows_progid, parse_xdg_desktop_name, BrowserKind,
};

use super::{case, check, check_eq, TestResult};

pub fn run() -> Vec<TestResult> {
    vec![
        case("browser_detect::parse_xdg_desktop_firefox", parse_xdg_desktop_firefox),
        case("browser_detect::parse_xdg_desktop_chrome_chromium_distinct",
             parse_xdg_desktop_chrome_chromium_distinct),
        case("browser_detect::parse_xdg_desktop_brave_edge_vivaldi_opera_arc",
             parse_xdg_desktop_brave_edge_vivaldi_opera_arc),
        case("browser_detect::parse_xdg_desktop_unknown_returns_other_with_basename",
             parse_xdg_desktop_unknown_returns_other_with_basename),
        case("browser_detect::parse_macos_handler_chrome", parse_macos_handler_chrome),
        case("browser_detect::parse_macos_handler_firefox", parse_macos_handler_firefox),
        case("browser_detect::parse_macos_handler_returns_none_when_no_http_scheme",
             parse_macos_handler_returns_none_when_no_http_scheme),
        case("browser_detect::parse_macos_handler_picks_only_http_not_https",
             parse_macos_handler_picks_only_http_not_https),
        case("browser_detect::bundle_id_chrome", bundle_id_chrome),
        case("browser_detect::bundle_id_firefox_variants", bundle_id_firefox_variants),
        case("browser_detect::bundle_id_arc", bundle_id_arc),
        case("browser_detect::windows_progid_chrome_firefox_edge",
             windows_progid_chrome_firefox_edge),
        case("browser_detect::windows_progid_unknown_returns_other_preserving_case",
             windows_progid_unknown_returns_other_preserving_case),
        case("browser_detect::supports_cdp_excludes_only_firefox", supports_cdp_excludes_only_firefox),
        case("browser_detect::default_browser_path_does_not_panic_on_real_host",
             default_browser_path_does_not_panic_on_real_host),
    ]
}

fn parse_xdg_desktop_firefox() -> Result<(), String> {
    check_eq(parse_xdg_desktop_name("firefox.desktop"), BrowserKind::Firefox, "firefox.desktop")?;
    check_eq(
        parse_xdg_desktop_name("firefox-esr.desktop"),
        BrowserKind::Firefox,
        "firefox-esr.desktop",
    )
}

fn parse_xdg_desktop_chrome_chromium_distinct() -> Result<(), String> {
    check_eq(
        parse_xdg_desktop_name("google-chrome.desktop"),
        BrowserKind::Chrome,
        "google-chrome",
    )?;
    check_eq(parse_xdg_desktop_name("chromium.desktop"), BrowserKind::Chromium, "chromium")?;
    check_eq(
        parse_xdg_desktop_name("chromium-browser.desktop"),
        BrowserKind::Chromium,
        "chromium-browser",
    )
}

fn parse_xdg_desktop_brave_edge_vivaldi_opera_arc() -> Result<(), String> {
    check_eq(parse_xdg_desktop_name("brave-browser.desktop"), BrowserKind::Brave, "brave")?;
    check_eq(parse_xdg_desktop_name("microsoft-edge.desktop"), BrowserKind::Edge, "edge")?;
    check_eq(parse_xdg_desktop_name("vivaldi-stable.desktop"), BrowserKind::Vivaldi, "vivaldi")?;
    check_eq(parse_xdg_desktop_name("opera.desktop"), BrowserKind::Opera, "opera")?;
    check_eq(parse_xdg_desktop_name("arc.desktop"), BrowserKind::Arc, "arc")
}

fn parse_xdg_desktop_unknown_returns_other_with_basename() -> Result<(), String> {
    match parse_xdg_desktop_name("snortwidget.desktop") {
        BrowserKind::Other(s) => check_eq(s, "snortwidget".to_string(), "Other basename"),
        other => Err(format!("expected Other variant, got {other:?}")),
    }
}

fn parse_macos_handler_chrome() -> Result<(), String> {
    let plist = r#"
{
    LSHandlers =     (
                {
            LSHandlerContentType = "public.plain-text";
            LSHandlerRoleAll = "com.apple.textedit";
        },
                {
            LSHandlerRoleAll = "com.google.chrome";
            LSHandlerURLScheme = http;
        },
                {
            LSHandlerRoleAll = "com.google.chrome";
            LSHandlerURLScheme = https;
        }
    );
}
"#;
    check_eq(
        parse_macos_launchservices_http_handler(plist),
        Some("com.google.chrome".to_string()),
        "chrome handler",
    )
}

fn parse_macos_handler_firefox() -> Result<(), String> {
    let plist = r#"
{
    LSHandlers = (
        {
            LSHandlerRoleAll = "org.mozilla.firefox";
            LSHandlerURLScheme = http;
        }
    );
}
"#;
    check_eq(
        parse_macos_launchservices_http_handler(plist),
        Some("org.mozilla.firefox".to_string()),
        "firefox handler",
    )
}

fn parse_macos_handler_returns_none_when_no_http_scheme() -> Result<(), String> {
    let plist = r#"
{
    LSHandlers = (
        {
            LSHandlerRoleAll = "com.apple.textedit";
            LSHandlerContentType = "public.plain-text";
        }
    );
}
"#;
    check_eq(
        parse_macos_launchservices_http_handler(plist),
        None,
        "no http scheme → None",
    )
}

fn parse_macos_handler_picks_only_http_not_https() -> Result<(), String> {
    let plist = r#"
{
    LSHandlers = (
        {
            LSHandlerRoleAll = "https.handler";
            LSHandlerURLScheme = https;
        },
        {
            LSHandlerRoleAll = "http.handler";
            LSHandlerURLScheme = http;
        }
    );
}
"#;
    check_eq(
        parse_macos_launchservices_http_handler(plist),
        Some("http.handler".to_string()),
        "http wins over https",
    )
}

fn bundle_id_chrome() -> Result<(), String> {
    check_eq(bundle_id_to_kind("com.google.chrome"), BrowserKind::Chrome, "chrome")?;
    check_eq(
        bundle_id_to_kind("com.google.chrome.canary"),
        BrowserKind::Chrome,
        "chrome.canary",
    )
}

fn bundle_id_firefox_variants() -> Result<(), String> {
    check_eq(
        bundle_id_to_kind("org.mozilla.firefox"),
        BrowserKind::Firefox,
        "firefox",
    )?;
    check_eq(
        bundle_id_to_kind("org.mozilla.firefoxnightly"),
        BrowserKind::Firefox,
        "firefox-nightly",
    )
}

fn bundle_id_arc() -> Result<(), String> {
    check_eq(
        bundle_id_to_kind("company.thebrowser.browser"),
        BrowserKind::Arc,
        "Arc bundle",
    )
}

fn windows_progid_chrome_firefox_edge() -> Result<(), String> {
    check_eq(parse_windows_progid("ChromeHTML"), BrowserKind::Chrome, "ChromeHTML")?;
    check_eq(parse_windows_progid("FirefoxURL-1234"), BrowserKind::Firefox, "FirefoxURL")?;
    check_eq(parse_windows_progid("MSEdgeHTM"), BrowserKind::Edge, "MSEdgeHTM")?;
    check_eq(parse_windows_progid("BraveHTML"), BrowserKind::Brave, "BraveHTML")
}

fn windows_progid_unknown_returns_other_preserving_case() -> Result<(), String> {
    match parse_windows_progid("WeirdBrowserHTML") {
        BrowserKind::Other(s) => check_eq(s, "WeirdBrowserHTML".to_string(), "Other case-preserved"),
        other => Err(format!("expected Other variant, got {other:?}")),
    }
}

fn supports_cdp_excludes_only_firefox() -> Result<(), String> {
    check(BrowserKind::Chrome.supports_cdp(), "Chrome supports CDP")?;
    check(BrowserKind::Chromium.supports_cdp(), "Chromium supports CDP")?;
    check(BrowserKind::Edge.supports_cdp(), "Edge supports CDP")?;
    check(BrowserKind::Brave.supports_cdp(), "Brave supports CDP")?;
    check(BrowserKind::Arc.supports_cdp(), "Arc supports CDP")?;
    check(BrowserKind::Vivaldi.supports_cdp(), "Vivaldi supports CDP")?;
    check(BrowserKind::Opera.supports_cdp(), "Opera supports CDP")?;
    check(!BrowserKind::Firefox.supports_cdp(), "Firefox does NOT support CDP")
}

fn default_browser_path_does_not_panic_on_real_host() -> Result<(), String> {
    // Real value: this fn shells out to `xdg-settings` (Linux) /
    // `defaults read` (macOS) / scans PATH (Windows). Each branch
    // could panic on an unexpected exit code, malformed plist, or
    // a missing env var. Path coverage (executes without panic) is
    // the contract.
    let _ = default_browser();
    Ok(())
}
