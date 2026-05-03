// SPDX-License-Identifier: Unlicense
// Unlicense — public domain — cochranblock.org

use anyhow::{anyhow, Context, Result};
use atsisbroken::{
    bridge::serve_native_messaging, paths, resume, seed_corpus_fingerprint, version,
    FeedbackQueue, Mode, Profile,
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
    /// Attach to a running Chromium and fill forms.
    Run {
        #[arg(long, value_parser = parse_mode)]
        mode: Option<Mode>,
        #[arg(long)]
        cdp: Option<String>,
    },
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
        Cmd::Run { mode, cdp } => cmd_run(mode, cdp).await,
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

async fn cmd_run(_mode: Option<Mode>, _cdp: Option<String>) -> Result<()> {
    eprintln!(
        "atsisbroken {} — run: CDP attach + fill loop (lands in Phase 1 #4)",
        version()
    );
    Ok(())
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
