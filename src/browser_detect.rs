// SPDX-License-Identifier: Unlicense
//! Default-browser detection per OS.
//!
//! Linux:   `xdg-settings get default-web-browser` returns a `.desktop`
//!          filename; we infer kind from the basename and resolve the
//!          executable via `which`.
//! macOS:   `defaults read com.apple.LaunchServices/com.apple.launchservices.secure`
//!          enumerates handler bundle IDs; we grep for the `http` row
//!          and map to a known app path.
//! Windows: registry-based — `HKCU\SOFTWARE\Microsoft\Windows\Shell\
//!          Associations\UrlAssociations\http\UserChoice\ProgId`.
//!          Stubbed for now (requires `winreg`); falls back to PATH-based
//!          discovery on Windows builds.
//!
//! All three OS branches share a parser layer that's pure-string and
//! unit-testable without spawning subprocesses.

use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserKind {
    Chrome,
    Chromium,
    Firefox,
    Edge,
    Brave,
    Arc,
    Vivaldi,
    Opera,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultBrowser {
    pub kind: BrowserKind,
    pub path: Option<PathBuf>,
}

impl BrowserKind {
    /// True if this browser speaks CDP. Firefox uses Marionette/WebDriver
    /// instead, so it falls through to userscript in the strategy ladder.
    pub fn supports_cdp(&self) -> bool {
        !matches!(self, BrowserKind::Firefox)
    }
}

/// Best-effort cross-platform detection. Returns None when no signal
/// is available (e.g. headless CI without a default-browser preference).
pub fn default_browser() -> Option<DefaultBrowser> {
    #[cfg(target_os = "linux")]
    return linux::detect();
    #[cfg(target_os = "macos")]
    return macos::detect();
    #[cfg(target_os = "windows")]
    return windows::detect();
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    return None;
}

// ─── Pure parsers (testable without subprocess) ──────────────────────────

/// Linux: map a `.desktop` filename to a [`BrowserKind`].
pub fn parse_xdg_desktop_name(desktop_name: &str) -> BrowserKind {
    let lc = desktop_name.to_ascii_lowercase();
    if lc.contains("firefox") {
        BrowserKind::Firefox
    } else if lc.contains("google-chrome") || lc.contains("chrome.desktop") {
        BrowserKind::Chrome
    } else if lc.contains("chromium") {
        BrowserKind::Chromium
    } else if lc.contains("brave") {
        BrowserKind::Brave
    } else if lc.contains("microsoft-edge") || lc.contains("msedge") {
        BrowserKind::Edge
    } else if lc.contains("vivaldi") {
        BrowserKind::Vivaldi
    } else if lc.contains("opera") {
        BrowserKind::Opera
    } else if lc.contains("arc") {
        BrowserKind::Arc
    } else {
        // Strip ".desktop" suffix when surfacing to the user.
        let trimmed = lc.trim_end_matches(".desktop").to_string();
        BrowserKind::Other(trimmed)
    }
}

/// macOS: from a `defaults read` plist excerpt, find the bundle ID
/// registered for the `http` URL scheme. Plist format is the Apple
/// pseudo-JSON that `defaults read` emits.
pub fn parse_macos_launchservices_http_handler(plist_text: &str) -> Option<String> {
    // Look for blocks shaped like:
    //   {
    //       LSHandlerRoleAll = "com.google.chrome";
    //       LSHandlerURLScheme = http;
    //   }
    // Block-level walk; track the most recent LSHandlerRoleAll seen
    // until we close the block, and emit it iff that block contained
    // LSHandlerURLScheme = http.
    let mut current_role: Option<String> = None;
    let mut seen_http_scheme = false;
    let mut http_handler: Option<String> = None;

    for line in plist_text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('{') {
            current_role = None;
            seen_http_scheme = false;
        } else if trimmed.starts_with('}') {
            if seen_http_scheme {
                http_handler = current_role.clone();
            }
            current_role = None;
            seen_http_scheme = false;
        } else if let Some(rest) = trimmed.strip_prefix("LSHandlerRoleAll = ") {
            // Strip trailing semicolon and surrounding quotes.
            let val = rest.trim_end_matches(';').trim_matches('"');
            current_role = Some(val.to_string());
        } else if let Some(rest) = trimmed.strip_prefix("LSHandlerURLScheme = ") {
            let val = rest.trim_end_matches(';').trim_matches('"');
            if val == "http" {
                seen_http_scheme = true;
            }
        }
    }
    http_handler
}

