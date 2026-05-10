// SPDX-License-Identifier: Unlicense

//! CLI dispatch — moved out of `main.rs` so tests can drive it
//! in-process. The cochranblock pattern: the production binary is
//! a thin shell, every command is a `pub fn` on the lib that takes
//! explicit handles for the things it would otherwise read from
//! the process environment (stdin, stdout, stderr, $HOME).
//!
//! `src/main.rs` builds a [`CliCtx`] from real handles and calls
//! [`run`]. `src/tests/cli_smoke.rs` builds a [`CliCtx`] over a
//! [`tempdir`] home + `Vec<u8>` writers + an `&[u8]` stdin, runs
//! the same dispatch, and asserts on the captured output bytes.
//!
//! Two commands fall outside the testable surface:
//!
//! - `Browse` — needs winit's event loop + a real OS window.
//! - `Tui`   — needs a real terminal.
//!
//! Both are shimmed: production calls into the GUI/TUI modules as
//! before; tests don't construct these subcommands.

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::strategy::{ClipboardTool, Strategy};
use crate::{
    bridge::serve_native_messaging, cdp, paths, resume, seed_corpus_fingerprint, version,
    FeedbackQueue, Mode, Profile,
};

#[derive(Parser, Debug)]
#[command(
    name = "atsisbroken",
    version,
    about = "ATS is broken. Fill it locally with a model you trained.",
    long_about = "Run with no subcommand to drop into the TUI (the default \
                  interface). Subcommands are available for scripting and \
                  one-shot tasks."
)]
pub struct Cli {
    /// Path to a profile TOML to use instead of `~/.atsisbroken/profile.toml`.
    #[arg(long, global = true)]
    pub profile: Option<String>,

    /// Optional. Run with no subcommand to launch the TUI.
    #[command(subcommand)]
    pub cmd: Option<Cmd>,
}

#[derive(Subcommand, Debug)]
pub enum Cmd {
    Init {
        #[arg(long)]
        resume: Option<String>,
    },
    Run {
        #[arg(long, value_parser = parse_mode)]
        mode: Option<Mode>,
        #[arg(long)]
        cdp: Option<String>,
        #[arg(long)]
        strategy: Option<String>,
        #[arg(long)]
        url: Option<String>,
    },
    Userscript,
    Bookmarklet,
    Copy {
        key: String,
    },
    Speak,
    CdpProbe,
    Tui,
    #[command(hide = true)]
    TuiSnapshot {
        #[arg(long, default_value_t = 0)]
        tab: usize,
        #[arg(long, default_value_t = 120)]
        width: u16,
        #[arg(long, default_value_t = 32)]
        height: u16,
    },
    Graduate {
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        back: bool,
    },
    Sync,
    Export {
        #[arg(long)]
        out: String,
    },
    Bridge,
    InstallBridge {
        #[arg(long)]
        extension_id: String,
        #[arg(long)]
        binary: Option<String>,
    },
    Feedback,
    ConnectGithub {
        #[arg(long)]
        handle: Option<String>,
        #[arg(long)]
        token: Option<String>,
    },
    SyncGithub {
        #[arg(long)]
        handle: Option<String>,
    },
    #[cfg(feature = "gui")]
    Browse {
        #[arg(long)]
        url: Option<String>,
    },
    #[cfg(feature = "gui")]
    Inspect {
        #[arg(long)]
        url: Option<String>,
    },
    Status,
}

/// clap value-parser for `--mode`.
pub fn parse_mode(s: &str) -> std::result::Result<Mode, String> {
    Mode::from_cli_str(s).ok_or_else(|| format!("unknown mode: {s}"))
}

/// Everything a CLI command needs that would otherwise come from
/// the process environment. Tests build one of these over an
/// in-memory home dir + Vec<u8> writers; production builds one
/// over the real `$HOME` and the real stdio.
pub struct CliCtx<R: Read, W: Write, E: Write> {
    pub home: PathBuf,
    pub stdin: R,
    pub stdout: W,
    pub stderr: E,
}

impl<R: Read, W: Write, E: Write> CliCtx<R, W, E> {
    pub fn new(home: PathBuf, stdin: R, stdout: W, stderr: E) -> Self {
        Self {
            home,
            stdin,
            stdout,
            stderr,
        }
    }
}

