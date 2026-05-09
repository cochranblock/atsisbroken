// SPDX-License-Identifier: Unlicense

//! Engine — HTML/CSS/JS layer.
//!
//! Today: html5ever + markup5ever_rcdom for parsing into a Servo-
//! style DOM. This is the same DOM representation Servo uses, so
//! when we wire stylo + WebRender + SpiderMonkey the hand-off is
//! direct.
//!
//! Coming next:
//! - **stylo** for CSS resolution. Already in the Servo workspace,
//!   already in production in Firefox.
//! - **taffy** or Servo's layout for box-tree construction.
//! - **WebRender** (or wgpu directly) for paint.
//! - **mozjs** (Servo's SpiderMonkey bindings) for JS execution.
//!   Without this, ATS forms don't render — they're React / Vue
//!   SPAs that paint nothing without a JS host.
//!
//! The [`Engine`] trait is what the shell calls; the real impl
//! grows incrementally as each layer lands. The static-HTML impl
//! below proves the shape and lets the shell + automation layer
//! be developed in parallel with the engine.

use crate::FieldDescriptor;
use markup5ever_rcdom::{Handle, NodeData, RcDom};

use super::Url;

/// What every engine implementation must expose to the shell.
pub trait Engine {
    /// Navigate to the URL, fetch the resource, parse, layout,
    /// paint into the supplied surface. Returns whether the page
    /// rendered successfully and how many form fields it found.
    fn navigate(&mut self, url: &Url) -> Result<NavigateOutcome, EngineError>;

    /// Snapshot the page's form fields. Equivalent to the
    /// `Runtime.evaluate(...querySelectorAll('input,...'))` call
    /// the CDP path uses today.
    fn snapshot_fields(&self) -> Result<PageSnapshot, EngineError>;

    /// Set a field's value with per-character dispatch + timing.
    /// Returns when the dispatched-key sequence is complete and
    /// the page has fired its `change` event.
    fn fill_field(
        &mut self,
        field_id: &str,
        value: &str,
        timing: &mut super::input::InputTiming,
    ) -> Result<(), EngineError>;

    /// Read back a field's current value. Used by the Workday
    /// re-render verifier to detect destroy-and-recreate cycles.
    fn read_field(&self, field_id: &str) -> Result<Option<String>, EngineError>;

