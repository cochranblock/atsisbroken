// SPDX-License-Identifier: Unlicense

//! URL handling. We don't pull in the full `url` crate yet — the
//! browser only needs to distinguish absolute URLs from relative,
//! split scheme + host + path, and roundtrip through `Display`.
//! When we wire WebRender / Servo, this module becomes a thin
//! facade over `url::Url`; until then it carries enough behavior
//! to drive the address bar and the network fetch path.

use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Url {
    /// "https" / "http" / "atsisbroken" (internal pages).
    pub scheme: String,
    /// "boards.greenhouse.io" / "" for atsisbroken:// pages.
    pub host: String,
    /// Optional non-default port. None means scheme default.
    pub port: Option<u16>,
    /// "/embed/job_app?for_organization=foo" — leading slash
    /// preserved.
    pub path: String,
    /// Everything after `?`, no leading `?`. Empty when absent.
    pub query: String,
    /// Everything after `#`, no leading `#`. Empty when absent.
    pub fragment: String,
}

impl Url {
    /// Construct a URL for an internal `atsisbroken://` page.
    /// Used for the audit log, applications ledger, settings.
    pub fn internal(page: &str) -> Self {
        Self {
            scheme: "atsisbroken".into(),
            host: page.to_string(),
            port: None,
            path: String::new(),
            query: String::new(),
            fragment: String::new(),
        }
    }

    /// Default homepage on browser launch — the live atsisbroken
    /// landing page. Until the subdomain's DNS lands the launched
    /// browser will surface a "Couldn't load…" page (rendered by
    /// the audit-fix-#11 worker thread); when DNS goes live the
    /// browser starts serving the real page on every launch with
    /// no code change. The in-process `atsisbroken://home` page
    /// stays reachable as the offline / scaffolding fallback —
    /// see [`Self::internal_home`].
    pub fn home() -> Self {
        Url {
            scheme: "https".into(),
            host: "atsisbroken.cochranblock.org".into(),
            port: None,
            path: "/".into(),
            query: String::new(),
            fragment: String::new(),
        }
    }

    /// The in-process `atsisbroken://home` page. Reachable
    /// directly via the address bar; was the default home before
    /// the network landing page took over.
    pub fn internal_home() -> Self {
        Self::internal("home")
    }

    /// True when the URL points to one of our internal pages.
    pub fn is_internal(&self) -> bool {
        self.scheme == "atsisbroken"
    }

    /// True when the URL is fetchable over the network.
    pub fn is_network(&self) -> bool {
        matches!(self.scheme.as_str(), "http" | "https")
    }
}