/// Dispatch a parsed `Cli` against `ctx`. The single entry point
/// production `main` and `tests::cli_smoke` both call.
pub fn run<R: Read, W: Write, E: Write>(cli: Cli, ctx: &mut CliCtx<R, W, E>) -> Result<()> {
    let profile_override = cli.profile.as_deref().map(PathBuf::from);
    match cli.cmd {
        None => cmd_tui(),
        Some(Cmd::Init { resume }) => cmd_init(resume, profile_override.as_deref(), ctx),
        Some(Cmd::Run {
            mode,
            cdp,
            strategy,
            url,
        }) => tokio_run(cmd_run(
            mode,
            cdp,
            strategy,
            url,
            profile_override.as_deref(),
            ctx,
        )),
        Some(Cmd::Userscript) => cmd_userscript(profile_override.as_deref(), ctx),
        Some(Cmd::Bookmarklet) => cmd_bookmarklet(profile_override.as_deref(), ctx),
        Some(Cmd::Copy { key }) => cmd_copy(key, profile_override.as_deref(), ctx),
        Some(Cmd::Speak) => cmd_speak(profile_override.as_deref(), ctx),
        Some(Cmd::CdpProbe) => tokio_run(cmd_cdp_probe(ctx)),
        Some(Cmd::Graduate { yes, back }) => cmd_graduate(yes, back, ctx),
        Some(Cmd::Sync) => cmd_sync(ctx),
        Some(Cmd::Export { out }) => cmd_export(out, ctx),
        Some(Cmd::Bridge) => cmd_bridge(ctx),
        Some(Cmd::InstallBridge {
            extension_id,
            binary,
        }) => cmd_install_bridge(extension_id, binary, ctx),
        Some(Cmd::Feedback) => cmd_feedback(ctx),
        Some(Cmd::ConnectGithub { handle, token }) => cmd_connect_github(handle, token, ctx),
        Some(Cmd::SyncGithub { handle }) => tokio_run(cmd_sync_github(handle, ctx)),
        #[cfg(feature = "gui")]
        Some(Cmd::Browse { url }) => cmd_browse(url),
        #[cfg(feature = "gui")]
        Some(Cmd::Inspect { url }) => cmd_inspect(url, ctx),
        Some(Cmd::Status) => cmd_status(profile_override.as_deref(), ctx),
        Some(Cmd::Tui) => cmd_tui(),
        Some(Cmd::TuiSnapshot { tab, width, height }) => {
            cmd_tui_snapshot(tab, width, height, ctx)
        }
    }
}

/// Construct a tokio runtime and block_on the future. Used only
/// for commands that genuinely need tokio (chromiumoxide,
/// async reqwest in github sync).
fn tokio_run<F: std::future::Future<Output = Result<()>>>(future: F) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(future)
}

// ─── per-command implementations ────────────────────────────────────