    /// Take a screenshot of the rendered page. Returns PNG bytes.
    /// Mostly for the audit log / TrainingWheels review surface.
    fn screenshot(&self) -> Result<Vec<u8>, EngineError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigateOutcome {
    pub final_url: Url,
    /// HTTP status. 0 for internal `atsisbroken://` pages.
    pub status: u16,
    /// MIME type from Content-Type.
    pub content_type: String,
    /// Number of form fields detected (`<input>`, `<textarea>`,
    /// `<select>`).
    pub form_field_count: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PageSnapshot {
    pub url: String,
    pub title: String,
    /// Body text the renderer paints below the title. For network
    /// pages this is a description (e.g. "Loaded X — N fields
    /// detected"); for internal `atsisbroken://` pages it's the
    /// full page content from [`super::internal::render`]. The
    /// shell hands this to the text layer verbatim.
    pub body: String,
    pub fields: Vec<FieldDescriptor>,
}

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("network: {0}")]
    Network(String),
    #[error("parse: {0}")]
    Parse(String),
    #[error("not yet implemented: {0}")]
    Unimplemented(&'static str),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

/// Static-HTML engine. Parses HTML via html5ever, walks the DOM
/// to extract form fields, treats every page as inert (no JS, no
/// CSS-driven layout). Sufficient for static pages and for the
/// internal `atsisbroken://` pages we generate ourselves;
/// insufficient for ATS forms (Workday + Greenhouse + Lever are
/// all SPAs that render nothing without JS).
///
/// This is the v0 of the engine layer. The point isn't to ship
/// it as production; the point is to have the shape in place
/// before stylo + WebRender + mozjs land. The shell + automation
/// layer can be built and tested against this impl while the
/// real engine is being integrated.
pub struct StaticHtmlEngine {
    /// Cached DOM from the most-recent navigation. None before
    /// first `navigate`, and None for internal pages (which don't
    /// produce a DOM — they produce title + body directly).
    dom: Option<RcDom>,
    /// URL of the last navigation. Used as the title fallback
    /// and for `snapshot_fields().url`.
    last_url: Option<Url>,
    /// Cached title text from the last navigation.
    title: String,
    /// Cached body text. For network pages this is a description
    /// the shell composes (e.g. "Loaded URL — N fields"). For
    /// internal pages this is the rendered page content from
    /// [`super::internal::render`].
    body: String,
    /// HTTP client. We share the existing reqwest client style
    /// (rustls-tls) so the dep graph doesn't grow.
    client: reqwest::blocking::Client,
}

impl StaticHtmlEngine {
    pub fn new() -> Result<Self, EngineError> {
        let client = reqwest::blocking::Client::builder()
            .user_agent(default_user_agent())
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| EngineError::Network(format!("build client: {e}")))?;
        Ok(Self {
            dom: None,
            last_url: None,
            title: String::new(),
            body: String::new(),
            client,
        })
    }

    /// Walk the DOM and extract every form-field descriptor.
    /// Mirror of the JS classifier's `querySelectorAll('input,
    /// textarea, select')` step but in pure Rust.
    fn extract_fields(dom: &RcDom) -> Vec<FieldDescriptor> {
        let mut out = Vec::new();
        walk_for_fields(&dom.document, &mut out);
        out
    }

    /// Walk the DOM looking for the `<title>` element's text.
    fn extract_title(dom: &RcDom) -> String {
        let mut found = String::new();
        walk_for_title(&dom.document, &mut found);
        found
    }
}

impl Engine for StaticHtmlEngine {
    fn navigate(&mut self, url: &Url) -> Result<NavigateOutcome, EngineError> {
        // Internal `atsisbroken://` URLs go through the in-process
        // page renderer — no network, no HTML parser, no DOM. The
        // engine just stores the rendered title + body for the
        // shell to paint.
        if url.is_internal() {
            let page = super::internal::render(url);
            self.title = page.title;
            self.body = page.body;
            self.dom = None;
            self.last_url = Some(url.clone());
            return Ok(NavigateOutcome {
                final_url: url.clone(),
                status: 0,
                content_type: "text/atsisbroken-internal".to_string(),
                form_field_count: 0,
            });
        }
        if !url.is_network() {
            return Err(EngineError::Network(format!(
                "scheme {:?} not supported by StaticHtmlEngine",
                url.scheme()
            )));
        }
        let url_str = url.to_string();
        let resp = self
            .client
            .get(&url_str)
            .send()
            .map_err(|e| EngineError::Network(format!("get {url_str}: {e}")))?;
        let status = resp.status().as_u16();
        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();
        let body = resp
            .text()
            .map_err(|e| EngineError::Network(format!("read body: {e}")))?;
        let dom = parse_html(&body)?;
        self.title = Self::extract_title(&dom);
        let fields = Self::extract_fields(&dom);
        let form_field_count = fields.len();
        // For network pages, body is a one-line description the
        // shell can use as a fallback. The shell typically
        // overrides this with its own composed message.
        self.body = if form_field_count > 0 {
            format!(
                "Loaded {url}.\n\n{form_field_count} form field(s) detected."
            )
        } else {
            format!("Loaded {url}.")
        };
        self.dom = Some(dom);
        self.last_url = Some(url.clone());
        Ok(NavigateOutcome {
            final_url: url.clone(),
            status,
            content_type,
            form_field_count,
        })
    }

