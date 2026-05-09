// SPDX-License-Identifier: Unlicense

//! Browser shell — the entry point.
//!
//! Owns the winit event loop, the window, the engine, and the
//! automation flow. Public surface is `BrowserConfig` + `run()`.
//! Everything else is private.

use std::sync::Arc;
use std::thread;

use glyphon::FontSystem;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::window::WindowId;

use super::engine::{Engine, EngineError, StaticHtmlEngine};
use super::fingerprint::{FingerprintOverrides, FingerprintProfile};
use super::input::{HumanInputProfile, InputTiming};
use super::url::Url;
use super::window::WindowState;

/// Worker → event-loop message. The winit event loop is the only
/// place the WindowState lives, so off-thread navigations come
/// back through here. winit's user-event channel guarantees
/// in-order delivery on the event-loop thread.
#[derive(Debug, Clone)]
pub enum NavMsg {
    /// Network navigation completed successfully. Caller (the
    /// user_event handler) calls `set_page` with the result.
    Loaded {
        url: Url,
        title: String,
        body: String,
    },
    /// Network navigation failed. The error is rendered into the
    /// page body so the user sees it without needing a terminal.
    Error {
        url: Url,
        error: String,
    },
}

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
///
/// Engine construction failure surfaces here, before the event
/// loop runs. Previously a failed `StaticHtmlEngine::new()` was
/// silently swallowed via a `NullEngine` fallback (Rust audit bug
/// #5); the user got an inert window with no diagnostic. Now the
/// `?` propagates `EngineError` through `BrowserError` so the
/// caller sees what's wrong.
pub fn run(config: BrowserConfig) -> Result<(), BrowserError> {
    // EventLoop carries a NavMsg user-event channel so worker
    // threads can post navigation results back to the event-loop
    // thread without taking a lock on WindowState. winit's
    // EventLoopProxy is the only sanctioned way to wake the
    // loop from another thread. (Audit bug #11.)
    let event_loop = EventLoop::<NavMsg>::with_user_event()
        .build()
        .map_err(|e| BrowserError::EventLoop(format!("create: {e}")))?;
    let proxy = event_loop.create_proxy();
    let mut app = App::new(config, proxy)?;
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
    /// Engine used for SYNCHRONOUS navigations only — internal
    /// `atsisbroken://` pages, which are pure in-process renders
    /// with no network I/O. Network navigations happen on a
    /// worker thread that constructs its own engine; results
    /// flow back via NavMsg. The engine itself is `!Send`
    /// (RcDom uses Rc), so it can't be moved across threads.
    engine: Box<dyn Engine>,
    #[allow(dead_code)] // wired in when input layer lands
    timing: InputTiming,
    /// Pre-built FontSystem. Built in App::new (which runs before
    /// the event loop starts), then moved into WindowState on
    /// `resumed`. Building inside `resumed` would freeze winit's
    /// event handler for ~100-300ms on machines with many fonts
    /// (Rust audit bug #6). Option so we can `take()` the value
    /// when handing it off; should never be None after the first
    /// resumed runs.
    font_system: Option<FontSystem>,
    /// Captured if window init fails inside `resumed`. winit's
    /// ApplicationHandler is fire-and-forget; we need a side-
    /// channel to surface mid-event-loop errors to `run()`.
    startup_error: Option<BrowserError>,
    /// Proxy to the event loop. Worker threads use it to post
    /// NavMsg results back; the event-loop thread receives them
    /// via `user_event`. (Audit bug #11.)
    proxy: EventLoopProxy<NavMsg>,
}

impl App {
    fn new(
        config: BrowserConfig,
        proxy: EventLoopProxy<NavMsg>,
    ) -> Result<Self, BrowserError> {
        let timing = InputTiming::new(config.input.clone());
        let engine: Box<dyn Engine> = Box::new(StaticHtmlEngine::new()?);
        // Build FontSystem here, BEFORE the event loop runs.
        // FontSystem::new() invokes fontdb::Database::load_system_fonts
        // which scans /usr/share/fonts (Linux), /System/Library/Fonts
        // (macOS), or %WINDIR%/Fonts (Windows). On dev machines with
        // many fonts this takes ~100-300ms. Doing it here means the
        // user's window opens INSTANTLY when resumed fires; doing it
        // inside resumed would freeze the OS event loop during the
        // scan.
        let font_system = FontSystem::new();
        Ok(Self {
            config,
            state: None,
            engine,
            timing,
            font_system: Some(font_system),
            startup_error: None,
            proxy,
        })
    }
}

