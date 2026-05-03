// SPDX-License-Identifier: Unlicense
// Unlicense — public domain — cochranblock.org
//
// atsisbroken — single-binary CLI. Subcommands:
//   init      — paste resume, build Profile + seed the personal classifier
//   run       — attach to Chromium via CDP and start filling forms
//   graduate  — flip Mode::TrainingWheels → Mode::Chaos. No more prompts.
//   status    — show profile path, model path, mode, seed corpus fingerprint
//
// The CDP loop and the trainer are stubbed for this scaffold pass. Schema,
// modes, feedback type, and the seed corpus are real and tested.

use anyhow::Result;
use atsisbroken::{seed_corpus_fingerprint, version, Mode};
use clap::{Parser, Subcommand};

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
        /// Path to a plain-text resume.
        #[arg(long)]
        resume: Option<String>,
    },
    /// Attach to a running Chromium and fill forms.
    Run {
        /// Override the autonomy mode for this session.
        #[arg(long, value_parser = parse_mode)]
        mode: Option<Mode>,
        /// CDP endpoint of an already-running Chromium (e.g. `ws://localhost:9222/...`).
        #[arg(long)]
        cdp: Option<String>,
    },
    /// Take off the training wheels. Subsequent `run`s fill autonomously.
    Graduate,
    /// Print the current configuration.
    Status,
}

fn parse_mode(s: &str) -> Result<Mode, String> {
    match s {
        "training-wheels" | "training_wheels" | "training" => Ok(Mode::TrainingWheels),
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
        Cmd::Status => cmd_status().await,
    }
}

async fn cmd_init(_resume: Option<String>) -> Result<()> {
    eprintln!(
        "atsisbroken {} — init: parsing resume + seeding classifier (not yet implemented)",
        version()
    );
    Ok(())
}

async fn cmd_run(_mode: Option<Mode>, _cdp: Option<String>) -> Result<()> {
    eprintln!(
        "atsisbroken {} — run: CDP attach + fill loop (not yet implemented)",
        version()
    );
    Ok(())
}

async fn cmd_graduate() -> Result<()> {
    eprintln!("atsisbroken {} — graduate: flipping mode to Chaos (not yet implemented)", version());
    Ok(())
}

async fn cmd_status() -> Result<()> {
    println!("atsisbroken {}", version());
    println!("seed corpus fingerprint: {:08x}", seed_corpus_fingerprint());
    println!("default mode: {:?}", Mode::default());
    Ok(())
}