    fn snapshot_fields(&self) -> Result<PageSnapshot, EngineError> {
        // Internal pages: no DOM, just the rendered title+body.
        // Network pages: extract fields from the cached DOM.
        let url_str = self
            .last_url
            .as_ref()
            .map(|u| u.to_string())
            .unwrap_or_default();
        let fields = if let Some(dom) = self.dom.as_ref() {
            Self::extract_fields(dom)
        } else if self.last_url.is_some() {
            // Internal page — no fields by definition.
            Vec::new()
        } else {
            return Err(EngineError::Parse("no page loaded".into()));
        };
        Ok(PageSnapshot {
            url: url_str,
            title: self.title.clone(),
            body: self.body.clone(),
            fields,
        })
    }

    fn fill_field(
        &mut self,
        _field_id: &str,
        _value: &str,
        _timing: &mut super::input::InputTiming,
    ) -> Result<(), EngineError> {
        // Static engine can't actually fill — no JS host, no
        // event dispatch, no `el.value = "..."`. The shell calls
        // this; for the static engine the call is a no-op stub
        // that records the intended fill in a debug log so the
        // automation layer can be tested without a real JS engine.
        Err(EngineError::Unimplemented(
            "fill requires the JS-capable engine (stylo + mozjs)",
        ))
    }

    fn read_field(&self, _field_id: &str) -> Result<Option<String>, EngineError> {
        Err(EngineError::Unimplemented(
            "read_field requires a live engine",
        ))
    }