#[cfg(feature = "gui")]
fn cmd_browse(url: Option<String>) -> Result<()> {
    use crate::browser::{launch, BrowserConfig, Url};
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
fn cmd_inspect<R: Read, W: Write, E: Write>(
    url: Option<String>,
    ctx: &mut CliCtx<R, W, E>,
) -> Result<()> {
    use crate::browser::{inspect, Url};
    let url: Url = match url {
        Some(s) => s
            .parse()
            .map_err(|e| anyhow!("invalid --url {s:?}: {e}"))?,
        None => Url::home(),
    };
    let snap = inspect(&url).map_err(|e| anyhow!("inspect: {e}"))?;
    writeln!(ctx.stdout, "URL: {}", snap.url)?;
    writeln!(ctx.stdout, "Title: {}", snap.title)?;
    writeln!(ctx.stdout, "Fields: {}", snap.fields.len())?;
    writeln!(ctx.stdout)?;
    writeln!(ctx.stdout, "{}", snap.body)?;
    Ok(())
}

#[cfg(feature = "tui")]
fn cmd_tui() -> Result<()> {
    crate::tui::run().map_err(|e| anyhow!("{e}"))
}

#[cfg(not(feature = "tui"))]
fn cmd_tui() -> Result<()> {
    Err(anyhow!(
        "atsisbroken was built without the `tui` feature. Run a subcommand instead, or rebuild with --features tui."
    ))
}

#[cfg(feature = "tui")]
fn cmd_tui_snapshot<R: Read, W: Write, E: Write>(
    tab: usize,
    width: u16,
    height: u16,
    ctx: &mut CliCtx<R, W, E>,
) -> Result<()> {
    let html = crate::tui::render_html_for_screenshot(tab, width, height)
        .map_err(|e| anyhow!("{e}"))?;
    write!(ctx.stdout, "{html}")?;
    Ok(())
}

#[cfg(not(feature = "tui"))]
fn cmd_tui_snapshot<R: Read, W: Write, E: Write>(
    _tab: usize,
    _width: u16,
    _height: u16,
    _ctx: &mut CliCtx<R, W, E>,
) -> Result<()> {
    Err(anyhow!("tui feature not built"))
}

fn cmd_init<R: Read, W: Write, E: Write>(
    resume_path: Option<String>,
    profile_override: Option<&Path>,
    ctx: &mut CliCtx<R, W, E>,
) -> Result<()> {
    let text = match resume_path {
        Some(path) => std::fs::read_to_string(&path)
            .with_context(|| format!("read resume file {path}"))?,
        None => {
            writeln!(ctx.stderr, "paste resume text and press ctrl-D when done:")?;
            let mut buf = String::new();
            ctx.stdin.read_to_string(&mut buf)?;
            buf
        }
    };
    if text.trim().is_empty() {
        return Err(anyhow!("resume text was empty"));
    }
    let profile: Profile = resume::parse_resume(&text);
    let path =
        paths::profile_path_with_override_and_home(profile_override, &ctx.home);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let toml_text = toml::to_string_pretty(&profile).context("serialize profile")?;
    std::fs::write(&path, toml_text)
        .with_context(|| format!("write {}", path.display()))?;
    writeln!(
        ctx.stderr,
        "atsisbroken {} — wrote profile to {}",
        version(),
        path.display()
    )?;
    writeln!(
        ctx.stderr,
        "  full_name={:?} email={:?} phone={:?}",
        profile.full_name, profile.email, profile.phone
    )?;
    Ok(())
}

async fn cmd_run<R: Read, W: Write, E: Write>(
    _mode: Option<Mode>,
    _cdp: Option<String>,
    forced: Option<String>,
    url: Option<String>,
    profile_override: Option<&Path>,
    ctx: &mut CliCtx<R, W, E>,
) -> Result<()> {
    let profile = load_profile_or_hint(profile_override, &ctx.home)?;

    if let Some(u) = url.as_deref() {
        return run_cdp_url(&profile, u, ctx).await;
    }

    let chosen = match forced.as_deref() {
        Some(s) => Strategy::from_cli_str(s).map_err(|e| anyhow!(e))?,
        None => crate::strategy::detect(),
    };
    writeln!(
        ctx.stderr,
        "atsisbroken {} — strategy: {:?}",
        version(),
        chosen
    )?;
    match chosen {
        Strategy::CdpAttach { endpoint } => run_cdp_attach(&endpoint, ctx).await,
        Strategy::CdpLaunch { browser_path: _ } => {
            writeln!(
                ctx.stderr,
                "  cdp-launch needs a URL. Run with `--url <ATS-form-url>` to drive the fill loop."
            )?;
            writeln!(
                ctx.stderr,
                "  Falling through to userscript output for the no-URL case:"
            )?;
            write!(ctx.stdout, "{}", crate::strategy::userscript(&profile))?;
            Ok(())
        }
        Strategy::Extension => {
            writeln!(
                ctx.stderr,
                "  extension is installed; the extension fills via its content script. \n  Run `atsisbroken bridge` (Chrome will spawn it on demand)."
            )?;
            Ok(())
        }
        Strategy::Userscript => {
            write!(ctx.stdout, "{}", crate::strategy::userscript(&profile))?;
            Ok(())
        }
        Strategy::Bookmarklet => {
            writeln!(ctx.stdout, "{}", crate::strategy::bookmarklet(&profile))?;
            Ok(())
        }
        Strategy::Clipboard { tool } => {
            writeln!(
                ctx.stderr,
                "  clipboard tool: {} — use `atsisbroken copy <key>` to copy individual fields.",
                tool.binary()
            )?;
            Ok(())
        }
        Strategy::Speak => {
            crate::strategy::speak(&profile, &mut ctx.stdout)?;
            Ok(())
        }
    }
}

async fn run_cdp_url<R: Read, W: Write, E: Write>(
    profile: &Profile,
    url: &str,
    ctx: &mut CliCtx<R, W, E>,
) -> Result<()> {
    use crate::run_loop;
    let screenshot_dir = paths::atsisbroken_dir_with_home(&ctx.home);
    let summary = run_loop::run_against_url(profile, url, Some(&screenshot_dir))
        .await
        .context("run_against_url")?;
    let queue_depth = run_loop::persist_feedback(summary.feedback_events.clone())
        .context("persist feedback")?;
    writeln!(
        ctx.stderr,
        "{}",
        run_loop::format_summary(&summary, queue_depth)
    )?;
    Ok(())
}

async fn run_cdp_attach<R: Read, W: Write, E: Write>(
    endpoint: &str,
    ctx: &mut CliCtx<R, W, E>,
) -> Result<()> {
    let (_host, port) =
        cdp::parse_endpoint(endpoint).ok_or_else(|| anyhow!("bad endpoint: {endpoint}"))?;
    let tabs = cdp::list_tabs(port).context("list CDP tabs")?;
    if tabs.is_empty() {
        writeln!(ctx.stderr, "  CDP attached on port {port} — no tabs open.")?;
    } else {
        writeln!(
            ctx.stderr,
            "  CDP attached on port {port} — {} tab(s):",
            tabs.len()
        )?;
        for (i, t) in tabs.iter().enumerate() {
            writeln!(ctx.stderr, "    [{i}] {} — {}", t.title, t.url)?;
        }
        writeln!(
            ctx.stderr,
            "  Fill loop wires in next iteration. For now, use --strategy=userscript or --strategy=bookmarklet."
        )?;
    }
    Ok(())
}

fn load_profile_or_hint(
    override_path: Option<&Path>,
    home: &Path,
) -> Result<Profile> {
    let path = paths::profile_path_with_override_and_home(override_path, home);
    if !path.exists() {
        return Err(anyhow!(
            "profile not found at {} — run `atsisbroken init` first",
            path.display()
        ));
    }
    let text = std::fs::read_to_string(&path)?;
    Ok(toml::from_str(&text).context("parse profile.toml")?)
}

fn cmd_userscript<R: Read, W: Write, E: Write>(
    profile_override: Option<&Path>,
    ctx: &mut CliCtx<R, W, E>,
) -> Result<()> {
    let p = load_profile_or_hint(profile_override, &ctx.home)?;
    write!(ctx.stdout, "{}", crate::strategy::userscript(&p))?;
    Ok(())
}

fn cmd_bookmarklet<R: Read, W: Write, E: Write>(
    profile_override: Option<&Path>,
    ctx: &mut CliCtx<R, W, E>,
) -> Result<()> {
    let p = load_profile_or_hint(profile_override, &ctx.home)?;
    writeln!(ctx.stdout, "{}", crate::strategy::bookmarklet(&p))?;
    Ok(())
}

fn cmd_copy<R: Read, W: Write, E: Write>(
    key: String,
    profile_override: Option<&Path>,
    ctx: &mut CliCtx<R, W, E>,
) -> Result<()> {
    let p = load_profile_or_hint(profile_override, &ctx.home)?;
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
    let tool: ClipboardTool = crate::strategy::detect_clipboard_tool()
        .ok_or_else(|| anyhow!("no clipboard tool found (pbcopy/xclip/wl-copy/clip.exe)"))?;
    crate::strategy::copy_to_clipboard(tool, &value)?;
    writeln!(
        ctx.stderr,
        "atsisbroken {} — copied {} via {}",
        version(),
        key,
        tool.binary()
    )?;
    Ok(())
}

fn cmd_speak<R: Read, W: Write, E: Write>(
    profile_override: Option<&Path>,
    ctx: &mut CliCtx<R, W, E>,
) -> Result<()> {
    let p = load_profile_or_hint(profile_override, &ctx.home)?;
    crate::strategy::speak(&p, &mut ctx.stdout)?;
    Ok(())
}

async fn cmd_cdp_probe<R: Read, W: Write, E: Write>(
    ctx: &mut CliCtx<R, W, E>,
) -> Result<()> {
    match crate::strategy::probe_cdp_endpoint() {
        Some(endpoint) => run_cdp_attach(&endpoint, ctx).await,
        None => {
            writeln!(
                ctx.stderr,
                "atsisbroken {} — no Chromium debug port reachable. Launch Chrome with --remote-debugging-port=9222 to enable CDP attach.",
                version()
            )?;
            Ok(())
        }
    }
}

fn cmd_graduate<R: Read, W: Write, E: Write>(
    yes: bool,
    back: bool,
    ctx: &mut CliCtx<R, W, E>,
) -> Result<()> {
    use crate::config::Config;

    // load_from with explicit home, falling back to default (no file).
    let cfg_path = paths::config_path_with_home(&ctx.home);
    let mut cfg = Config::load_from(&cfg_path).context("load config")?;
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
                return Err(anyhow!("already at Chaos — pass --back to step down"));
            }
        }
    };

    writeln!(
        ctx.stderr,
        "atsisbroken {} — current mode: {:?} → target mode: {:?}",
        version(),
        current,
        target
    )?;
    if !yes {
        write!(ctx.stderr, "  proceed? [y/N] ")?;
        ctx.stderr.flush()?;
        let mut line = String::new();
        // Read a single line from stdin. We read up to a newline.
        let mut buf = [0u8; 1];
        loop {
            match ctx.stdin.read(&mut buf) {
                Ok(0) => break,
                Ok(_) => {
                    if buf[0] == b'\n' {
                        break;
                    }
                    line.push(buf[0] as char);
                }
                Err(e) => return Err(anyhow!("stdin: {e}")),
            }
        }
        let answer = line.trim().to_ascii_lowercase();
        if !(answer == "y" || answer == "yes") {
            writeln!(ctx.stderr, "  cancelled. mode unchanged.")?;
            return Ok(());
        }
    }

    if back {
        cfg.step_back().map_err(|e| anyhow!("{e}"))?;
    } else {
        cfg.graduate().map_err(|e| anyhow!("{e}"))?;
    }
    paths::ensure_dir_with_home(&ctx.home)?;
    cfg.save_to(&cfg_path).context("save config")?;
    writeln!(ctx.stderr, "  ✓ mode persisted to {}", cfg_path.display())?;
    Ok(())
}

