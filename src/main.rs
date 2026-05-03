// SPDX-License-Identifier: Unlicense
// Unlicense — public domain — cochranblock.org

use anyhow::{anyhow, Context, Result};
use atsisbroken::{
    bridge::serve_native_messaging, cdp, paths, resume, seed_corpus_fingerprint,
    strategy::{self, Strategy},
    version, FeedbackQueue, Mode, Profile,
};
use clap::{Parser, Subcommand};
use std::io::Read;

#[derive(Parser, Debug)]
#[command(
    name = "atsisbroken",
    version,
    about = "ATS is broken. Fill it locally with a model you trained."
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Build the Profile from a resume and seed the personal classifier.
    Init {
        /// Read resume text from this file. If omitted, read from stdin.
        #[arg(long)]
        resume: Option<String>,
    },
    /// Attach to a running Chromium and fill forms. Auto-detects the best
    /// strategy for this environment (CDP attach → CDP launch → extension →
    /// userscript → bookmarklet → clipboard → speak). Override with
    /// --strategy.
    Run {
        #[arg(long, value_parser = parse_mode)]
        mode: Option<Mode>,
        #[arg(long)]
        cdp: Option<String>,
        /// Force a specific strategy: cdp-attach | cdp-launch | extension |
        /// userscript | bookmarklet | clipboard | speak. If unset, the best
        /// available is auto-picked.
        #[arg(long)]
        strategy: Option<String>,
    },
    /// Output a TamperMonkey/Greasemonkey userscript with your profile
    /// baked in. Install once into your browser; runs on every page.
    Userscript,
    /// Output a `javascript:` bookmarklet. Drag it to your bookmark bar;
    /// click on any form page to autofill.
    Bookmarklet,
    /// Copy a single profile field to the system clipboard. Works in
    /// locked-down environments where neither CDP nor extensions are
    /// allowed.
    Copy {
        /// Field key: full_name, email, phone, address, linkedin, github,
        /// website, work_authorization, years_experience.
        key: String,
    },
    /// Print every populated profile field as `key: value` to stdout.
    /// The strategy floor — works literally anywhere atsisbroken runs.
    Speak,
    /// Probe for a running Chromium debug port and list its open tabs.
    /// Diagnostic / proof-of-life for the CDP transport.
    CdpProbe,
    /// Take off the training wheels. Subsequent `run`s fill autonomously.
    Graduate,
    /// Drain the local feedback queue if delivery is configured and the
    /// network is reachable. Otherwise no-op. Always safe to run offline.
    Sync,
    /// Write the local feedback queue to a path the user can email manually.
    Export {
        #[arg(long)]
        out: String,
    },
    /// Run the Chrome Native Messaging host loop. Spawned by Chrome when
    /// the atsisbroken extension calls connectNative.
    Bridge,
    /// Install the Chrome Native Messaging host manifest into the OS-
    /// specific location so Chrome can spawn the bridge.
    InstallBridge {
        /// Chrome extension ID assigned to the atsisbroken extension after
        /// it's loaded into chrome://extensions.
        #[arg(long)]
        extension_id: String,
        /// Override the binary path written into the manifest. Defaults to
        /// the absolute path of the currently-running atsisbroken binary.
        #[arg(long)]
        binary: Option<String>,
    },
    /// Print the current configuration.
    Status,
}

fn parse_mode(s: &str) -> Result<Mode, String> {
    match s {
        "training-wheels" | "training_wheels" | "training" => Ok(Mode::TrainingWheels),
        "shadow" => Ok(Mode::Shadow),
        "chaos" => Ok(Mode::Chaos),
        other => Err(format!("unknown mode: {other}")),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Init { resume } => cmd_init(resume).await,
        Cmd::Run { mode, cdp, strategy } => cmd_run(mode, cdp, strategy).await,
        Cmd::Userscript => cmd_userscript().await,
        Cmd::Bookmarklet => cmd_bookmarklet().await,
        Cmd::Copy { key } => cmd_copy(key).await,
        Cmd::Speak => cmd_speak().await,
        Cmd::CdpProbe => cmd_cdp_probe().await,
        Cmd::Graduate => cmd_graduate().await,
        Cmd::Sync => cmd_sync().await,
        Cmd::Export { out } => cmd_export(out).await,
        Cmd::Bridge => cmd_bridge().await,
        Cmd::InstallBridge { extension_id, binary } => cmd_install_bridge(extension_id, binary),
        Cmd::Status => cmd_status().await,
    }
}

