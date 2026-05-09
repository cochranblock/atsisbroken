// SPDX-License-Identifier: Unlicense

//! Browser shell — the entry point.
//!
//! Owns the winit event loop, the window, the engine, and the
//! automation flow. Public surface is `BrowserConfig` + `run()`.
//! Everything else is private.

use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::WindowId;

use super::engine::{Engine, EngineError, StaticHtmlEngine};
use super::fingerprint::{FingerprintOverrides, FingerprintProfile};
use super::input::{HumanInputProfile, InputTiming};
use super::url::Url;
use super::window::WindowState;

#[derive(Debug, Clone)]
pub struct BrowserConfig {
    /// Initial URL on launch. Defaults to `atsisbroken://home`.
    pub start_url: Url,
    /// Window title.
    pub title: String,
    /// Anti-fingerprint defaults.
    pub fingerprint: FingerprintProfile,
    /// Per-domain fingerprint overrides.
    pub fingerprint_overrides: FingerprintOverrides,
    /// Human-input profile (typing speed, jitter, dwell).
    pub input: HumanInputProfile,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            start_url: Url::home(),
            title: format!("atsisbroken {}", env!("CARGO_PKG_VERSION")),
            fingerprint: FingerprintProfile::balanced(),
            fingerprint_overrides: FingerprintOverrides::default(),
            input: HumanInputProfile::default(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BrowserError {
    #[error("event loop: {0}")]
    EventLoop(String),
    #[error("engine: {0}")]
    Engine(#[from] EngineError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("anyhow: {0}")]
    Other(#[from] anyhow::Error),
}

/// Run the browser. Blocks until the user closes the window.
pub fn run(config: BrowserConfig) -> Result<(), BrowserError> {
    let event_loop = EventLoop::new()
        .map_err(|e| BrowserError::EventLoop(format!("create: {e}")))?;
    let mut app = App::new(config);
    event_loop
        .run_app(&mut app)
        .map_err(|e| BrowserError::EventLoop(format!("run: {e}")))?;
    if let Some(err) = app.startup_error.take() {
        return Err(err);
    }
    Ok(())
}

/// winit's ApplicationHandler implementor. Owns the window state
/// and the engine.
struct App {
    config: BrowserConfig,
    state: Option<WindowState>,
    engine: Box<dyn Engine>,
    timing: InputTiming,
    /// Captured at startup if window/engine init fails. The
    /// event-loop API is fire-and-forget; we need a side-channel
    /// to surface startup failure to the caller of `run()`.
    startup_error: Option<BrowserError>,
}

impl App {
    fn new(config: BrowserConfig) -> Self {
        let timing = InputTiming::new(config.input.clone());
        // Default engine is the static-HTML one. When stylo +
        // mozjs are integrated, this becomes a feature-gated
        // selection. The shell doesn't care which Engine impl
        // it talks to as long as the trait holds.
        let engine: Box<dyn Engine> = match StaticHtmlEngine::new() {
            Ok(e) => Box::new(e),
            Err(_) => Box::new(NullEngine),
        };
        Self {
            config,
            state: None,
            engine,
            timing,
            startup_error: None,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            // winit fires `resumed` again on platforms that
            // suspend (Android). We've already initialized; no-op.
            return;
        }
        match WindowState::new(event_loop, &self.config.title) {
            Ok(mut state) => {
                // Eager navigate: take the configured start URL
                // immediately so the first paint shows something.
                let url_str = self.config.start_url.to_string();
                let (title, body) = match self.engine.navigate(&self.config.start_url) {
                    Ok(_) => match self.engine.snapshot_fields() {
                        Ok(snap) => (
                            if snap.title.is_empty() {
                                self.config.start_url.host.clone()
                            } else {
                                snap.title
                            },
                            describe_page(&self.config.start_url, snap.fields.len()),
                        ),
                        Err(_) => (
                            self.config.start_url.host.clone(),
                            describe_page(&self.config.start_url, 0),
                        ),
                    },
                    Err(e) => (
                        "atsisbroken".to_string(),
                        format!(
                            "Couldn't load {url_str}\n\n{e}\n\nThe rendering engine here is the \
                             static-HTML scaffold; full HTML/CSS/JS rendering lights up when \
                             stylo + WebRender + mozjs land in subsequent commits."
                        ),
                    ),
                };
                state.set_page(url_str, title, body);
                self.state = Some(state);
            }
            Err(e) => {
                self.startup_error = Some(BrowserError::Other(e));
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let state = match self.state.as_mut() {
            Some(s) => s,
            None => return,
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                state.resize(size);
                state.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                if let Err(e) = state.render() {
                    eprintln!("render: {e}");
                }
            }
            _ => {}
        }
    }
}

/// Describe a freshly-navigated page in human-readable text.
/// Used as the body when the engine succeeded but didn't produce
/// rich content yet (most cases today since the engine is just
/// the HTML parser; CSS + paint + JS come later).
fn describe_page(url: &Url, field_count: usize) -> String {
    if url.is_internal() {
        match url.host.as_str() {
            "home" => "atsisbroken: the browser for filling job applications.\n\n\
                      Connect what you've made. We'll write your applications from it.\n\n\
                      \tatsisbroken://connections    your sources\n\
                      \tatsisbroken://summary        what we say about you\n\
                      \tatsisbroken://discover       jobs from public boards\n\
                      \tatsisbroken://applications   your ledger\n\n\
                      Type any URL above to navigate."
                .to_string(),
            "connections" => "Your connections.\n\n\
                              No sources connected yet. Click + Add a source to begin.\n\
                              (Connection tile UI lands in the next commit; \
                              today this page is a placeholder.)"
                .to_string(),
            "summary" => "Your auto-summary.\n\n\
                          Connect a source first. We need products to summarize from."
                .to_string(),
            other => format!("atsisbroken://{other} — page not yet implemented."),
        }
    } else if field_count > 0 {
        format!(
            "Loaded {url}.\n\n\
             {field_count} form field(s) detected. \
             The form-fill flow lights up when stylo + mozjs land — today the \
             engine identifies fields but can't yet drive them through the \
             new rendering stack."
        )
    } else {
        format!("Loaded {url}.\n\nNo form fields detected on this page.")
    }
}

/// Inert engine used when the real engine fails to construct.
/// The shell stays usable (window opens, can be closed); the
/// page just stays empty. Surfaces the failure via stderr but
/// doesn't crash the binary.
struct NullEngine;

impl Engine for NullEngine {
    fn navigate(&mut self, _url: &Url) -> Result<super::engine::NavigateOutcome, EngineError> {
        Err(EngineError::Unimplemented("null engine"))
    }
    fn snapshot_fields(&self) -> Result<super::engine::PageSnapshot, EngineError> {
        Err(EngineError::Unimplemented("null engine"))
    }
    fn fill_field(
        &mut self,
        _field_id: &str,
        _value: &str,
        _timing: &mut InputTiming,
    ) -> Result<(), EngineError> {
        Err(EngineError::Unimplemented("null engine"))
    }
    fn read_field(&self, _field_id: &str) -> Result<Option<String>, EngineError> {
        Err(EngineError::Unimplemented("null engine"))
    }
    fn screenshot(&self) -> Result<Vec<u8>, EngineError> {
        Err(EngineError::Unimplemented("null engine"))
    }
}

// Suppress unused warning for the Arc import on builds where
// no platform window backend is active.
#[allow(dead_code)]
fn _arc_window_marker() -> Option<Arc<()>> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_internal_home_start() {
        let c = BrowserConfig::default();
        assert!(c.start_url.is_internal());
        assert_eq!(c.start_url.host, "home");
    }

    #[test]
    fn default_config_title_includes_crate_version() {
        let c = BrowserConfig::default();
        assert!(c.title.contains("atsisbroken"));
        assert!(c.title.contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn default_config_balanced_fingerprint() {
        let c = BrowserConfig::default();
        // Hard contract: default never advertises automation.
        assert!(!c.fingerprint.navigator_webdriver);
    }
}