fn cmd_sync<R: Read, W: Write, E: Write>(ctx: &mut CliCtx<R, W, E>) -> Result<()> {
    let path = paths::feedback_jsonl_path_with_home(&ctx.home);
    let q = FeedbackQueue::load_from(&path).context("load feedback queue")?;
    if q.is_empty() {
        writeln!(
            ctx.stderr,
            "atsisbroken {} — sync: feedback queue is empty, nothing to do",
            version()
        )?;
    } else {
        writeln!(
            ctx.stderr,
            "atsisbroken {} — sync: {} events queued at {}. Delivery destination not configured (LocalOnly).",
            version(),
            q.len(),
            path.display()
        )?;
    }
    Ok(())
}

fn cmd_export<R: Read, W: Write, E: Write>(
    out: String,
    ctx: &mut CliCtx<R, W, E>,
) -> Result<()> {
    let path = paths::feedback_jsonl_path_with_home(&ctx.home);
    let q = FeedbackQueue::load_from(&path).context("load feedback queue")?;
    std::fs::write(&out, q.to_jsonl().context("serialize jsonl")?)
        .with_context(|| format!("write {out}"))?;
    writeln!(
        ctx.stderr,
        "atsisbroken {} — export: {} events → {}",
        version(),
        q.len(),
        out
    )?;
    Ok(())
}

