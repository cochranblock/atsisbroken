// SPDX-License-Identifier: Unlicense
// Unlicense — public domain — cochranblock.org

//! atsisbroken — production binary entry point.
//!
//! Three responsibilities, intentionally minimal:
//!   1. Parse argv via clap (`atsisbroken::cli::Cli`).
//!   2. Build a [`atsisbroken::cli::CliCtx`] over the real
//!      `$HOME` and the real stdio.
//!   3. Hand both to `atsisbroken::cli::run`.
//!
//! Every command's logic — including the println!/eprintln! → write
//! conversions and HOME path resolution — lives in `crate::cli` so
//! the test binary can drive the same dispatch in-process with
//! `Vec<u8>` writers + a tempdir home, no subprocess required.
//!
//! This is the cochranblock pattern: the production binary IS a thin
//! shell over the lib, which both binaries share.

use atsisbroken::cli::{run, Cli, CliCtx};
use clap::Parser;
use std::io::{stderr, stdin, stdout};
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();

    // $HOME / %USERPROFILE% with a "." fallback. Same logic as
    // paths::home_dir uses internally; CliCtx takes the resolved
    // PathBuf so cli code never reads env vars directly.
    let home: PathBuf = std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."));

    let stdin_handle = stdin();
    let stdout_handle = stdout();
    let stderr_handle = stderr();

    let mut ctx = CliCtx::new(
        home,
        stdin_handle.lock(),
        stdout_handle.lock(),
        stderr_handle.lock(),
    );

    match run(cli, &mut ctx) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            // Cmd-level errors print to stderr; the binary then
            // exits non-zero so callers can detect failure via $?.
            let _ = std::io::Write::write_all(
                &mut std::io::stderr(),
                format!("error: {e}\n").as_bytes(),
            );
            ExitCode::FAILURE
        }
    }
}
