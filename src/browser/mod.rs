// SPDX-License-Identifier: Unlicense
// Unlicense — public domain — cochranblock.org
// Contributors: GotEmCoach, KOVA, Claude Opus 4.7

//! atsisbroken — the browser shell.
//!
//! This is the product. Single Rust binary, Servo-derived engine
//! eventually, no Chrome dependency, no extension. The user opens
//! atsisbroken, navigates to a job posting, and watches the form
//! get filled with per-character human-like timing and verbatim-
//! source attribution for every freetext slot.
//!
//! ## Architecture
//!
//! ```text
//! ┌─ browser shell (this module) ────────────────────────────┐
//! │  window + event loop          (winit)                    │
//! │  render surface               (wgpu)                     │
//! │  navigation + chrome UI       (this module)              │
//! │  in-process automation API    (no CDP — direct calls)    │
//! │  per-character input layer    (Gaussian-jittered timing) │
//! │  anti-fingerprint controls    (UA / canvas / etc.)       │
//! ├─ engine (HTML/CSS/JS) ───────────────────────────────────┤
//! │  html5ever                  HTML parsing       (now)     │
//! │  markup5ever_rcdom          DOM representation (now)     │
//! │  stylo                      CSS engine         (next)    │
//! │  WebRender                  paint              (next)    │
//! │  SpiderMonkey via mozjs     JS execution       (next)    │
//! ├─ brain (existing crate work) ────────────────────────────┤
//! │  classifier                 predict_field_key            │
//! │  Profile + sub-structs      schema (Phase G)             │
//! │  GitHub source              github::sync_user_inventory  │
//! │  Blog source                (Phase I.5 — coming)         │
//! │  question classifier        (Phase J — coming)           │
//! │  answer composer            (Phase K — coming)           │
//! │  correction overlay         learning::CorrectionOverlay  │
//! └──────────────────────────────────────────────────────────┘
//! ```
//!
//! The browser shell calls into the brain on every navigation +
//! field encounter. The brain doesn't know about the browser; it
//! takes [`crate::FieldDescriptor`]s and a [`crate::Profile`] and
//! returns decisions. The browser turns those decisions into
//! per-character keystrokes dispatched through the engine.

#![cfg(feature = "gui")]

mod connector;
mod engine;
mod fingerprint;
mod input;
mod internal;
mod products;
mod shell;
mod text;
mod url;
mod window;

pub use engine::{Engine, NavigateOutcome, PageSnapshot};
pub use fingerprint::FingerprintProfile;
pub use input::{HumanInputProfile, KeyTimingDistribution};
pub use shell::{run, BrowserConfig, BrowserError};
pub use url::Url;
pub use window::WindowState;

/// Public entry point. Construct a [`BrowserConfig`] and call
/// [`run`] from `main`. The function blocks until the user closes
/// the window. All long-running work — engine ticks, network
/// fetches, automation flows — happens inside the event loop.
pub fn launch(config: BrowserConfig) -> Result<(), BrowserError> {
    shell::run(config)
}