fn cmd_bridge<R: Read, W: Write, E: Write>(ctx: &mut CliCtx<R, W, E>) -> Result<()> {
    let path = paths::feedback_jsonl_path_with_home(&ctx.home);
    let mut queue = FeedbackQueue::load_from(&path).context("load feedback queue")?;
    let result = serve_native_messaging(&mut ctx.stdin, &mut ctx.stdout, &mut queue);
    paths::ensure_dir_with_home(&ctx.home)?;
    queue.save_to(&path).context("save feedback queue")?;
    result.map_err(|e| anyhow!("bridge: {e}"))
}

fn cmd_install_bridge<R: Read, W: Write, E: Write>(
    extension_id: String,
    binary: Option<String>,
    ctx: &mut CliCtx<R, W, E>,
) -> Result<()> {
    let host_dir = paths::chrome_native_host_dir_with_home(&ctx.home)
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
    writeln!(
        ctx.stderr,
        "atsisbroken {} — installed Native Messaging host at {}",
        version(),
        manifest_path.display()
    )?;
    Ok(())
}

fn cmd_feedback<R: Read, W: Write, E: Write>(
    ctx: &mut CliCtx<R, W, E>,
) -> Result<()> {
    use crate::learning::{CorrectionOverlay, FeedbackStats};
    let path = paths::feedback_jsonl_path_with_home(&ctx.home);
    let q = FeedbackQueue::load_from(&path).context("load feedback queue")?;
    let stats = FeedbackStats::from_queue(&q.events);
    let overlay = CorrectionOverlay::from_queue(&q.events);

    writeln!(ctx.stdout, "atsisbroken {}", version())?;
    writeln!(ctx.stdout, "feedback ledger: {}", path.display())?;
    writeln!(ctx.stdout)?;
    if stats.total == 0 {
        writeln!(
            ctx.stdout,
            "  (no events yet — run `atsisbroken run --url <ATS-URL>` to start collecting)"
        )?;
        return Ok(());
    }
    writeln!(
        ctx.stdout,
        "  total events: {}     accepted: {}     rejected: {}",
        stats.total, stats.accepted, stats.rejected
    )?;
    writeln!(
        ctx.stdout,
        "  correction overlay holds {} prior decision(s)",
        overlay.len()
    )?;
    if !stats.top_rejected_keys.is_empty() {
        writeln!(ctx.stdout)?;
        writeln!(
            ctx.stdout,
            "  top rejected keys (run loop will skip these silently next time):"
        )?;
        for (k, n) in &stats.top_rejected_keys {
            writeln!(ctx.stdout, "    {n:>4}  {k}")?;
        }
    }
    if !stats.per_key_accepted.is_empty() {
        writeln!(ctx.stdout)?;
        writeln!(
            ctx.stdout,
            "  accepted-by-key (graduated to auto-fill in any mode):"
        )?;
        let mut sorted: Vec<(&String, &usize)> = stats.per_key_accepted.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        for (k, n) in sorted.iter().take(8) {
            writeln!(ctx.stdout, "    {n:>4}  {k}")?;
        }
    }
    Ok(())
}

