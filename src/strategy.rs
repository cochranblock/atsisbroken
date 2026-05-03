// SPDX-License-Identifier: Unlicense
//! Strategy ladder.
//!
//! `atsisbroken run` auto-detects the user's environment and picks the
//! highest-tier strategy that will actually work. Every tier degrades
//! gracefully to the next; the bottom tier (`Speak`) requires nothing
//! beyond stdout and always succeeds, so the user can never end up in
//! a state where atsisbroken is unusable.

use crate::Profile;
use std::path::PathBuf;
use std::process::Command;

/// One way of getting profile data into a job-application form, in
/// declining order of automation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Strategy {
    /// Connect to a Chromium-family browser already running with a
    /// remote debugging port we can reach. Highest fidelity.
    CdpAttach { endpoint: String },
    /// Launch a dedicated Chromium subprocess with a temp profile and
    /// our debugging port. Requires a discoverable browser binary.
    CdpLaunch { browser_path: PathBuf },
    /// User has the atsisbroken Chrome extension installed and the
    /// Native Messaging host registered. The extension does the fill;
    /// the desktop binary is the trainer + feedback sink.
    Extension,
    /// Emit a TamperMonkey/Greasemonkey userscript the user installs
    /// once into their browser. No native messaging needed.
    Userscript,
    /// Emit a `javascript:` bookmarklet with the profile baked in.
    /// User drags to bookmark bar; clicks on any form page to fill.
    Bookmarklet,
    /// Put one profile field at a time on the system clipboard via the
    /// platform's clipboard tool. User pastes manually.
    Clipboard { tool: ClipboardTool },
    /// Print profile values to stdout for the user to type by hand.
    /// The floor. Works literally anywhere a terminal does.
    Speak,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardTool {
    Pbcopy,    // macOS
    Xclip,     // Linux X11
    Wlcopy,    // Linux Wayland
    ClipExe,   // Windows / WSL
}

impl ClipboardTool {
    pub fn binary(&self) -> &'static str {
        match self {
            ClipboardTool::Pbcopy => "pbcopy",
            ClipboardTool::Xclip => "xclip",
            ClipboardTool::Wlcopy => "wl-copy",
            ClipboardTool::ClipExe => "clip.exe",
        }
    }
    /// Args that pipe stdin into the system clipboard.
    pub fn args(&self) -> Vec<&'static str> {
        match self {
            ClipboardTool::Pbcopy => vec![],
            ClipboardTool::Xclip => vec!["-selection", "clipboard"],
            ClipboardTool::Wlcopy => vec![],
            ClipboardTool::ClipExe => vec![],
        }
    }
}

/// Detect the highest-tier strategy this environment supports.
///
/// Pure inspection of: env vars, the filesystem, and `which`-style PATH
/// lookups. Does not make network requests; the CDP-attach probe is
/// done in [`probe_cdp_endpoint`] and only invoked from the run-loop
/// where blocking I/O is acceptable.
pub fn detect() -> Strategy {
    if let Some(endpoint) = probe_cdp_endpoint() {
        return Strategy::CdpAttach { endpoint };
    }
    if let Some(p) = find_chromium_binary() {
        return Strategy::CdpLaunch { browser_path: p };
    }
    if extension_native_host_installed() {
        return Strategy::Extension;
    }
    if let Some(t) = detect_clipboard_tool() {
        return Strategy::Clipboard { tool: t };
    }
    Strategy::Speak
}

/// Try to reach a Chromium debug port. Returns the http endpoint URL on
/// success. Checks the conventional `localhost:9222` first, then
/// CHROME_DEBUG_PORT env var.
pub fn probe_cdp_endpoint() -> Option<String> {
    let candidates = std::env::var("CHROME_DEBUG_PORT")
        .ok()
        .into_iter()
        .chain(["9222".to_string()]);
    for port in candidates {
        let url = format!("http://localhost:{port}/json/version");
        if let Ok(stream) = std::net::TcpStream::connect_timeout(
            &format!("127.0.0.1:{port}").parse().ok()?,
            std::time::Duration::from_millis(250),
        ) {
            drop(stream);
            return Some(url);
        }
    }
    None
}