/// Map a macOS bundle ID to a [`BrowserKind`].
pub fn bundle_id_to_kind(bundle_id: &str) -> BrowserKind {
    let lc = bundle_id.to_ascii_lowercase();
    if lc == "com.google.chrome" || lc == "com.google.chrome.canary" {
        BrowserKind::Chrome
    } else if lc.starts_with("org.mozilla.firefox") || lc.starts_with("org.mozilla.nightly") {
        BrowserKind::Firefox
    } else if lc == "org.chromium.chromium" {
        BrowserKind::Chromium
    } else if lc.starts_with("com.brave.browser") {
        BrowserKind::Brave
    } else if lc.starts_with("com.microsoft.edgemac") {
        BrowserKind::Edge
    } else if lc == "company.thebrowser.browser" {
        BrowserKind::Arc
    } else if lc.starts_with("com.vivaldi") {
        BrowserKind::Vivaldi
    } else if lc.starts_with("com.operasoftware") {
        BrowserKind::Opera
    } else {
        BrowserKind::Other(bundle_id.to_string())
    }
}

/// Windows: map a registry ProgId (e.g. `ChromeHTML`, `FirefoxURL`) to a
/// [`BrowserKind`]. Pure string analysis; the registry read happens in
/// the platform-gated module.
pub fn parse_windows_progid(progid: &str) -> BrowserKind {
    let lc = progid.to_ascii_lowercase();
    if lc.starts_with("chromehtml") {
        BrowserKind::Chrome
    } else if lc.starts_with("firefoxurl") {
        BrowserKind::Firefox
    } else if lc.starts_with("msedgehtm") || lc.starts_with("msedgehtml") {
        BrowserKind::Edge
    } else if lc.starts_with("bravehtml") {
        BrowserKind::Brave
    } else if lc.starts_with("chromiumhtm") {
        BrowserKind::Chromium
    } else if lc.starts_with("operastable") {
        BrowserKind::Opera
    } else if lc.contains("vivaldi") {
        BrowserKind::Vivaldi
    } else {
        BrowserKind::Other(progid.to_string())
    }
}