fn cmd_status<R: Read, W: Write, E: Write>(
    profile_override: Option<&Path>,
    ctx: &mut CliCtx<R, W, E>,
) -> Result<()> {
    let profile_path =
        paths::profile_path_with_override_and_home(profile_override, &ctx.home);
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
    let feedback_path = paths::feedback_jsonl_path_with_home(&ctx.home);
    let queue_depth = if feedback_path.exists() {
        FeedbackQueue::load_from(&feedback_path)
            .map(|q| q.len())
            .unwrap_or(0)
    } else {
        0
    };
    let detected = crate::browser_detect::default_browser();
    let cfg_path = paths::config_path_with_home(&ctx.home);
    let cfg = crate::config::Config::load_from(&cfg_path).unwrap_or_default();
    writeln!(ctx.stdout, "atsisbroken {}", version())?;
    writeln!(
        ctx.stdout,
        "seed corpus fingerprint: {:08x}",
        seed_corpus_fingerprint()
    )?;
    writeln!(ctx.stdout, "mode: {:?}", cfg.mode)?;
    if cfg.mode == Mode::default() && !cfg_path.exists() {
        writeln!(ctx.stdout, "  (default — never run `atsisbroken graduate`)")?;
    } else {
        writeln!(ctx.stdout, "  (persisted in {})", cfg_path.display())?;
    }
    writeln!(ctx.stdout, "profile path: {}", profile_path.display())?;
    let init_state = if !exists {
        "no — run `atsisbroken init`"
    } else if !populated {
        "exists but empty — re-run `atsisbroken init` with a fuller resume"
    } else {
        "yes"
    };
    writeln!(ctx.stdout, "initialized: {init_state}")?;
    writeln!(
        ctx.stdout,
        "feedback queue: {} events at {}",
        queue_depth,
        feedback_path.display()
    )?;
    match detected {
        Some(b) => writeln!(
            ctx.stdout,
            "default browser: {:?}{}",
            b.kind,
            b.path
                .as_ref()
                .map(|p| format!(" ({})", p.display()))
                .unwrap_or_default()
        )?,
        None => writeln!(ctx.stdout, "default browser: (not detected)")?,
    }
    Ok(())
}