/// Compose `(title, body)` for the page surface from a navigate
/// + snapshot result pair. Pulled out of the `resumed` closure so
/// the worker thread can call it from off-thread without
/// duplicating the formatting logic.
fn compose_page_text(
    url: &Url,
    nav: Result<(), EngineError>,
    snap: Result<super::engine::PageSnapshot, EngineError>,
) -> (String, String) {
    let url_str = url.to_string();
    match nav {
        Ok(()) => match snap {
            Ok(s) => (
                if s.title.is_empty() { url.host().to_string() } else { s.title },
                s.body,
            ),
            Err(_) => (url.host().to_string(), String::new()),
        },
        Err(e) => (
            "atsisbroken".to_string(),
            format!(
                "Couldn't load {url_str}\n\n{e}\n\nThe rendering engine here is the \
                 static-HTML scaffold; full HTML/CSS/JS rendering lights up when \
                 stylo + WebRender + mozjs land in subsequent commits."
            ),
        ),
    }
}

/// Spawn a worker thread that fetches `url` via a fresh
/// `StaticHtmlEngine` (the existing one is `!Send`) and posts
/// the composed result back through `proxy`. Errors are
/// rendered into the page body the user sees, not just logged.
fn spawn_navigate(url: Url, proxy: EventLoopProxy<NavMsg>) {
    thread::spawn(move || {
        let mut engine = match StaticHtmlEngine::new() {
            Ok(e) => e,
            Err(e) => {
                let _ = proxy.send_event(NavMsg::Error {
                    url,
                    error: format!("engine init: {e}"),
                });
                return;
            }
        };
        let msg = match engine.navigate(&url) {
            Ok(_) => match engine.snapshot_fields() {
                Ok(snap) => {
                    let title = if snap.title.is_empty() {
                        url.host().to_string()
                    } else {
                        snap.title
                    };
                    NavMsg::Loaded {
                        url,
                        title,
                        body: snap.body,
                    }
                }
                Err(e) => NavMsg::Error {
                    url,
                    error: format!("snapshot: {e}"),
                },
            },
            Err(e) => NavMsg::Error {
                url,
                error: e.to_string(),
            },
        };
        // EventLoopProxy::send_event errors only when the event
        // loop has already exited (user closed the window during
        // the fetch). In that case the result is irrelevant;
        // drop it.
        let _ = proxy.send_event(msg);
    });
}

impl ApplicationHandler<NavMsg> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            // winit fires `resumed` again on platforms that
            // suspend (Android). We've already initialized; no-op.
            return;
        }
        // Move the pre-built FontSystem into WindowState. Should
        // always be Some here — App::new builds it before the
        // event loop starts. None would mean a previous resumed
        // already consumed it; that path is guarded by the
        // is_some() check above.
        let Some(font_system) = self.font_system.take() else {
            self.startup_error = Some(BrowserError::Other(anyhow::anyhow!(
                "font_system already consumed — duplicate resumed event?"
            )));
            event_loop.exit();
            return;
        };
        match WindowState::new(event_loop, &self.config.title, font_system) {
            Ok(mut state) => {
                let url = self.config.start_url.clone();
                let url_str = url.to_string();
                if url.is_internal() {
                    // Internal pages render in-process. Pure
                    // function call, no network — safe to do
                    // synchronously inside the event handler.
                    let nav = self.engine.navigate(&url).map(|_| ());
                    let snap = self.engine.snapshot_fields();
                    let (title, body) = compose_page_text(&url, nav, snap);
                    state.set_page(url_str, title, body);
                } else {
                    // Network pages: punt the fetch onto a worker
                    // thread so the OS event loop is never blocked
                    // on reqwest::blocking. The placeholder body
                    // shows immediately; user_event() replaces it
                    // when the worker returns. (Audit bug #11.)
                    state.set_page(
                        url_str.clone(),
                        url.host().to_string(),
                        format!("Loading {url_str}…"),
                    );
                    spawn_navigate(url, self.proxy.clone());
                }
                self.state = Some(state);
            }
            Err(e) => {
                self.startup_error = Some(BrowserError::Other(e));
                event_loop.exit();
            }
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: NavMsg) {
        // Worker-thread navigations report back here. If the
        // window was closed in flight (state already None) the
        // result is irrelevant.
        let Some(state) = self.state.as_mut() else { return };
        match event {
            NavMsg::Loaded { url, title, body } => {
                state.set_page(url.to_string(), title, body);
            }
            NavMsg::Error { url, error } => {
                let url_str = url.to_string();
                state.set_page(
                    url_str.clone(),
                    "atsisbroken".to_string(),
                    format!(
                        "Couldn't load {url_str}\n\n{error}\n\nThe rendering engine \
                         here is the static-HTML scaffold; full HTML/CSS/JS rendering \
                         lights up when stylo + WebRender + mozjs land in subsequent \
                         commits."
                    ),
                );
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
                // Dedup via state.render_log so a stuck
                // SurfaceError (`Outdated` mid-resize, `Lost` on
                // a GPU disconnect) doesn't spam stderr every
                // vsync. WindowState's RenderLog handles the
                // text-layer sites; this slot covers the outer
                // surface present.
                match state.render() {
                    Ok(()) => state.render_log.frame_ok(),
                    Err(e) => state.render_log.frame_err(e),
                }
            }
            _ => {}
        }
    }
}