// ─── Platform-gated subprocess layer ─────────────────────────────────────

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    pub fn detect() -> Option<DefaultBrowser> {
        let out = Command::new("xdg-settings")
            .args(["get", "default-web-browser"])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if name.is_empty() {
            return None;
        }
        let kind = parse_xdg_desktop_name(&name);
        let path = which_for_kind(&kind);
        Some(DefaultBrowser { kind, path })
    }

    fn which_for_kind(kind: &BrowserKind) -> Option<PathBuf> {
        let candidates: &[&str] = match kind {
            BrowserKind::Firefox => &["firefox", "firefox-esr"],
            BrowserKind::Chrome => &["google-chrome", "google-chrome-stable"],
            BrowserKind::Chromium => &["chromium", "chromium-browser"],
            BrowserKind::Brave => &["brave-browser", "brave"],
            BrowserKind::Edge => &["microsoft-edge", "microsoft-edge-stable"],
            BrowserKind::Vivaldi => &["vivaldi", "vivaldi-stable"],
            BrowserKind::Opera => &["opera"],
            BrowserKind::Arc => &[],
            BrowserKind::Other(_) => &[],
        };
        for c in candidates {
            if let Some(p) = which(c) {
                return Some(p);
            }
        }
        None
    }

    fn which(bin: &str) -> Option<PathBuf> {
        let path = std::env::var_os("PATH")?;
        for dir in std::env::split_paths(&path) {
            let full = dir.join(bin);
            if full.is_file() {
                return Some(full);
            }
        }
        None
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    pub fn detect() -> Option<DefaultBrowser> {
        let out = Command::new("defaults")
            .args([
                "read",
                "com.apple.LaunchServices/com.apple.launchservices.secure",
            ])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&out.stdout);
        let bundle_id = parse_macos_launchservices_http_handler(&text)?;
        let kind = bundle_id_to_kind(&bundle_id);
        let path = path_for_macos_kind(&kind);
        Some(DefaultBrowser { kind, path })
    }

    fn path_for_macos_kind(kind: &BrowserKind) -> Option<PathBuf> {
        let candidates: &[&str] = match kind {
            BrowserKind::Chrome => &[
                "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            ],
            BrowserKind::Firefox => &[
                "/Applications/Firefox.app/Contents/MacOS/firefox",
            ],
            BrowserKind::Chromium => &[
                "/Applications/Chromium.app/Contents/MacOS/Chromium",
            ],
            BrowserKind::Brave => &[
                "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser",
            ],
            BrowserKind::Edge => &[
                "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
            ],
            BrowserKind::Arc => &["/Applications/Arc.app/Contents/MacOS/Arc"],
            BrowserKind::Vivaldi => &[
                "/Applications/Vivaldi.app/Contents/MacOS/Vivaldi",
            ],
            BrowserKind::Opera => &["/Applications/Opera.app/Contents/MacOS/Opera"],
            BrowserKind::Other(_) => &[],
        };
        for c in candidates {
            let p = PathBuf::from(c);
            if p.exists() {
                return Some(p);
            }
        }
        None
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use super::*;
    pub fn detect() -> Option<DefaultBrowser> {
        // TODO: read HKCU UserChoice via the `winreg` crate when added.
        // For now: PATH-based fallback so atsisbroken on Windows still
        // gets *some* result.
        for (bin, kind) in [
            ("chrome.exe", BrowserKind::Chrome),
            ("msedge.exe", BrowserKind::Edge),
            ("firefox.exe", BrowserKind::Firefox),
            ("brave.exe", BrowserKind::Brave),
        ] {
            if let Some(p) = which(bin) {
                return Some(DefaultBrowser {
                    kind,
                    path: Some(p),
                });
            }
        }
        None
    }

    fn which(bin: &str) -> Option<PathBuf> {
        let path = std::env::var_os("PATH")?;
        for dir in std::env::split_paths(&path) {
            let full = dir.join(bin);
            if full.is_file() {
                return Some(full);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_xdg_desktop_firefox() {
        assert_eq!(parse_xdg_desktop_name("firefox.desktop"), BrowserKind::Firefox);
        assert_eq!(parse_xdg_desktop_name("firefox-esr.desktop"), BrowserKind::Firefox);
    }

    #[test]
    fn parse_xdg_desktop_chrome_chromium_distinct() {
        assert_eq!(
            parse_xdg_desktop_name("google-chrome.desktop"),
            BrowserKind::Chrome
        );
        assert_eq!(parse_xdg_desktop_name("chromium.desktop"), BrowserKind::Chromium);
        assert_eq!(
            parse_xdg_desktop_name("chromium-browser.desktop"),
            BrowserKind::Chromium
        );
    }

    #[test]
    fn parse_xdg_desktop_brave_edge_vivaldi_opera_arc() {
        assert_eq!(parse_xdg_desktop_name("brave-browser.desktop"), BrowserKind::Brave);
        assert_eq!(parse_xdg_desktop_name("microsoft-edge.desktop"), BrowserKind::Edge);
        assert_eq!(parse_xdg_desktop_name("vivaldi-stable.desktop"), BrowserKind::Vivaldi);
        assert_eq!(parse_xdg_desktop_name("opera.desktop"), BrowserKind::Opera);
        assert_eq!(parse_xdg_desktop_name("arc.desktop"), BrowserKind::Arc);
    }

    #[test]
    fn parse_xdg_desktop_unknown_returns_other_with_basename() {
        match parse_xdg_desktop_name("snortwidget.desktop") {
            BrowserKind::Other(s) => assert_eq!(s, "snortwidget"),
            _ => panic!("expected Other variant"),
        }
    }

    #[test]
    fn parse_macos_handler_chrome() {
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
        let h = parse_macos_launchservices_http_handler(plist);
        assert_eq!(h, Some("com.google.chrome".to_string()));
    }

    #[test]
    fn parse_macos_handler_firefox() {
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
        let h = parse_macos_launchservices_http_handler(plist);
        assert_eq!(h, Some("org.mozilla.firefox".to_string()));
    }

    #[test]
    fn parse_macos_handler_returns_none_when_no_http_scheme() {
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
        assert_eq!(parse_macos_launchservices_http_handler(plist), None);
    }

    #[test]
    fn parse_macos_handler_picks_only_http_not_https() {
        // Block 1 is https, block 2 is http — only block 2 should win.
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
        assert_eq!(
            parse_macos_launchservices_http_handler(plist),
            Some("http.handler".to_string())
        );
    }

    #[test]
    fn bundle_id_chrome() {
        assert_eq!(bundle_id_to_kind("com.google.chrome"), BrowserKind::Chrome);
        assert_eq!(
            bundle_id_to_kind("com.google.chrome.canary"),
            BrowserKind::Chrome
        );
    }

    #[test]
    fn bundle_id_firefox_variants() {
        assert_eq!(
            bundle_id_to_kind("org.mozilla.firefox"),
            BrowserKind::Firefox
        );
        assert_eq!(
            bundle_id_to_kind("org.mozilla.firefoxnightly"),
            BrowserKind::Firefox
        );
    }

    #[test]
    fn bundle_id_arc() {
        assert_eq!(
            bundle_id_to_kind("company.thebrowser.browser"),
            BrowserKind::Arc
        );
    }

    #[test]
    fn windows_progid_chrome_firefox_edge() {
        assert_eq!(parse_windows_progid("ChromeHTML"), BrowserKind::Chrome);
        assert_eq!(parse_windows_progid("FirefoxURL-1234"), BrowserKind::Firefox);
        assert_eq!(parse_windows_progid("MSEdgeHTM"), BrowserKind::Edge);
        assert_eq!(parse_windows_progid("BraveHTML"), BrowserKind::Brave);
    }

    #[test]
    fn windows_progid_unknown_returns_other_preserving_case() {
        match parse_windows_progid("WeirdBrowserHTML") {
            BrowserKind::Other(s) => assert_eq!(s, "WeirdBrowserHTML"),
            _ => panic!("expected Other variant"),
        }
    }

    #[test]
    fn supports_cdp_excludes_only_firefox() {
        assert!(BrowserKind::Chrome.supports_cdp());
        assert!(BrowserKind::Chromium.supports_cdp());
        assert!(BrowserKind::Edge.supports_cdp());
        assert!(BrowserKind::Brave.supports_cdp());
        assert!(BrowserKind::Arc.supports_cdp());
        assert!(BrowserKind::Vivaldi.supports_cdp());
        assert!(BrowserKind::Opera.supports_cdp());
        assert!(!BrowserKind::Firefox.supports_cdp());
    }

    /// Real value: this fn shells out to `xdg-settings` (Linux) /
    /// `defaults read` (macOS) / scans PATH (Windows). Each branch
    /// could panic on an unexpected exit code, malformed plist line,
    /// or a missing env var. The test guards every cfg-gated branch
    /// against a regression that introduces an `unwrap` on a
    /// non-Some path. We don't assert the *value* because CI hosts
    /// have no default-browser preference — but the *path coverage*
    /// (executes without panic) is the contract.
    #[test]
    fn default_browser_path_does_not_panic_on_real_host() {
        let _ = default_browser();
    }
}