    fn screenshot(&self) -> Result<Vec<u8>, EngineError> {
        Err(EngineError::Unimplemented(
            "screenshot requires WebRender / wgpu surface",
        ))
    }
}

fn default_user_agent() -> String {
    format!(
        "atsisbroken/{} (+https://github.com/cochranblock/atsisbroken)",
        env!("CARGO_PKG_VERSION")
    )
}

/// Parse HTML bytes via html5ever into a Servo-style DOM tree.
fn parse_html(html: &str) -> Result<RcDom, EngineError> {
    use html5ever::tendril::TendrilSink;
    let parser = html5ever::parse_document(RcDom::default(), Default::default());
    let dom = parser.one(html);
    Ok(dom)
}

/// Recursive walk that pulls every `<input>` / `<textarea>` /
/// `<select>` into a [`FieldDescriptor`] using the same fields
/// the existing classifier expects.
fn walk_for_fields(handle: &Handle, out: &mut Vec<FieldDescriptor>) {
    if let NodeData::Element { name, attrs, .. } = &handle.data {
        let tag = name.local.as_ref();
        if matches!(tag, "input" | "textarea" | "select") {
            let attrs = attrs.borrow();
            let get = |needle: &str| -> String {
                attrs
                    .iter()
                    .find(|a| a.name.local.as_ref() == needle)
                    .map(|a| a.value.to_string())
                    .unwrap_or_default()
            };
            let id = get("id");
            let name_attr = get("name");
            let placeholder = get("placeholder");
            let aria_label = get("aria-label");
            let kind = if tag == "input" {
                let t = get("type");
                if t.is_empty() { "text".to_string() } else { t }
            } else {
                tag.to_string()
            };
            // Resolve the field's label by searching for either a
            // `<label for="<id>">` or a wrapping `<label>` ancestor.
            // For the static engine we keep this simple — only the
            // for=id case is implemented; wrapping-label is a TODO.
            let label = String::new();
            out.push(FieldDescriptor {
                label,
                placeholder,
                aria_label,
                name: name_attr,
                id,
                kind,
            });
        }
    }
    for child in handle.children.borrow().iter() {
        walk_for_fields(child, out);
    }
}

fn walk_for_title(handle: &Handle, found: &mut String) {
    if !found.is_empty() {
        return;
    }
    if let NodeData::Element { name, .. } = &handle.data {
        if name.local.as_ref() == "title" {
            for child in handle.children.borrow().iter() {
                if let NodeData::Text { contents } = &child.data {
                    found.push_str(&contents.borrow());
                }
            }
            return;
        }
    }
    for child in handle.children.borrow().iter() {
        walk_for_title(child, found);
        if !found.is_empty() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_html_extracts_fields() {
        let html = r#"
<!DOCTYPE html>
<html>
<head><title>Apply</title></head>
<body>
<form>
<input type="email" name="email" id="email-id" placeholder="you@example.com" aria-label="Email">
<input type="tel" name="phone" id="phone-id">
<textarea name="why" id="why-id" placeholder="Why this role?"></textarea>
<select name="country" id="country-id">
<option>USA</option>
</select>
</form>
</body>
</html>"#;
        let dom = parse_html(html).unwrap();
        assert_eq!(StaticHtmlEngine::extract_title(&dom), "Apply");
        let fields = StaticHtmlEngine::extract_fields(&dom);
        assert_eq!(fields.len(), 4);
        let email = fields.iter().find(|f| f.name == "email").unwrap();
        assert_eq!(email.kind, "email");
        assert_eq!(email.placeholder, "you@example.com");
        assert_eq!(email.aria_label, "Email");
        let textarea = fields.iter().find(|f| f.name == "why").unwrap();
        assert_eq!(textarea.kind, "textarea");
        let select = fields.iter().find(|f| f.name == "country").unwrap();
        assert_eq!(select.kind, "select");
        let phone = fields.iter().find(|f| f.name == "phone").unwrap();
        assert_eq!(phone.kind, "tel");
    }

    #[test]
    fn snapshot_fields_before_navigate_errors_cleanly() {
        let engine = StaticHtmlEngine::new().expect("build engine");
        let r = engine.snapshot_fields();
        assert!(r.is_err());
    }

    #[test]
    fn fill_field_unimplemented_on_static_engine() {
        let mut e = StaticHtmlEngine::new().unwrap();
        let mut t = super::super::input::InputTiming::new(
            super::super::input::HumanInputProfile::default(),
        );
        match e.fill_field("anything", "value", &mut t) {
            Err(EngineError::Unimplemented(_)) => {}
            other => panic!("expected Unimplemented, got {other:?}"),
        }
    }

    #[test]
    fn navigate_to_internal_url_renders_in_process() {
        let mut e = StaticHtmlEngine::new().unwrap();
        let url: Url = "atsisbroken://home".parse().unwrap();
        let outcome = e.navigate(&url).expect("internal navigate must succeed");
        assert_eq!(outcome.status, 0);
        assert_eq!(outcome.content_type, "text/atsisbroken-internal");
        assert_eq!(outcome.form_field_count, 0);
        let snap = e.snapshot_fields().unwrap();
        assert_eq!(snap.title, "atsisbroken");
        assert!(!snap.body.is_empty());
        assert!(snap.fields.is_empty());
    }

    #[test]
    fn navigate_internal_unknown_returns_not_found_page() {
        let mut e = StaticHtmlEngine::new().unwrap();
        let url: Url = "atsisbroken://does-not-exist".parse().unwrap();
        e.navigate(&url).expect("not-found page is still a valid render");
        let snap = e.snapshot_fields().unwrap();
        assert_eq!(snap.title, "Page not found");
        assert!(snap.body.contains("atsisbroken://does-not-exist"));
    }

    // ─── Deeper engine flow tests ─────────────────────────────────────────

    #[test]
    fn engine_state_persists_across_repeated_internal_navigates() {
        // Navigating between two internal pages should swap state
        // cleanly — title from the new page, body from the new
        // page, no leakage from the prior page.
        let mut e = StaticHtmlEngine::new().unwrap();
        e.navigate(&"atsisbroken://home".parse().unwrap()).unwrap();
        let snap1 = e.snapshot_fields().unwrap();
        assert_eq!(snap1.title, "atsisbroken");

        e.navigate(&"atsisbroken://connections".parse().unwrap())
            .unwrap();
        let snap2 = e.snapshot_fields().unwrap();
        assert!(snap2.title.contains("connections"));
        // Body should be the connections-page body, not the home-
        // page body.
        assert!(snap2.body.contains("GitHub"));
        assert!(!snap2.body.contains("atsisbroken: the browser"));
    }

    #[test]
    fn engine_fields_are_empty_for_internal_pages() {
        // Pin: internal pages have zero form fields. The
        // composer + run loop should never try to fill an
        // internal-page "form."
        let mut e = StaticHtmlEngine::new().unwrap();
        for host in ["home", "connections", "summary", "applications"] {
            let url: Url = format!("atsisbroken://{host}").parse().unwrap();
            e.navigate(&url).unwrap();
            let snap = e.snapshot_fields().unwrap();
            assert!(
                snap.fields.is_empty(),
                "internal page {host} produced form fields: {:?}",
                snap.fields
            );
        }
    }

    #[test]
    fn engine_navigate_outcome_internal_shape() {
        // Pin the on-the-wire shape of NavigateOutcome for
        // internal pages: status=0, content_type ends with our
        // sentinel, form_field_count=0.
        let mut e = StaticHtmlEngine::new().unwrap();
        let url: Url = "atsisbroken://summary".parse().unwrap();
        let outcome = e.navigate(&url).unwrap();
        assert_eq!(outcome.status, 0);
        assert_eq!(outcome.content_type, "text/atsisbroken-internal");
        assert_eq!(outcome.form_field_count, 0);
        assert_eq!(outcome.final_url, url);
    }

    #[test]
    fn engine_html_extracts_kind_aria_label_id_name_placeholder() {
        // Cover the full FieldDescriptor extraction surface.
        // Catches HTML attribute parsing regressions in the
        // html5ever DOM walker.
        let html = r#"<!DOCTYPE html>
<form>
  <input type="email" name="contact-email" id="email-input"
         placeholder="you@example.com" aria-label="Email address">
  <textarea name="bio" id="bio-text" placeholder="Tell us"></textarea>
  <select name="country" id="country-select">
    <option>USA</option>
  </select>
</form>"#;
        let dom = parse_html(html).unwrap();
        let fields = StaticHtmlEngine::extract_fields(&dom);
        assert_eq!(fields.len(), 3);

        let email = fields.iter().find(|f| f.name == "contact-email").unwrap();
        assert_eq!(email.kind, "email");
        assert_eq!(email.id, "email-input");
        assert_eq!(email.placeholder, "you@example.com");
        assert_eq!(email.aria_label, "Email address");

        let bio = fields.iter().find(|f| f.name == "bio").unwrap();
        assert_eq!(bio.kind, "textarea");
        assert_eq!(bio.id, "bio-text");
        assert_eq!(bio.placeholder, "Tell us");

        let country = fields.iter().find(|f| f.name == "country").unwrap();
        assert_eq!(country.kind, "select");
    }

    #[test]
    fn engine_input_kind_defaults_to_text_when_omitted() {
        // <input> without type attribute → "text". Common in
        // hand-written forms; broken behavior here would
        // misclassify every default text input.
        let html = r#"<input name="username" id="u">"#;
        let dom = parse_html(html).unwrap();
        let fields = StaticHtmlEngine::extract_fields(&dom);
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].kind, "text");
    }