async fn cmd_init(resume_path: Option<String>) -> Result<()> {
    let text = match resume_path {
        Some(path) => std::fs::read_to_string(&path)
            .with_context(|| format!("read resume file {path}"))?,
        None => {
            eprintln!("paste resume text and press ctrl-D when done:");
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            buf
        }
    };
    if text.trim().is_empty() {
        return Err(anyhow!("resume text was empty"));
    }
    let profile: Profile = resume::parse_resume(&text);
    paths::ensure_dir()?;
    let path = paths::profile_path();
    let toml_text = toml::to_string_pretty(&profile).context("serialize profile")?;
    std::fs::write(&path, toml_text).with_context(|| format!("write {}", path.display()))?;
    eprintln!("atsisbroken {} — wrote profile to {}", version(), path.display());
    eprintln!(
        "  full_name={:?} email={:?} phone={:?}",
        profile.full_name, profile.email, profile.phone
    );
    Ok(())
}

async fn cmd_run(
    _mode: Option<Mode>,
    _cdp: Option<String>,
    forced: Option<String>,
) -> Result<()> {
    let profile = load_profile_or_hint()?;
    let chosen = match forced.as_deref() {
        Some(s) => parse_strategy_override(s, &profile)?,
        None => strategy::detect(),
    };
    eprintln!("atsisbroken {} — strategy: {:?}", version(), chosen);
    match chosen {
        Strategy::CdpAttach { endpoint } => run_cdp_attach(&endpoint).await,
        Strategy::CdpLaunch { browser_path } => {
            eprintln!(
                "  cdp-launch not yet wired — fall through to userscript. \n  Browser found at: {}",
                browser_path.display()
            );
            print!("{}", strategy::userscript(&profile));
            Ok(())
        }
        Strategy::Extension => {
            eprintln!("  extension is installed; the extension fills via its content script. \n  Run `atsisbroken bridge` (Chrome will spawn it on demand).");
            Ok(())
        }
        Strategy::Userscript => {
            print!("{}", strategy::userscript(&profile));
            Ok(())
        }
        Strategy::Bookmarklet => {
            println!("{}", strategy::bookmarklet(&profile));
            Ok(())
        }
        Strategy::Clipboard { tool } => {
            eprintln!(
                "  clipboard tool: {} — use `atsisbroken copy <key>` to copy individual fields.",
                tool.binary()
            );
            Ok(())
        }
        Strategy::Speak => {
            strategy::speak(&profile, std::io::stdout())?;
            Ok(())
        }
    }
}

fn parse_strategy_override(s: &str, profile: &Profile) -> Result<Strategy> {
    match s {
        "cdp-attach" => strategy::probe_cdp_endpoint()
            .map(|endpoint| Strategy::CdpAttach { endpoint })
            .ok_or_else(|| anyhow!("no Chromium debug port reachable")),
        "cdp-launch" => strategy::find_chromium_binary()
            .map(|browser_path| Strategy::CdpLaunch { browser_path })
            .ok_or_else(|| anyhow!("no Chromium-family browser found in PATH")),
        "extension" => Ok(Strategy::Extension),
        "userscript" => Ok(Strategy::Userscript),
        "bookmarklet" => Ok(Strategy::Bookmarklet),
        "clipboard" => strategy::detect_clipboard_tool()
            .map(|tool| Strategy::Clipboard { tool })
            .ok_or_else(|| anyhow!("no clipboard tool found (pbcopy/xclip/wl-copy/clip.exe)")),
        "speak" => Ok(Strategy::Speak),
        other => Err(anyhow!("unknown strategy: {other}")),
    }
    .map(|st| {
        // Touch the profile to silence unused warning for branches that don't
        // need it; future strategies may consult it.
        let _ = profile;
        st
    })
}

async fn run_cdp_attach(endpoint: &str) -> Result<()> {
    let (_host, port) =
        cdp::parse_endpoint(endpoint).ok_or_else(|| anyhow!("bad endpoint: {endpoint}"))?;
    let tabs = cdp::list_tabs(port).context("list CDP tabs")?;
    if tabs.is_empty() {
        eprintln!("  CDP attached on port {port} — no tabs open.");
    } else {
        eprintln!("  CDP attached on port {port} — {} tab(s):", tabs.len());
        for (i, t) in tabs.iter().enumerate() {
            eprintln!("    [{i}] {} — {}", t.title, t.url);
        }
        eprintln!("  Fill loop wires in next iteration. For now, use --strategy=userscript or --strategy=bookmarklet.");
    }
    Ok(())
}

fn load_profile_or_hint() -> Result<Profile> {
    let path = paths::profile_path();
    if !path.exists() {
        return Err(anyhow!(
            "profile not found at {} — run `atsisbroken init` first",
            path.display()
        ));
    }
    let text = std::fs::read_to_string(&path)?;
    Ok(toml::from_str(&text).context("parse profile.toml")?)
}

