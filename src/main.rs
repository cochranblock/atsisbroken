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
    about = "ATS is broken. Fill it locally with a model you trained.",
    long_about = "Run with no subcommand to drop into the TUI (the default \
                  interface). Subcommands are available for scripting and \
                  one-shot tasks."
)]
struct Cli {
    /// Path to a profile TOML to use instead of `~/.atsisbroken/profile.toml`.
    /// Lets you maintain multiple profiles side by side (e.g. one per
    /// client if you're a career counselor).
    #[arg(long, global = true)]
    profile: Option<String>,

    /// Optional. Run with no subcommand to launch the TUI.
    #[command(subcommand)]
    cmd: Option<Cmd>,
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
    /// --strategy. With --url, drives the CDP fill loop end-to-end.
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
        /// URL to navigate to and fill. With this flag, atsisbroken
        /// launches Chromium, navigates, classifies, fills (NEVER
        /// submits), and saves a screenshot.
        #[arg(long)]
        url: Option<String>,
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
    /// Launch the TUI explicitly. Same effect as running with no
    /// subcommand. Documented so it appears in --help.
    Tui,
    /// Render one TUI tab to HTML. Hidden from --help; used by
    /// scripts/capture-screenshots.sh to produce real PNG screenshots
    /// of the TUI via headless Chromium.
    #[command(hide = true)]
    TuiSnapshot {
        #[arg(long, default_value_t = 0)]
        tab: usize,
        #[arg(long, default_value_t = 120)]
        width: u16,
        #[arg(long, default_value_t = 32)]
        height: u16,
    },
    /// Advance one rung up the autonomy ladder
    /// (TrainingWheels → Shadow → Chaos). Persists to
    /// `~/.atsisbroken/config.toml`. Prompts for confirmation
    /// unless `--yes`.
    Graduate {
        /// Skip the confirmation prompt.
        #[arg(long)]
        yes: bool,
        /// Step backward (Chaos → Shadow → TrainingWheels) instead.
        #[arg(long)]
        back: bool,
    },
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
    /// Summarize the local feedback ledger
    /// (`~/.atsisbroken/feedback.jsonl`). Read-only; shows totals,
    /// per-key acceptance, top-rejected keys, and how many entries
    /// the correction overlay holds.
    Feedback,
    /// Save a GitHub Personal Access Token (and optionally the
    /// handle) so `sync-github` can pull at the 5000 req/h rate.
    /// Without a token, sync still works but is rate-limited to
    /// 60 req/h. Token file is chmod 600 on Unix.
    ConnectGithub {
        /// GitHub login. Stored in the inventory file at sync time;
        /// can be passed here so sync-github knows whose repos to
        /// fetch without a flag.
        #[arg(long)]
        handle: Option<String>,
        /// Personal Access Token. Reads from stdin if omitted.
        /// `public_repo` + `read:user` scopes are sufficient.
        #[arg(long)]
        token: Option<String>,
    },
    /// Pull public-repo metadata, README excerpts, and recent
    /// commit messages from GitHub into
    /// `~/.atsisbroken/github_inventory.json`. Phase K's answer
    /// composer (not yet implemented) will draw on this inventory
    /// to assemble free-form answers ("describe a project,"
    /// "biggest technical challenge," etc.), citing each emitted
    /// fragment back to a public URL.
    SyncGithub {
        /// GitHub login. Falls back to the handle stored in
        /// `~/.atsisbroken/github_inventory.json` from a prior
        /// `connect-github`.
        #[arg(long)]
        handle: Option<String>,
    },
    /// Open the atsisbroken browser. Single-window, Servo-derived
    /// engine (when the engine layer lands). Optionally navigates
    /// to URL on launch. The browser IS the product surface;
    /// this subcommand is the equivalent of double-clicking the
    /// app icon. Future: become the default behavior of running
    /// `atsisbroken` with no subcommand, retiring the TUI.
    #[cfg(feature = "gui")]
    Browse {
        /// URL to open on launch. Defaults to atsisbroken://home.
        #[arg(long)]
        url: Option<String>,
    },
    /// Print a rendered page's title + body to stdout without
    /// opening a window. Used for scripting, headless inspection,
    /// and integration tests. Same routing as `browse` — internal
    /// pages render in-process via the page router; network URLs
    /// fetch + parse but only the body text is printed (no DOM
    /// dump).
    #[cfg(feature = "gui")]
    Inspect {
        /// URL to inspect. Defaults to atsisbroken://home.
        #[arg(long)]
        url: Option<String>,
    },
    /// Print the current configuration.
    Status,
}