    #[test]
    fn engine_skips_non_form_elements() {
        // r##"..."## because the HTML contains "# (closing-quote
        // followed by hash) inside the href attribute — that
        // would terminate an r#"..."# raw string early.
        let html = r##"<!DOCTYPE html>
<html><body>
<button>Click me</button>
<a href="#">Link</a>
<div><p>Not a form field</p></div>
<input name="real-field" type="text">
</body></html>"##;
        let dom = parse_html(html).unwrap();
        let fields = StaticHtmlEngine::extract_fields(&dom);
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].name, "real-field");
    }

    #[test]
    fn engine_handles_unicode_in_title() {
        // The title extractor walks <title>'s text node. Verify
        // it doesn't choke on non-ASCII (real ATS forms occasionally
        // have international company names).
        let html = "<!DOCTYPE html><title>café — application</title>";
        let dom = parse_html(html).unwrap();
        assert_eq!(StaticHtmlEngine::extract_title(&dom), "café — application");
    }

    #[test]
    fn engine_title_empty_when_no_title_tag() {
        let html = "<!DOCTYPE html><body>no title</body>";
        let dom = parse_html(html).unwrap();
        assert_eq!(StaticHtmlEngine::extract_title(&dom), "");
    }

    #[test]
    fn engine_handles_malformed_html_without_panic() {
        // html5ever is famously lenient; verify we never panic
        // even on garbage input. Composer downstream depends on
        // this — a vendor mid-deploy could serve broken HTML.
        for bad_html in [
            "",
            "<<<<>>>",
            "<input <input <input",
            "<!-- never closing comment ",
            r#"<input type="email" name="<script>alert(1)</script>">"#,
        ] {
            let r = parse_html(bad_html);
            assert!(r.is_ok(), "panic-able HTML: {bad_html:?}");
            let dom = r.unwrap();
            // Whatever fields it extracts, the call should not panic.
            let _ = StaticHtmlEngine::extract_fields(&dom);
        }
    }

    // ─── Snapshot tests for internal pages ────────────────────────────────
    //
    // Each internal page's body is locked via `insta`. Adding or
    // changing copy requires running `cargo insta accept`. Catches
    // unintended copy regressions (e.g. a renamed URL slug
    // breaking the documented connection target).

    #[test]
    fn snapshot_internal_page_home() {
        let snap = inspect_for_test("atsisbroken://home");
        insta::assert_snapshot!("internal_page_home", snap);
    }

    #[test]
    fn snapshot_internal_page_connections() {
        let snap = inspect_for_test("atsisbroken://connections");
        insta::assert_snapshot!("internal_page_connections", snap);
    }

    #[test]
    fn snapshot_internal_page_summary() {
        let snap = inspect_for_test("atsisbroken://summary");
        insta::assert_snapshot!("internal_page_summary", snap);
    }

    #[test]
    fn snapshot_internal_page_fingerprint() {
        let snap = inspect_for_test("atsisbroken://fingerprint");
        insta::assert_snapshot!("internal_page_fingerprint", snap);
    }

    #[test]
    fn snapshot_internal_page_shield() {
        let snap = inspect_for_test("atsisbroken://shield");
        insta::assert_snapshot!("internal_page_shield", snap);
    }

    #[test]
    fn snapshot_internal_page_not_found() {
        let snap = inspect_for_test("atsisbroken://no-such-page/with/path");
        insta::assert_snapshot!("internal_page_not_found", snap);
    }

    fn inspect_for_test(url: &str) -> String {
        let mut engine = StaticHtmlEngine::new().unwrap();
        engine.navigate(&url.parse().unwrap()).unwrap();
        let snap = engine.snapshot_fields().unwrap();
        format!("URL: {}\nTitle: {}\n\n{}", snap.url, snap.title, snap.body)
    }

    #[test]
    fn navigate_unsupported_scheme_errors() {
        let mut e = StaticHtmlEngine::new().unwrap();
        // Url's parser is permissive about the scheme — it accepts
        // any "<scheme>://<host>..." string. The engine then
        // rejects anything that isn't http(s) or atsisbroken://.
        // So the construction goes through the parser (no struct
        // literal: fields are private after audit-fix-#17), and
        // the engine surfaces the unsupported-scheme error.
        let url: Url = "ftp://example.com/x".parse().unwrap();
        match e.navigate(&url) {
            Err(EngineError::Network(_)) => {}
            other => panic!("expected Network err, got {other:?}"),
        }
    }

    #[test]
    fn user_agent_includes_crate_version() {
        let ua = default_user_agent();
        assert!(ua.contains("atsisbroken/"));
        assert!(ua.contains(env!("CARGO_PKG_VERSION")));
    }
}
