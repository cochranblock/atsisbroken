// SPDX-License-Identifier: Unlicense
// Unlicense — public domain — cochranblock.org
// Contributors: GotEmCoach, KOVA, Claude Opus 4.7

//! atsisbroken — the browser shell.
//!
//! This is the product. Single Rust binary, Servo-derived engine
//! (in flight), no Chrome dependency, no extension. Design intent:
//! the user opens atsisbroken, navigates to a job posting, and
//! watches the form get filled with per-character human-like
//! timing, while every freetext token is anchored to a public URL
//! the user already wrote. The per-character input layer, the
//! composer, and the audit-trail wiring land in subsequent phases;
//! today's binary ships the schema, the page router, the wgpu +
//! glyphon render pipeline, and the network plumbing for connectors.
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
// pub(crate) for the modules the tests tree (src/tests/) reaches
// into directly. External callers still use the re-exports below;
// pub(crate) just makes the module *path* visible inside the crate.
pub(crate) mod fingerprint;
pub(crate) mod input;
pub(crate) mod internal;
pub(crate) mod products;
pub(crate) mod shell;
pub(crate) mod text;
mod url;
pub(crate) mod window;

pub use connector::{AuthShape, ConnectorKind};
pub use engine::{Engine, EngineError, NavigateOutcome, PageSnapshot, StaticHtmlEngine};
pub use fingerprint::FingerprintProfile;
pub use input::{HumanInputProfile, KeyTimingDistribution};
pub use shell::{run, BrowserConfig, BrowserError};
pub use url::Url;
pub use window::WindowState;

/// Headless inspect — render the page for `url` without opening
/// a window. Returns a [`PageSnapshot`] (URL, title, body, fields).
/// Used by the `atsisbroken inspect` CLI subcommand and by
/// integration tests that need to verify page output without GUI
/// dependencies.
///
/// For internal `atsisbroken://` URLs we route through the
/// in-process page renderer directly — no HTTP client, no tokio
/// blocking-runtime drop, no engine lifecycle. For network URLs
/// the full StaticHtmlEngine is constructed.
pub fn inspect(url: &Url) -> Result<PageSnapshot, EngineError> {
    if url.is_internal() {
        let page = internal::render(url);
        return Ok(PageSnapshot {
            url: url.to_string(),
            title: page.title,
            body: page.body,
            fields: Vec::new(),
        });
    }
    let mut engine = StaticHtmlEngine::new()?;
    engine.navigate(url)?;
    engine.snapshot_fields()
}

/// Public entry point. Construct a [`BrowserConfig`] and call
/// [`run`] from `main`. The function blocks until the user closes
/// the window. All long-running work — engine ticks, network
/// fetches, automation flows — happens inside the event loop.
pub fn launch(config: BrowserConfig) -> Result<(), BrowserError> {
    shell::run(config)
}