/// clap value-parser for `--mode`. Wraps [`Mode::from_cli_str`]
/// (lib-tested) and adapts its `Option<Mode>` to the
/// `Result<_, String>` shape clap expects from a value parser.
fn parse_mode(s: &str) -> Result<Mode, String> {
    Mode::from_cli_str(s).ok_or_else(|| format!("unknown mode: {s}"))
}

// Sync main. Async-needing commands (chromiumoxide, async reqwest)
// construct a tokio runtime explicitly via `tokio_run`. The default
// path stays sync so reqwest::blocking inside cmd_browse / cmd_inspect
// never collides with a wrapping runtime — the bug from the Rust
// audit (#2). Network IO is the natural blocker for those flows.
fn main() -> Result<()> {
    let cli = Cli::parse();
    let profile_override = cli.profile.as_deref().map(std::path::PathBuf::from);
    match cli.cmd {
        None => cmd_tui(),
        Some(Cmd::Init { resume }) => cmd_init(resume, profile_override),
        Some(Cmd::Run { mode, cdp, strategy, url }) => {
            tokio_run(cmd_run(mode, cdp, strategy, url, profile_override))
        }
        Some(Cmd::Userscript) => cmd_userscript(profile_override),
        Some(Cmd::Bookmarklet) => cmd_bookmarklet(profile_override),
        Some(Cmd::Copy { key }) => cmd_copy(key, profile_override),
        Some(Cmd::Speak) => cmd_speak(profile_override),
        Some(Cmd::CdpProbe) => tokio_run(cmd_cdp_probe()),
        Some(Cmd::Graduate { yes, back }) => cmd_graduate(yes, back),
        Some(Cmd::Sync) => cmd_sync(),
        Some(Cmd::Export { out }) => cmd_export(out),
        Some(Cmd::Bridge) => cmd_bridge(),
        Some(Cmd::InstallBridge { extension_id, binary }) => cmd_install_bridge(extension_id, binary),
        Some(Cmd::Feedback) => cmd_feedback(),
        Some(Cmd::ConnectGithub { handle, token }) => cmd_connect_github(handle, token),
        Some(Cmd::SyncGithub { handle }) => tokio_run(cmd_sync_github(handle)),
        #[cfg(feature = "gui")]
        Some(Cmd::Browse { url }) => cmd_browse(url),
        #[cfg(feature = "gui")]
        Some(Cmd::Inspect { url }) => cmd_inspect(url),
        Some(Cmd::Status) => cmd_status(profile_override),
        Some(Cmd::Tui) => cmd_tui(),
        Some(Cmd::TuiSnapshot { tab, width, height }) => cmd_tui_snapshot(tab, width, height),
    }
}

/// Construct a tokio runtime locally and block_on the future.
/// Used only for commands that genuinely need tokio (chromiumoxide
/// CDP client + async reqwest::Client in github sync). Most commands
/// don't need it; the bare-sync default is what fixes the
/// "reqwest::blocking inside #[tokio::main]" panic.
fn tokio_run<F: std::future::Future<Output = Result<()>>>(future: F) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(future)
}

#[cfg(feature = "gui")]
fn cmd_browse(url: Option<String>) -> Result<()> {
    use atsisbroken::browser::{launch, BrowserConfig, Url};
    let mut config = BrowserConfig::default();
    if let Some(s) = url {
        config.start_url = s
            .parse::<Url>()
            .map_err(|e| anyhow!("invalid --url {s:?}: {e}"))?;
    }
    launch(config).map_err(|e| anyhow!("browser: {e}"))?;
    Ok(())
}

#[cfg(feature = "gui")]
fn cmd_inspect(url: Option<String>) -> Result<()> {
    use atsisbroken::browser::{inspect, Url};
    let url: Url = match url {
        Some(s) => s
            .parse()
            .map_err(|e| anyhow!("invalid --url {s:?}: {e}"))?,
        None => Url::home(),
    };
    let snap = inspect(&url).map_err(|e| anyhow!("inspect: {e}"))?;
    println!("URL: {}", snap.url);
    println!("Title: {}", snap.title);
    println!("Fields: {}", snap.fields.len());
    println!();
    println!("{}", snap.body);
    Ok(())
}

#[cfg(feature = "tui")]
fn cmd_tui() -> Result<()> {
    atsisbroken::tui::run().map_err(|e| anyhow!("{e}"))
}