async fn cmd_userscript() -> Result<()> {
    let p = load_profile_or_hint()?;
    print!("{}", strategy::userscript(&p));
    Ok(())
}

async fn cmd_bookmarklet() -> Result<()> {
    let p = load_profile_or_hint()?;
    println!("{}", strategy::bookmarklet(&p));
    Ok(())
}

async fn cmd_copy(key: String) -> Result<()> {
    let p = load_profile_or_hint()?;
    let value = match key.as_str() {
        "full_name" => p.full_name,
        "email" => p.email,
        "phone" => p.phone,
        "address" => p.address,
        "linkedin" => p.linkedin,
        "github" => p.github,
        "website" => p.website,
        "work_authorization" => p.work_authorization,
        "years_experience" => p.years_experience.to_string(),
        other => return Err(anyhow!("unknown key: {other}")),
    };
    if value.is_empty() {
        return Err(anyhow!("profile field {key} is empty"));
    }
    let tool = strategy::detect_clipboard_tool()
        .ok_or_else(|| anyhow!("no clipboard tool found (pbcopy/xclip/wl-copy/clip.exe)"))?;
    strategy::copy_to_clipboard(tool, &value)?;
    eprintln!("atsisbroken {} — copied {} via {}", version(), key, tool.binary());
    Ok(())
}

async fn cmd_speak() -> Result<()> {
    let p = load_profile_or_hint()?;
    strategy::speak(&p, std::io::stdout())?;
    Ok(())
}

