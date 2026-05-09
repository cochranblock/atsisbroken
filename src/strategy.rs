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

impl Strategy {
    /// Resolve a CLI `--strategy <s>` override into a concrete
    /// [`Strategy`]. Pure logic for the flag-match; the tier-
    /// detection branches (CDP probe, chromium-binary lookup,
    /// clipboard-tool detection) are environment-dependent and
    /// return Err when the requested strategy isn't reachable.
    ///
    /// Error type is `String` rather than `anyhow::Error` so this
    /// function stays in the lib (anyhow is a binary-side dep
    /// only). main.rs's CLI parser converts to anyhow::Error at
    /// the call site.
    pub fn from_cli_str(s: &str) -> Result<Strategy, String> {
        match s {
            "cdp-attach" => probe_cdp_endpoint()
                .map(|endpoint| Strategy::CdpAttach { endpoint })
                .ok_or_else(|| "no Chromium debug port reachable".to_string()),
            "cdp-launch" => find_chromium_binary()
                .map(|browser_path| Strategy::CdpLaunch { browser_path })
                .ok_or_else(|| "no Chromium-family browser found in PATH".to_string()),
            "extension" => Ok(Strategy::Extension),
            "userscript" => Ok(Strategy::Userscript),
            "bookmarklet" => Ok(Strategy::Bookmarklet),
            "clipboard" => detect_clipboard_tool()
                .map(|tool| Strategy::Clipboard { tool })
                .ok_or_else(|| {
                    "no clipboard tool found (pbcopy/xclip/wl-copy/clip.exe)".to_string()
                }),
            "speak" => Ok(Strategy::Speak),
            other => Err(format!("unknown strategy: {other}")),
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