#[cfg(not(feature = "tui"))]
fn cmd_tui() -> Result<()> {
    Err(anyhow!(
        "atsisbroken was built without the `tui` feature. Run a subcommand instead, or rebuild with --features tui."
    ))
}

#[cfg(feature = "tui")]
fn cmd_tui_snapshot(tab: usize, width: u16, height: u16) -> Result<()> {
    let html = atsisbroken::tui::render_html_for_screenshot(tab, width, height)
        .map_err(|e| anyhow!("{e}"))?;
    print!("{html}");
    Ok(())
}

#[cfg(not(feature = "tui"))]
fn cmd_tui_snapshot(_tab: usize, _width: u16, _height: u16) -> Result<()> {
    Err(anyhow!("tui feature not built"))
}

fn cmd_init(resume_path: Option<String>, profile_override: Option<std::path::PathBuf>) -> Result<()> {
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
    let path = paths::profile_path_with_override(profile_override.as_deref());
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
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
    url: Option<String>,
    profile_override: Option<std::path::PathBuf>,
) -> Result<()> {
    let profile = load_profile_or_hint(profile_override.as_deref())?;

    // If --url is given, take the CDP-launch fill path directly,
    // regardless of strategy detection. The user has been explicit.
    if let Some(u) = url.as_deref() {
        return run_cdp_url(&profile, u).await;
    }

    let chosen = match forced.as_deref() {
        Some(s) => parse_strategy_override(s)?,
        None => strategy::detect(),
    };
    eprintln!("atsisbroken {} — strategy: {:?}", version(), chosen);
    match chosen {
        Strategy::CdpAttach { endpoint } => run_cdp_attach(&endpoint).await,
        Strategy::CdpLaunch { browser_path: _ } => {
            eprintln!(
                "  cdp-launch needs a URL. Run with `--url <ATS-form-url>` to drive the fill loop."
            );
            eprintln!("  Falling through to userscript output for the no-URL case:");
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

async fn run_cdp_url(profile: &Profile, url: &str) -> Result<()> {
    use atsisbroken::run_loop;
    let screenshot_dir = paths::atsisbroken_dir();
    let summary = run_loop::run_against_url(profile, url, Some(&screenshot_dir))
        .await
        .context("run_against_url")?;
    let queue_depth = run_loop::persist_feedback(summary.feedback_events.clone())
        .context("persist feedback")?;
    eprintln!("{}", run_loop::format_summary(&summary, queue_depth));
    Ok(())
}

/// Wrap the lib's [`Strategy::from_cli_str`] in anyhow::Error
/// for the main fn's Result. Lib returns String (no anyhow dep);
/// callers want anyhow::Error so they can `?` cleanly.
fn parse_strategy_override(s: &str) -> Result<Strategy> {
    Strategy::from_cli_str(s).map_err(|e| anyhow!(e))
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

fn load_profile_or_hint(override_path: Option<&std::path::Path>) -> Result<Profile> {
    let path = paths::profile_path_with_override(override_path);
    if !path.exists() {
        return Err(anyhow!(
            "profile not found at {} — run `atsisbroken init` first",
            path.display()
        ));
    }
    let text = std::fs::read_to_string(&path)?;
    Ok(toml::from_str(&text).context("parse profile.toml")?)
}

fn cmd_userscript(profile_override: Option<std::path::PathBuf>) -> Result<()> {
    let p = load_profile_or_hint(profile_override.as_deref())?;
    print!("{}", strategy::userscript(&p));
    Ok(())
}

fn cmd_bookmarklet(profile_override: Option<std::path::PathBuf>) -> Result<()> {
    let p = load_profile_or_hint(profile_override.as_deref())?;
    println!("{}", strategy::bookmarklet(&p));
    Ok(())
}

fn cmd_copy(key: String, profile_override: Option<std::path::PathBuf>) -> Result<()> {
    let p = load_profile_or_hint(profile_override.as_deref())?;
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

fn cmd_speak(profile_override: Option<std::path::PathBuf>) -> Result<()> {
    let p = load_profile_or_hint(profile_override.as_deref())?;
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

// No `#[cfg(test)] mod tests` here. main.rs is the production
// binary's entry point; tests live in the lib (Mode::from_cli_str,
// Strategy::from_cli_str, ClipboardTool::args) and in
// tests/cli_smoke.rs (end-to-end CLI behavior). The TRIPLE SIMS
// gate runner is the second declared bin, src/bin/atsisbroken-test.rs.

fn cmd_graduate(yes: bool, back: bool) -> Result<()> {
    use atsisbroken::config::Config;
    use std::io::BufRead;

    let mut cfg = Config::load().context("load config")?;
    let current = cfg.mode;
    let target = if back {
        match current {
            Mode::Chaos => Mode::Shadow,
            Mode::Shadow => Mode::TrainingWheels,
            Mode::TrainingWheels => {
                return Err(anyhow!(
                    "already at TrainingWheels — already the most supervised mode"
                ));
            }
        }
    } else {
        match current {
            Mode::TrainingWheels => Mode::Shadow,
            Mode::Shadow => Mode::Chaos,
            Mode::Chaos => {
                return Err(anyhow!(
                    "already at Chaos — pass --back to step down"
                ));
            }
        }
    };

    eprintln!(
        "atsisbroken {} — current mode: {:?} → target mode: {:?}",
        version(),
        current,
        target
    );
    if !yes {
        eprint!("  proceed? [y/N] ");
        let _ = std::io::Write::flush(&mut std::io::stderr());
        let mut line = String::new();
        std::io::stdin().lock().read_line(&mut line)?;
        let answer = line.trim().to_ascii_lowercase();
        if !(answer == "y" || answer == "yes") {
            eprintln!("  cancelled. mode unchanged.");
            return Ok(());
        }
    }

    if back {
        cfg.step_back().map_err(|e| anyhow!("{e}"))?;
    } else {
        cfg.graduate().map_err(|e| anyhow!("{e}"))?;
    }
    paths::ensure_dir()?;
    cfg.save().context("save config")?;
    eprintln!(
        "  ✓ mode persisted to {}",
        paths::config_path().display()
    );
    Ok(())
}

fn cmd_sync() -> Result<()> {
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

fn cmd_export(out: String) -> Result<()> {
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

fn cmd_bridge() -> Result<()> {
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

fn cmd_feedback() -> Result<()> {
    use atsisbroken::learning::{CorrectionOverlay, FeedbackStats};
    let path = paths::feedback_jsonl_path();
    let q = FeedbackQueue::load_from(&path).context("load feedback queue")?;
    let stats = FeedbackStats::from_queue(&q.events);
    let overlay = CorrectionOverlay::from_queue(&q.events);

    println!("atsisbroken {}", version());
    println!("feedback ledger: {}", path.display());
    println!();
    if stats.total == 0 {
        println!("  (no events yet — run `atsisbroken run --url <ATS-URL>` to start collecting)");
        return Ok(());
    }
    println!(
        "  total events: {}     accepted: {}     rejected: {}",
        stats.total, stats.accepted, stats.rejected
    );
    println!("  correction overlay holds {} prior decision(s)", overlay.len());
    if !stats.top_rejected_keys.is_empty() {
        println!();
        println!("  top rejected keys (run loop will skip these silently next time):");
        for (k, n) in &stats.top_rejected_keys {
            println!("    {n:>4}  {k}");
        }
    }
    if !stats.per_key_accepted.is_empty() {
        println!();
        println!("  accepted-by-key (graduated to auto-fill in any mode):");
        let mut sorted: Vec<(&String, &usize)> = stats.per_key_accepted.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        for (k, n) in sorted.iter().take(8) {
            println!("    {n:>4}  {k}");
        }
    }
    Ok(())
}

fn cmd_status(profile_override: Option<std::path::PathBuf>) -> Result<()> {
    let profile_path = paths::profile_path_with_override(profile_override.as_deref());
    let exists = profile_path.exists();
    let populated = if exists {
        std::fs::read_to_string(&profile_path)
            .ok()
            .and_then(|t| toml::from_str::<Profile>(&t).ok())
            .map(|p| p.is_meaningfully_populated())
            .unwrap_or(false)
    } else {
        false
    };
    let feedback_path = paths::feedback_jsonl_path();
    let queue_depth = if feedback_path.exists() {
        FeedbackQueue::load_from(&feedback_path).map(|q| q.len()).unwrap_or(0)
    } else {
        0
    };
    let detected = atsisbroken::browser_detect::default_browser();
    let cfg = atsisbroken::config::Config::load().unwrap_or_default();
    println!("atsisbroken {}", version());
    println!("seed corpus fingerprint: {:08x}", seed_corpus_fingerprint());
    println!("mode: {:?}", cfg.mode);
    if cfg.mode == Mode::default() && !paths::config_path().exists() {
        println!("  (default — never run `atsisbroken graduate`)");
    } else {
        println!("  (persisted in {})", paths::config_path().display());
    }
    println!("profile path: {}", profile_path.display());
    let init_state = if !exists {
        "no — run `atsisbroken init`"
    } else if !populated {
        "exists but empty — re-run `atsisbroken init` with a fuller resume"
    } else {
        "yes"
    };
    println!("initialized: {init_state}");
    println!("feedback queue: {} events at {}", queue_depth, feedback_path.display());
    match detected {
        Some(b) => println!(
            "default browser: {:?}{}",
            b.kind,
            b.path
                .as_ref()
                .map(|p| format!(" ({})", p.display()))
                .unwrap_or_default()
        ),
        None => println!("default browser: (not detected)"),
    }
    Ok(())
}

fn cmd_connect_github(handle: Option<String>, token: Option<String>) -> Result<()> {
    use atsisbroken::github;
    use std::io::Read;

    paths::ensure_dir().map_err(|e| anyhow!("ensure dir: {e}"))?;

    // Resolve the token: --token wins; otherwise read from stdin.
    // Stdin reads support piping (e.g., `pass show github | atsisbroken connect-github`).
    let token = match token {
        Some(t) => t,
        None => {
            eprintln!("Paste your GitHub Personal Access Token, then press Ctrl+D:");
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            buf
        }
    };
    let token = token.trim();
    if token.is_empty() {
        return Err(anyhow!(
            "no token provided (pass --token or paste before EOF). \
             Anonymous mode is supported by `sync-github` directly — \
             omit `connect-github` if you don't have a token."
        ));
    }
    let token_path = paths::github_token_path();
    github::save_github_token(token, &token_path)
        .map_err(|e| anyhow!("write token: {e}"))?;
    eprintln!("token saved to {} (chmod 600 on Unix)", token_path.display());

    if let Some(h) = handle {
        // Persist the handle alongside the token by stashing it in
        // the inventory file's `handle` field; sync-github reads it
        // when the user runs sync without --handle.
        let inv_path = paths::github_inventory_path();
        let mut inv = github::GithubInventory::load_from(&inv_path)
            .map_err(|e| anyhow!("load inventory: {e}"))?;
        inv.handle = h.clone();
        inv.save_to(&inv_path)
            .map_err(|e| anyhow!("save inventory: {e}"))?;
        eprintln!("handle saved: {h}");
    }
    eprintln!("next: run `atsisbroken sync-github` to fetch your repos.");
    Ok(())
}

async fn cmd_sync_github(handle: Option<String>) -> Result<()> {
    use atsisbroken::github;

    paths::ensure_dir().map_err(|e| anyhow!("ensure dir: {e}"))?;

    // Resolve the handle: --handle wins; otherwise pull from the
    // existing inventory (set by `connect-github --handle`).
    let inv_path = paths::github_inventory_path();
    let handle = match handle {
        Some(h) => h,
        None => {
            let inv = github::GithubInventory::load_from(&inv_path)
                .map_err(|e| anyhow!("load inventory: {e}"))?;
            if inv.handle.is_empty() {
                return Err(anyhow!(
                    "no handle on disk and --handle not provided. \
                     run `atsisbroken connect-github --handle <login>` first."
                ));
            }
            inv.handle
        }
    };

    let token_path = paths::github_token_path();
    let token = github::load_github_token(&token_path)
        .map_err(|e| anyhow!("read token: {e}"))?;
    if token.is_none() {
        eprintln!(
            "no token at {} — using anonymous mode (rate-limited 60 req/h).",
            token_path.display()
        );
    }

    eprintln!("syncing GitHub for {handle}…");
    let inv = github::sync_user_inventory(&handle, token.as_deref())
        .await
        .map_err(|e| anyhow!("sync: {e}"))?;
    inv.save_to(&inv_path)
        .map_err(|e| anyhow!("save inventory: {e}"))?;

    eprintln!(
        "synced {} public repos ({} top-N deep-fetched) to {}",
        inv.public_repos.len(),
        inv.public_repos
            .iter()
            .filter(|r| !r.readme_excerpts.is_empty() || !r.recent_commit_messages.is_empty())
            .count(),
        inv_path.display()
    );
    if inv.public_repos.is_empty() {
        eprintln!("(handle has no public repos, or the user is private.)");
    }
    Ok(())
}