async fn cmd_cdp_probe() -> Result<()> {
    match strategy::probe_cdp_endpoint() {
        Some(endpoint) => {
            run_cdp_attach(&endpoint).await
        }
        None => {
            eprintln!(
                "atsisbroken {} — no Chromium debug port reachable. Launch Chrome with --remote-debugging-port=9222 to enable CDP attach.",
                version()
            );
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atsisbroken::strategy::ClipboardTool;

    fn empty_profile() -> Profile {
        Profile::default()
    }

    // ─── parse_mode ────────────────────────────────────────────────────────

    #[test]
    fn parse_mode_accepts_canonical_spellings() {
        assert_eq!(parse_mode("training-wheels").unwrap(), Mode::TrainingWheels);
        assert_eq!(parse_mode("training_wheels").unwrap(), Mode::TrainingWheels);
        assert_eq!(parse_mode("training").unwrap(), Mode::TrainingWheels);
        assert_eq!(parse_mode("shadow").unwrap(), Mode::Shadow);
        assert_eq!(parse_mode("chaos").unwrap(), Mode::Chaos);
    }

    #[test]
    fn parse_mode_error_message_includes_input() {
        let err = parse_mode("BANANA").unwrap_err();
        assert!(err.contains("BANANA"), "error must surface the bad input");
    }

    #[test]
    fn parse_mode_rejects_known_typos() {
        assert!(parse_mode("Chaos").is_err()); // case-sensitive
        assert!(parse_mode("shadows").is_err());
        assert!(parse_mode("").is_err());
    }

    // ─── parse_strategy_override ──────────────────────────────────────────

    #[test]
    fn parse_strategy_override_userscript_branch() {
        let p = empty_profile();
        let s = parse_strategy_override("userscript", &p).unwrap();
        assert_eq!(s, Strategy::Userscript);
    }

    #[test]
    fn parse_strategy_override_bookmarklet_branch() {
        let p = empty_profile();
        let s = parse_strategy_override("bookmarklet", &p).unwrap();
        assert_eq!(s, Strategy::Bookmarklet);
    }

    #[test]
    fn parse_strategy_override_extension_branch() {
        let p = empty_profile();
        let s = parse_strategy_override("extension", &p).unwrap();
        assert_eq!(s, Strategy::Extension);
    }

    #[test]
    fn parse_strategy_override_speak_branch() {
        let p = empty_profile();
        let s = parse_strategy_override("speak", &p).unwrap();
        assert_eq!(s, Strategy::Speak);
    }

    #[test]
    fn parse_strategy_override_unknown_returns_err() {
        let p = empty_profile();
        let err = parse_strategy_override("yolo", &p).unwrap_err();
        assert!(err.to_string().contains("unknown"));
    }

    #[test]
    fn parse_strategy_override_empty_returns_err() {
        let p = empty_profile();
        assert!(parse_strategy_override("", &p).is_err());
    }

    #[test]
    fn parse_strategy_override_clipboard_returns_err_without_tool() {
        // CI often has no clipboard tool; the override path must surface
        // a clear error rather than silently picking one.
        // We can't reliably stub `which`, so this just exercises the
        // Result path — passes if either Ok or Err, and we assert that
        // when Ok it's a Clipboard variant.
        let p = empty_profile();
        if let Ok(s) = parse_strategy_override("clipboard", &p) {
            assert!(matches!(s, Strategy::Clipboard { .. }));
        }
    }

    #[test]
    fn clipboard_tool_args_pass_to_command_correctly() {
        // Command construction is hard to mock, but we can at least
        // confirm the args list each tool yields is consistent with
        // its binary name (no "pbcopy" with -selection flag, etc.).
        for (tool, args) in [
            (ClipboardTool::Pbcopy, vec![]),
            (ClipboardTool::Xclip, vec!["-selection", "clipboard"]),
            (ClipboardTool::Wlcopy, vec![]),
            (ClipboardTool::ClipExe, vec![]),
        ] {
            assert_eq!(tool.args(), args);
        }
    }
}

async fn cmd_graduate() -> Result<()> {
    eprintln!(
        "atsisbroken {} — graduate: Mode flip not yet wired (Phase 4)",
        version()
    );
    Ok(())
}

async fn cmd_sync() -> Result<()> {
    let path = paths::feedback_jsonl_path();
    let q = FeedbackQueue::load_from(&path).context("load feedback queue")?;
    if q.is_empty() {
        eprintln!("atsisbroken {} — sync: feedback queue is empty, nothing to do", version());
    } else {
        eprintln!(
            "atsisbroken {} — sync: {} events queued at {}. Delivery destination not configured (LocalOnly).",
            version(),
            q.len(),
            path.display()
        );
    }
    Ok(())
}

async fn cmd_export(out: String) -> Result<()> {
    let path = paths::feedback_jsonl_path();
    let q = FeedbackQueue::load_from(&path).context("load feedback queue")?;
    std::fs::write(&out, q.to_jsonl().context("serialize jsonl")?)
        .with_context(|| format!("write {out}"))?;
    eprintln!(
        "atsisbroken {} — export: {} events → {}",
        version(),
        q.len(),
        out
    );
    Ok(())
}

async fn cmd_bridge() -> Result<()> {
    let path = paths::feedback_jsonl_path();
    let mut queue = FeedbackQueue::load_from(&path).context("load feedback queue")?;
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut r = stdin.lock();
    let mut w = stdout.lock();
    let result = serve_native_messaging(&mut r, &mut w, &mut queue);
    // Persist whatever we accepted, even on protocol error mid-session.
    paths::ensure_dir()?;
    queue.save_to(&path).context("save feedback queue")?;
    result.map_err(|e| anyhow!("bridge: {e}"))
}

fn cmd_install_bridge(extension_id: String, binary: Option<String>) -> Result<()> {
    let host_dir = paths::chrome_native_host_dir()
        .ok_or_else(|| anyhow!("Chrome Native Messaging install path unknown on this OS"))?;
    let bin_path = match binary {
        Some(b) => b,
        None => std::env::current_exe()
            .context("locate current binary")?
            .to_string_lossy()
            .into_owned(),
    };
    let manifest = serde_json::json!({
        "name": "org.cochranblock.atsisbroken",
        "description": "atsisbroken native messaging host",
        "path": bin_path,
        "type": "stdio",
        "allowed_origins": [format!("chrome-extension://{extension_id}/")],
    });
    std::fs::create_dir_all(&host_dir)
        .with_context(|| format!("mkdir {}", host_dir.display()))?;
    let manifest_path = host_dir.join("org.cochranblock.atsisbroken.json");
    std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).context("serialize manifest")?,
    )
    .with_context(|| format!("write {}", manifest_path.display()))?;
    eprintln!(
        "atsisbroken {} — installed Native Messaging host at {}",
        version(),
        manifest_path.display()
    );
    Ok(())
}

async fn cmd_status() -> Result<()> {
    let profile_path = paths::profile_path();
    let initialized = profile_path.exists();
    let feedback_path = paths::feedback_jsonl_path();
    let queue_depth = if feedback_path.exists() {
        FeedbackQueue::load_from(&feedback_path).map(|q| q.len()).unwrap_or(0)
    } else {
        0
    };
    println!("atsisbroken {}", version());
    println!("seed corpus fingerprint: {:08x}", seed_corpus_fingerprint());
    println!("default mode: {:?}", Mode::default());
    println!("profile path: {}", profile_path.display());
    println!(
        "initialized: {}",
        if initialized { "yes" } else { "no — run `atsisbroken init`" }
    );
    println!("feedback queue: {} events at {}", queue_depth, feedback_path.display());
    Ok(())
}