impl fmt::Display for Url {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}://{}", self.scheme, self.host)?;
        if let Some(port) = self.port {
            write!(f, ":{port}")?;
        }
        write!(f, "{}", self.path)?;
        if !self.query.is_empty() {
            write!(f, "?{}", self.query)?;
        }
        if !self.fragment.is_empty() {
            write!(f, "#{}", self.fragment)?;
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UrlParseError {
    #[error("missing scheme separator (`://`) in {0:?}")]
    NoSchemeSeparator(String),
    #[error("empty host in {0:?}")]
    EmptyHost(String),
    #[error("invalid port {0:?}")]
    InvalidPort(String),
}

impl FromStr for Url {
    type Err = UrlParseError;

    /// Permissive parser. Accepts:
    ///   https://host/path?q=v#frag
    ///   http://host:8080/
    ///   atsisbroken://settings
    /// A bare hostname like "example.com" gets `https://` prepended.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        // Bare hostnames default to https.
        let normalized = if s.contains("://") {
            s.to_string()
        } else if !s.is_empty() && !s.starts_with('/') {
            format!("https://{s}")
        } else {
            return Err(UrlParseError::NoSchemeSeparator(s.to_string()));
        };

        let (scheme, rest) = normalized
            .split_once("://")
            .ok_or_else(|| UrlParseError::NoSchemeSeparator(s.to_string()))?;

        let (authority, after_authority) = match rest.find(['/', '?', '#']) {
            Some(i) => rest.split_at(i),
            None => (rest, ""),
        };
        if authority.is_empty() {
            return Err(UrlParseError::EmptyHost(s.to_string()));
        }
        let (host, port) = if let Some(colon) = authority.rfind(':') {
            let port_str = &authority[colon + 1..];
            // IPv6 brackets contain ':' too; only treat the trailing
            // ':<digits>' as a port.
            if port_str.chars().all(|c| c.is_ascii_digit()) && !port_str.is_empty() {
                let port: u16 = port_str
                    .parse()
                    .map_err(|_| UrlParseError::InvalidPort(port_str.to_string()))?;
                (authority[..colon].to_string(), Some(port))
            } else {
                (authority.to_string(), None)
            }
        } else {
            (authority.to_string(), None)
        };

        let mut tail = after_authority;
        let mut fragment = String::new();
        if let Some(hash) = tail.find('#') {
            fragment = tail[hash + 1..].to_string();
            tail = &tail[..hash];
        }
        let mut query = String::new();
        if let Some(qmark) = tail.find('?') {
            query = tail[qmark + 1..].to_string();
            tail = &tail[..qmark];
        }
        let path = if tail.is_empty() {
            String::new()
        } else {
            tail.to_string()
        };

        Ok(Url {
            scheme: scheme.to_string(),
            host,
            port,
            path,
            query,
            fragment,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_full_https() {
        let u: Url = "https://boards.greenhouse.io/example/jobs/12345?src=foo#about"
            .parse()
            .unwrap();
        assert_eq!(u.scheme, "https");
        assert_eq!(u.host, "boards.greenhouse.io");
        assert_eq!(u.port, None);
        assert_eq!(u.path, "/example/jobs/12345");
        assert_eq!(u.query, "src=foo");
        assert_eq!(u.fragment, "about");
    }

    #[test]
    fn parse_with_port() {
        let u: Url = "http://localhost:8080/test".parse().unwrap();
        assert_eq!(u.host, "localhost");
        assert_eq!(u.port, Some(8080));
        assert_eq!(u.path, "/test");
    }

    #[test]
    fn parse_atsisbroken_internal() {
        let u: Url = "atsisbroken://settings".parse().unwrap();
        assert_eq!(u.scheme, "atsisbroken");
        assert_eq!(u.host, "settings");
        assert_eq!(u.path, "");
        assert!(u.is_internal());
        assert!(!u.is_network());
    }

    #[test]
    fn parse_bare_hostname_defaults_https() {
        let u: Url = "example.com".parse().unwrap();
        assert_eq!(u.scheme, "https");
        assert_eq!(u.host, "example.com");
    }

    #[test]
    fn round_trip_via_display() {
        let original = "https://example.com:443/path?q=1#frag";
        let u: Url = original.parse().unwrap();
        // 443 is the default port for https; we DON'T elide it, the
        // user typed it explicitly.
        assert_eq!(u.to_string(), original);
    }

    #[test]
    fn empty_host_is_error() {
        assert!("https:///path".parse::<Url>().is_err());
    }

    #[test]
    fn home_is_network_landing_page() {
        // The default home is the live landing page on the
        // network. The in-process page is now reachable via
        // Url::internal_home() instead.
        let h = Url::home();
        assert!(h.is_network());
        assert!(!h.is_internal());
        assert_eq!(h.to_string(), "https://atsisbroken.cochranblock.org/");
    }

    #[test]
    fn internal_home_is_atsisbroken_scheme() {
        let h = Url::internal_home();
        assert!(h.is_internal());
        assert_eq!(h.to_string(), "atsisbroken://home");
    }

    #[test]
    fn invalid_port_is_error() {
        // ":notanumber" still treats it as host:port shape but
        // rejects non-digit port.
        // Actually our parser treats non-digit suffix as part of
        // the host, so this parses cleanly. Test the digit case
        // that overflows u16.
        let r = "http://host:99999/".parse::<Url>();
        assert!(r.is_err());
    }

    // ─── Property-based fuzz tests ────────────────────────────────────────
    //
    // Generate random byte sequences and assert the parser:
    // 1. Never panics (use `.parse::<Url>()` — panic-on-Err is
    //    the only signal this catches; we don't expect Ok on
    //    arbitrary inputs).
    // 2. When it returns Ok, Display round-trips equivalently
    //    (modulo the bare-hostname → https:// normalization).
    // 3. Doesn't loop / hang on adversarial input.

    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(500))]

        #[test]
        fn url_parser_does_not_panic_on_arbitrary_bytes(bytes: Vec<u8>) {
            // Convert to a UTF-8 string lossily; the parser
            // takes &str so non-UTF-8 can't reach it directly,
            // but URLs in the wild include weird percent-encoded
            // sequences and Unicode. Lossy conversion produces
            // the kind of input the parser sees in practice.
            let s = String::from_utf8_lossy(&bytes);
            let _ = s.parse::<Url>(); // panic = test fail
        }

        #[test]
        fn url_parser_does_not_panic_on_random_ascii(
            s in "[\\PC]{0,128}"
        ) {
            // \\PC = "any printable character" in proptest's
            // regex spec. Bounded length keeps the fuzz tractable.
            let _ = s.parse::<Url>();
        }

        #[test]
        fn url_with_valid_https_round_trips(
            host in "[a-z]{1,16}\\.[a-z]{2,4}",
            path in "/[a-z0-9/]{0,32}"
        ) {
            let original = format!("https://{host}{path}");
            let url: Url = original.parse().unwrap();
            assert_eq!(url.scheme, "https");
            assert_eq!(url.host, host);
            assert_eq!(url.path, path);
            assert_eq!(url.to_string(), original);
        }

        #[test]
        fn url_with_arbitrary_port_round_trips(
            host in "[a-z]{1,16}",
            port in 1u16..=65535,
        ) {
            let original = format!("http://{host}:{port}/");
            let url: Url = original.parse().unwrap();
            assert_eq!(url.host, host);
            assert_eq!(url.port, Some(port));
            assert_eq!(url.to_string(), original);
        }

        #[test]
        fn atsisbroken_internal_url_round_trips(
            page in "[a-z]{1,16}"
        ) {
            let original = format!("atsisbroken://{page}");
            let url: Url = original.parse().unwrap();
            assert_eq!(url.scheme, "atsisbroken");
            assert_eq!(url.host, page);
            assert!(url.is_internal());
            assert_eq!(url.to_string(), original);
        }
    }
}
