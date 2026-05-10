// SPDX-License-Identifier: Unlicense

//! URL handling. We don't pull in the full `url` crate yet — the
//! browser only needs to distinguish absolute URLs from relative,
//! split scheme + host + path, and roundtrip through `Display`.
//! When we wire WebRender / Servo, this module becomes a thin
//! facade over `url::Url`; until then it carries enough behavior
//! to drive the address bar and the network fetch path.

use std::fmt;
use std::str::FromStr;

/// Parsed URL. Every external constructor goes through
/// [`Url::from_str`] (the `s.parse::<Url>()` form) or one of the
/// in-module sanctioned helpers ([`Url::internal`], [`Url::home`],
/// [`Url::internal_home`]). Fields are private so a caller can't
/// bypass the parser by writing a struct literal — the audit
/// previously flagged the all-`pub` shape as a discipline gap;
/// fields read via accessors now.
///
/// If a future caller needs a constructor the parser can't
/// express, add a named helper in this module rather than
/// exposing the fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Url {
    scheme: String,
    host: String,
    port: Option<u16>,
    path: String,
    query: String,
    fragment: String,
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

    // ─── Field accessors ─────────────────────────────────────────

    /// Scheme — `"https"` / `"http"` / `"atsisbroken"` for
    /// internal pages.
    pub fn scheme(&self) -> &str {
        &self.scheme
    }
    /// Host — `"boards.greenhouse.io"`, or the page slug for
    /// internal `atsisbroken://` URLs (where the "host" is the
    /// page name).
    pub fn host(&self) -> &str {
        &self.host
    }
    /// Optional non-default port. `None` means "use the scheme's
    /// default port".
    pub fn port(&self) -> Option<u16> {
        self.port
    }
    /// Path with leading `/` preserved. Empty for URLs that omit
    /// the path component.
    pub fn path(&self) -> &str {
        &self.path
    }
    /// Query string without the leading `?`. Empty when absent.
    pub fn query(&self) -> &str {
        &self.query
    }
    /// Fragment without the leading `#`. Empty when absent.
    pub fn fragment(&self) -> &str {
        &self.fragment
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