// NullEngine retired. It existed solely to swallow
// `StaticHtmlEngine::new()` failure so the window could still
// open with an inert engine. Per Rust audit bug #5: that's silent
// failure. Now the constructor failure surfaces through
// `App::new` → `run` → caller, with the actual EngineError text
// preserved via thiserror's `#[from]`.
//
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
    fn default_config_starts_on_network_landing_page() {
        // The default home is the live landing page. Network
        // navigation goes through the audit-fix-#11 worker
        // thread, so the OS event loop never blocks on the
        // initial fetch.
        let c = BrowserConfig::default();
        assert!(c.start_url.is_network());
        assert_eq!(c.start_url.host(), "atsisbroken.cochranblock.org");
    }

    #[test]
    fn default_config_title_includes_crate_version() {
        let c = BrowserConfig::default();
        assert!(c.title.contains("atsisbroken"));
        assert!(c.title.contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn default_config_pins_webdriver_field_false() {
        // This pins the struct default — the FingerprintProfile
        // we hand the rest of the system says webdriver=false.
        // It is NOT a runtime guarantee against a live page; the
        // JS engine that would enforce it (mozjs) hasn't landed.
        // The pin still has value: when the JS layer arrives, this
        // is the value it will read, and any drift in the default
        // surfaces here.
        let c = BrowserConfig::default();
        assert!(!c.fingerprint.navigator_webdriver);
    }

    // ─── compose_page_text ───────────────────────────────────────────────

    use super::compose_page_text;
    use super::super::engine::PageSnapshot;

    fn url_for_test(s: &str) -> Url {
        s.parse().expect("test URL must parse")
    }

    #[test]
    fn compose_page_text_uses_snapshot_title_when_present() {
        let url = url_for_test("https://example.com/");
        let snap = PageSnapshot {
            url: url.to_string(),
            title: "Example Domain".into(),
            body: "This domain is for use in illustrative examples.".into(),
            fields: Vec::new(),
        };
        let (title, body) = compose_page_text(&url, Ok(()), Ok(snap));
        assert_eq!(title, "Example Domain");
        assert!(body.contains("illustrative"));
    }

    #[test]
    fn compose_page_text_falls_back_to_host_when_title_empty() {
        // Network pages without a <title> tag use the URL host as
        // the title — matches the address-bar idiom of "you're at
        // example.com" when the page hasn't named itself.
        let url = url_for_test("https://example.com/");
        let snap = PageSnapshot {
            url: url.to_string(),
            title: String::new(),
            body: "body text".into(),
            fields: Vec::new(),
        };
        let (title, body) = compose_page_text(&url, Ok(()), Ok(snap));
        assert_eq!(title, "example.com");
        assert_eq!(body, "body text");
    }

    #[test]
    fn compose_page_text_renders_navigate_error_into_body() {
        // Failed navigation renders the error inline so the user
        // sees it in the window — not just on stderr.
        let url = url_for_test("https://example.com/");
        let nav_err = Err(EngineError::Network("dns: no route to host".into()));
        let snap_err = Err(EngineError::Parse("no page loaded".into()));
        let (title, body) = compose_page_text(&url, nav_err, snap_err);
        assert_eq!(title, "atsisbroken");
        assert!(body.contains("Couldn't load"));
        assert!(body.contains("https://example.com/"));
        assert!(body.contains("dns: no route to host"));
    }

    // ─── Url routing dispatch ────────────────────────────────────────────

    #[test]
    fn internal_url_takes_synchronous_path() {
        // The dispatch in `resumed` keys off `is_internal()`. Pin
        // the predicate so a URL refactor doesn't silently route
        // an internal page through the network worker (which
        // would still work, but would be wasteful + confusing).
        let url = url_for_test("atsisbroken://home");
        assert!(url.is_internal());
        assert!(!url.is_network());
    }

    #[test]
    fn network_url_takes_async_worker_path() {
        let url = url_for_test("https://boards.greenhouse.io/example/jobs/123");
        assert!(!url.is_internal());
        assert!(url.is_network());
    }
}
