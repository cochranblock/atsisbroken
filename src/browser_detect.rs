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