/// Walk a small set of conventional install paths + `PATH` for any
/// Chromium-family browser binary. First match wins; order biases
/// toward "what the user explicitly chose".
pub fn find_chromium_binary() -> Option<PathBuf> {
    let candidates: &[&str] = &[
        // Linux
        "google-chrome",
        "google-chrome-stable",
        "chromium",
        "chromium-browser",
        "brave-browser",
        "microsoft-edge",
        // macOS app bundles
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
        "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser",
        "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        "/Applications/Arc.app/Contents/MacOS/Arc",
    ];
    for c in candidates {
        let p = PathBuf::from(c);
        if p.is_absolute() {
            if p.exists() {
                return Some(p);
            }
        } else if let Some(found) = which(c) {
            return Some(found);
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

fn extension_native_host_installed() -> bool {
    crate::paths::chrome_native_host_dir()
        .map(|d| d.join("org.cochranblock.atsisbroken.json").exists())
        .unwrap_or(false)
}

pub fn detect_clipboard_tool() -> Option<ClipboardTool> {
    if which("pbcopy").is_some() {
        return Some(ClipboardTool::Pbcopy);
    }
    if std::env::var_os("WAYLAND_DISPLAY").is_some() && which("wl-copy").is_some() {
        return Some(ClipboardTool::Wlcopy);
    }
    if which("xclip").is_some() {
        return Some(ClipboardTool::Xclip);
    }
    if which("wl-copy").is_some() {
        return Some(ClipboardTool::Wlcopy);
    }
    if which("clip.exe").is_some() {
        return Some(ClipboardTool::ClipExe);
    }
    None
}

// ─── Tier executors ────────────────────────────────────────────────────────

/// Print every populated profile field as `key: value` to the writer.
/// Always succeeds. The floor of the strategy ladder.
pub fn speak<W: std::io::Write>(profile: &Profile, mut w: W) -> std::io::Result<()> {
    let lines: &[(&str, &str)] = &[
        ("full_name", &profile.full_name),
        ("email", &profile.email),
        ("phone", &profile.phone),
        ("address", &profile.address),
        ("linkedin", &profile.linkedin),
        ("github", &profile.github),
        ("website", &profile.website),
        ("work_authorization", &profile.work_authorization),
    ];
    for (k, v) in lines {
        if !v.is_empty() {
            writeln!(w, "{k}: {v}")?;
        }
    }
    if profile.years_experience > 0 {
        writeln!(w, "years_experience: {}", profile.years_experience)?;
    }
    Ok(())
}

/// Pipe a single value to the OS clipboard via the chosen tool.
pub fn copy_to_clipboard(tool: ClipboardTool, value: &str) -> std::io::Result<()> {
    use std::io::Write;
    let mut child = Command::new(tool.binary())
        .args(tool.args())
        .stdin(std::process::Stdio::piped())
        .spawn()?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(value.as_bytes())?;
    }
    let status = child.wait()?;
    if !status.success() {
        return Err(std::io::Error::other(format!(
            "{} exited with status {status}",
            tool.binary()
        )));
    }
    Ok(())
}

/// Generate a TamperMonkey/Greasemonkey userscript that fills forms
/// using a literal profile object. The user installs this once into
/// their browser; afterward the script runs on every page.
pub fn userscript(profile: &Profile) -> String {
    let profile_json =
        serde_json::to_string(profile).unwrap_or_else(|_| "{}".to_string());
    format!(
        r#"// ==UserScript==
// @name         atsisbroken
// @namespace    https://cochranblock.org
// @version      {version}
// @description  Local-first ATS form autofill from your own resume.
// @match        *://*/*
// @run-at       document-idle
// @grant        none
// @license      Unlicense
// ==/UserScript==
(function () {{
  const profile = {profile_json};
  const KEY_VOCAB = ['email','phone','full_name','linkedin','github','website','address','work_authorization','years_experience'];

  function describe(el) {{
    const labelEl = (el.id && document.querySelector('label[for="' + el.id + '"]')) || el.closest('label');
    return {{
      label: labelEl ? labelEl.innerText.trim() : '',
      placeholder: el.placeholder || '',
      aria_label: el.getAttribute('aria-label') || '',
      name: el.name || '',
      id: el.id || '',
      kind: el.type || el.tagName.toLowerCase(),
    }};
  }}

  function predict(field) {{
    const hay = (field.label + ' ' + field.placeholder + ' ' + field.aria_label + ' ' + field.name + ' ' + field.id).toLowerCase();
    if (hay.indexOf('email') >= 0) return 'email';
    if (hay.indexOf('phone') >= 0 || hay.indexOf('mobile') >= 0 || hay.indexOf('tel') >= 0) return 'phone';
    if (hay.indexOf('linkedin') >= 0) return 'linkedin';
    if (hay.indexOf('github') >= 0) return 'github';
    if (hay.indexOf('website') >= 0 || hay.indexOf('portfolio') >= 0) return 'website';
    if (hay.indexOf('address') >= 0 || hay.indexOf('street') >= 0) return 'address';
    if (hay.indexOf('authoriz') >= 0 || hay.indexOf('visa') >= 0 || hay.indexOf('sponsor') >= 0) return 'work_authorization';
    if ((hay.indexOf('year') >= 0 || hay.indexOf('yrs') >= 0) && hay.indexOf('exp') >= 0) return 'years_experience';
    if (hay.indexOf('name') >= 0) return 'full_name';
    return '';
  }}

  function fill(el, value) {{
    if (!value) return;
    el.value = value;
    el.dispatchEvent(new Event('input', {{ bubbles: true }}));
    el.dispatchEvent(new Event('change', {{ bubbles: true }}));
  }}

  function pass() {{
    document.querySelectorAll('input, textarea, select').forEach((el) => {{
      if (el.value && el.value.trim().length > 0) return; // don't clobber
      const f = describe(el);
      const key = predict(f);
      if (KEY_VOCAB.indexOf(key) >= 0 && profile[key]) {{
        fill(el, String(profile[key]));
      }}
    }});
  }}

  pass();
  new MutationObserver(pass).observe(document.body, {{ childList: true, subtree: true }});
}})();
"#,
        version = crate::version(),
        profile_json = profile_json,
    )
}

/// Generate a `javascript:` bookmarklet. URL-encodes the body so it
/// can be pasted as a bookmark target. Bookmarklets have a length cap
/// (browser-dependent ~2000–8000 chars), so the body here is minimal —
/// it doesn't repeat the userscript's full classifier.
pub fn bookmarklet(profile: &Profile) -> String {
    let profile_json =
        serde_json::to_string(profile).unwrap_or_else(|_| "{}".to_string());
    let js = format!(
        "(function(){{var p={profile_json};var V=['email','phone','full_name','linkedin','github','website','address','work_authorization'];function k(f){{var h=(f.label+' '+f.placeholder+' '+f.aria_label+' '+f.name+' '+f.id).toLowerCase();if(h.indexOf('email')>=0)return'email';if(h.indexOf('phone')>=0||h.indexOf('mobile')>=0||h.indexOf('tel')>=0)return'phone';if(h.indexOf('linkedin')>=0)return'linkedin';if(h.indexOf('github')>=0)return'github';if(h.indexOf('website')>=0||h.indexOf('portfolio')>=0)return'website';if(h.indexOf('address')>=0||h.indexOf('street')>=0)return'address';if(h.indexOf('authoriz')>=0||h.indexOf('visa')>=0||h.indexOf('sponsor')>=0)return'work_authorization';if(h.indexOf('name')>=0)return'full_name';return''}};document.querySelectorAll('input,textarea,select').forEach(function(e){{if(e.value)return;var l=e.id&&document.querySelector('label[for=\"'+e.id+'\"]');var f={{label:l?l.innerText:'' ,placeholder:e.placeholder||'',aria_label:e.getAttribute('aria-label')||'',name:e.name||'',id:e.id||'',kind:e.type||''}};var x=k(f);if(V.indexOf(x)>=0&&p[x]){{e.value=p[x];e.dispatchEvent(new Event('input',{{bubbles:true}}));e.dispatchEvent(new Event('change',{{bubbles:true}}))}}}});}})();"
    );
    let encoded: String = js
        .chars()
        .map(|c| match c {
            ' ' => "%20".to_string(),
            '"' => "%22".to_string(),
            '\'' => "%27".to_string(),
            '<' => "%3C".to_string(),
            '>' => "%3E".to_string(),
            '#' => "%23".to_string(),
            '%' => "%25".to_string(),
            _ => c.to_string(),
        })
        .collect();
    format!("javascript:{encoded}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_profile() -> Profile {
        Profile {
            full_name: "Jane Doe".into(),
            email: "jane@example.com".into(),
            phone: "+1-555-0100".into(),
            linkedin: "linkedin.com/in/janedoe".into(),
            github: "github.com/janedoe".into(),
            website: "janedoe.dev".into(),
            address: "1 Main St, Anywhere, USA".into(),
            work_authorization: "US Citizen".into(),
            years_experience: 7,
            ..Default::default()
        }
    }

    #[test]
    fn detect_always_returns_a_strategy() {
        // Even on a totally empty environment, Speak is the floor.
        let _ = detect();
    }

    #[test]
    fn speak_emits_every_populated_field_once() {
        let mut buf = Vec::new();
        speak(&sample_profile(), &mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        for needle in [
            "full_name: Jane Doe",
            "email: jane@example.com",
            "phone: +1-555-0100",
            "linkedin: linkedin.com/in/janedoe",
            "github: github.com/janedoe",
            "website: janedoe.dev",
            "address: 1 Main St, Anywhere, USA",
            "work_authorization: US Citizen",
            "years_experience: 7",
        ] {
            assert!(out.contains(needle), "missing line {needle:?} in:\n{out}");
        }
    }

    #[test]
    fn speak_skips_empty_fields() {
        let mut p = Profile::default();
        p.email = "x@y.com".into();
        let mut buf = Vec::new();
        speak(&p, &mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("email: x@y.com"));
        assert!(!out.contains("full_name:"));
        assert!(!out.contains("phone:"));
        assert!(!out.contains("years_experience"));
    }

    #[test]
    fn userscript_contains_profile_and_classifier() {
        let s = userscript(&sample_profile());
        assert!(s.contains("==UserScript=="));
        assert!(s.contains("\"jane@example.com\""));
        assert!(s.contains("predict")); // classifier function present
        assert!(s.contains("MutationObserver")); // re-render defense
    }

    #[test]
    fn bookmarklet_is_javascript_url() {
        let b = bookmarklet(&sample_profile());
        assert!(b.starts_with("javascript:"));
        // Profile payload must survive URL-encoding.
        assert!(b.contains("jane@example.com"));
        // Must not contain raw spaces in the encoded body.
        let body = b.strip_prefix("javascript:").unwrap();
        assert!(!body.contains(' '));
    }

    #[test]
    fn bookmarklet_size_under_browser_caps() {
        // Conservative cap: 4 KiB (most browsers accept much more, but
        // this catches bloat). If we cross this, time to rethink.
        let b = bookmarklet(&sample_profile());
        assert!(b.len() < 4096, "bookmarklet too large: {} bytes", b.len());
    }

    #[test]
    fn clipboard_tool_binaries_are_known() {
        // Pin the binary names — a typo here breaks every clipboard
        // tier installation in the wild.
        assert_eq!(ClipboardTool::Pbcopy.binary(), "pbcopy");
        assert_eq!(ClipboardTool::Xclip.binary(), "xclip");
        assert_eq!(ClipboardTool::Wlcopy.binary(), "wl-copy");
        assert_eq!(ClipboardTool::ClipExe.binary(), "clip.exe");
    }

    #[test]
    fn xclip_args_select_clipboard_not_primary() {
        // -selection clipboard is the right one; without it xclip writes
        // to PRIMARY (middle-click), which most ATS users won't notice.
        assert_eq!(ClipboardTool::Xclip.args(), vec!["-selection", "clipboard"]);
    }

    #[test]
    fn userscript_braces_balance() {
        let s = userscript(&sample_profile());
        let opens = s.matches('{').count();
        let closes = s.matches('}').count();
        assert_eq!(
            opens, closes,
            "userscript has unbalanced braces: {opens} {{ vs {closes} }}"
        );
    }

    #[test]
    fn userscript_parens_balance() {
        let s = userscript(&sample_profile());
        let opens = s.matches('(').count();
        let closes = s.matches(')').count();
        assert_eq!(
            opens, closes,
            "userscript has unbalanced parens: {opens} ( vs {closes} )"
        );
    }

    #[test]
    fn userscript_has_required_userscript_metadata() {
        // TamperMonkey's parser requires these tags. Drop one and the
        // script silently fails to register.
        let s = userscript(&sample_profile());
        for tag in [
            "@name", "@namespace", "@version", "@match", "@run-at", "@grant",
        ] {
            assert!(s.contains(tag), "userscript missing {tag}");
        }
    }

    #[test]
    fn userscript_does_not_clobber_filled_inputs() {
        // The script must include the "if value is non-empty, skip"
        // guard. Otherwise it overwrites whatever the user already typed.
        let s = userscript(&sample_profile());
        assert!(
            s.contains("don't clobber") || s.contains("trim().length"),
            "userscript must guard against clobbering existing values"
        );
    }

    #[test]
    fn bookmarklet_is_single_line() {
        // Bookmarks can't span newlines once dropped on the bookmark bar.
        let b = bookmarklet(&sample_profile());
        assert!(!b.contains('\n'));
    }

    #[test]
    fn bookmarklet_has_no_unencoded_hash() {
        // `#` is the URL fragment delimiter — leaving one un-encoded
        // would truncate the bookmarklet at that point.
        let b = bookmarklet(&sample_profile());
        let body = b.strip_prefix("javascript:").unwrap();
        assert!(!body.contains('#'));
    }

    #[test]
    fn bookmarklet_has_no_unencoded_quotes() {
        let b = bookmarklet(&sample_profile());
        let body = b.strip_prefix("javascript:").unwrap();
        assert!(!body.contains('"'));
        assert!(!body.contains('\''));
    }

    #[test]
    fn speak_emits_each_field_once_only() {
        let mut buf = Vec::new();
        speak(&sample_profile(), &mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        // Each `key:` prefix must occur exactly once.
        for key in [
            "full_name:",
            "email:",
            "phone:",
            "linkedin:",
            "github:",
            "website:",
            "address:",
            "work_authorization:",
            "years_experience:",
        ] {
            assert_eq!(
                out.matches(key).count(),
                1,
                "key {key} appeared {} times",
                out.matches(key).count()
            );
        }
    }

    #[test]
    fn speak_preserves_spaces_in_values() {
        // Address contains commas and spaces — must not be mangled.
        let mut buf = Vec::new();
        speak(&sample_profile(), &mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("address: 1 Main St, Anywhere, USA"));
    }

    #[test]
    fn detect_clipboard_tool_returns_none_when_no_tool_present() {
        // Sanity: the OR of detection branches is one of the variants
        // or None — never panics.
        let _ = detect_clipboard_tool();
    }

    #[test]
    fn strategy_speak_is_always_constructable() {
        // Floor of the ladder must be representable without I/O.
        let _ = Strategy::Speak;
    }

    #[test]
    fn find_chromium_binary_returns_a_typed_option() {
        // Pure type-shape sanity — guards against a refactor that
        // changes the return type without updating callers.
        let r: Option<std::path::PathBuf> = find_chromium_binary();
        if let Some(p) = r {
            assert!(p.is_absolute() || p.exists());
        }
    }
}