fn cmd_connect_github<R: Read, W: Write, E: Write>(
    handle: Option<String>,
    token: Option<String>,
    ctx: &mut CliCtx<R, W, E>,
) -> Result<()> {
    use crate::github;

    paths::ensure_dir_with_home(&ctx.home).map_err(|e| anyhow!("ensure dir: {e}"))?;

    let token = match token {
        Some(t) => t,
        None => {
            writeln!(
                ctx.stderr,
                "Paste your GitHub Personal Access Token, then press Ctrl+D:"
            )?;
            let mut buf = String::new();
            ctx.stdin.read_to_string(&mut buf)?;
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
    let token_path = paths::github_token_path_with_home(&ctx.home);
    github::save_github_token(token, &token_path).map_err(|e| anyhow!("write token: {e}"))?;
    writeln!(
        ctx.stderr,
        "token saved to {} (chmod 600 on Unix)",
        token_path.display()
    )?;

    if let Some(h) = handle {
        let inv_path = paths::github_inventory_path_with_home(&ctx.home);
        let mut inv = github::GithubInventory::load_from(&inv_path)
            .map_err(|e| anyhow!("load inventory: {e}"))?;
        inv.handle = h.clone();
        inv.save_to(&inv_path)
            .map_err(|e| anyhow!("save inventory: {e}"))?;
        writeln!(ctx.stderr, "handle saved: {h}")?;
    }
    writeln!(
        ctx.stderr,
        "next: run `atsisbroken sync-github` to fetch your repos."
    )?;
    Ok(())
}

async fn cmd_sync_github<R: Read, W: Write, E: Write>(
    handle: Option<String>,
    ctx: &mut CliCtx<R, W, E>,
) -> Result<()> {
    use crate::github;

    paths::ensure_dir_with_home(&ctx.home).map_err(|e| anyhow!("ensure dir: {e}"))?;

    let inv_path = paths::github_inventory_path_with_home(&ctx.home);
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

    let token_path = paths::github_token_path_with_home(&ctx.home);
    let token = github::load_github_token(&token_path).map_err(|e| anyhow!("read token: {e}"))?;
    if token.is_none() {
        writeln!(
            ctx.stderr,
            "no token at {} — using anonymous mode (rate-limited 60 req/h).",
            token_path.display()
        )?;
    }

    writeln!(ctx.stderr, "syncing GitHub for {handle}…")?;
    let inv = github::sync_user_inventory(&handle, token.as_deref())
        .await
        .map_err(|e| anyhow!("sync: {e}"))?;
    inv.save_to(&inv_path)
        .map_err(|e| anyhow!("save inventory: {e}"))?;

    writeln!(
        ctx.stderr,
        "synced {} public repos ({} top-N deep-fetched) to {}",
        inv.public_repos.len(),
        inv.public_repos
            .iter()
            .filter(|r| !r.readme_excerpts.is_empty() || !r.recent_commit_messages.is_empty())
            .count(),
        inv_path.display()
    )?;
    if inv.public_repos.is_empty() {
        writeln!(
            ctx.stderr,
            "(handle has no public repos, or the user is private.)"
        )?;
    }
    Ok(())
}
