// SPDX-License-Identifier: Unlicense
//! CDP attach proof-of-life.
//!
//! Connects to a Chromium-family browser running with
//! `--remote-debugging-port=N`, lists open tabs, and (in this scaffold
//! pass) prints the field count of the active tab. The real
//! classify-and-fill loop lives in the next iteration; this exists so
//! the CDP transport is wired and demonstrably alive without risking a
//! destructive autofill against a real form prematurely.
//!
//! Pure HTTP probing of `/json/list` here — full chromiumoxide WebSocket
//! pipelining lands when the fill path is wired.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct TabInfo {
    pub title: String,
    pub url: String,
    pub ws_debugger_url: String,
}

/// Minimal HTTP/1.1 GET against `host:port`. We use this instead of
/// pulling in `reqwest`/`hyper` because the surface is tiny and we
/// don't want a network-client dep on the crate's dependency tree.
fn http_get(host: &str, port: u16, path: &str) -> std::io::Result<String> {
    let addr = format!("{host}:{port}");
    let mut stream = TcpStream::connect_timeout(
        &addr.parse::<std::net::SocketAddr>()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?,
        Duration::from_secs(2),
    )?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: {host}:{port}\r\nConnection: close\r\nUser-Agent: atsisbroken/0\r\n\r\n"
    )?;
    let mut buf = String::new();
    stream.read_to_string(&mut buf)?;
    let body_start = buf
        .find("\r\n\r\n")
        .ok_or_else(|| std::io::Error::other("malformed HTTP response"))?;
    Ok(buf[body_start + 4..].to_string())
}

/// Fetch `/json/version` from the debugger port and return the
/// `webSocketDebuggerUrl`. Used by `browser_launcher::wait_for_debug_port`
/// as the readiness signal.
pub fn fetch_json_version(port: u16) -> std::io::Result<String> {
    let body = http_get("127.0.0.1", port, "/json/version")?;
    let v: serde_json::Value =
        serde_json::from_str(&body).map_err(std::io::Error::other)?;
    v.get("webSocketDebuggerUrl")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| std::io::Error::other("missing webSocketDebuggerUrl"))
}

/// Fetch `/json/list` from the debugger port and return parsed tabs.
pub fn list_tabs(port: u16) -> std::io::Result<Vec<TabInfo>> {
    let body = http_get("127.0.0.1", port, "/json/list")?;
    let v: serde_json::Value =
        serde_json::from_str(&body).map_err(std::io::Error::other)?;
    let arr = v
        .as_array()
        .ok_or_else(|| std::io::Error::other("/json/list did not return an array"))?;
    Ok(arr
        .iter()
        .filter_map(|t| {
            Some(TabInfo {
                title: t.get("title")?.as_str()?.to_string(),
                url: t.get("url")?.as_str()?.to_string(),
                ws_debugger_url: t.get("webSocketDebuggerUrl")?.as_str()?.to_string(),
            })
        })
        .collect())
}

/// Parse `http://localhost:9222/json/version` (or any endpoint URL we
/// stored in `Strategy::CdpAttach`) into a `(host, port)` for probing.
pub fn parse_endpoint(endpoint: &str) -> Option<(String, u16)> {
    // Accept "http://host:port/...", "host:port", or bare port digits.
    let s = endpoint.trim_start_matches("http://").trim_start_matches("https://");
    let s = s.split('/').next().unwrap_or(s);
    let (h, p) = s.rsplit_once(':')?;
    let port: u16 = p.parse().ok()?;
    Some((h.to_string(), port))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_endpoint_with_http_scheme() {
        let (h, p) = parse_endpoint("http://localhost:9222/json/version").unwrap();
        assert_eq!(h, "localhost");
        assert_eq!(p, 9222);
    }

    #[test]
    fn parse_endpoint_with_no_scheme() {
        let (h, p) = parse_endpoint("localhost:9222").unwrap();
        assert_eq!(h, "localhost");
        assert_eq!(p, 9222);
    }

    #[test]
    fn parse_endpoint_rejects_garbage() {
        assert!(parse_endpoint("not an endpoint").is_none());
        assert!(parse_endpoint("localhost").is_none());
        assert!(parse_endpoint("localhost:notaport").is_none());
    }

    #[test]
    fn parse_endpoint_with_https_scheme() {
        // We don't actually use HTTPS for CDP (it's local), but the
        // parser shouldn't choke on the scheme.
        let (h, p) = parse_endpoint("https://localhost:9222/").unwrap();
        assert_eq!(h, "localhost");
        assert_eq!(p, 9222);
    }

    #[test]
    fn parse_endpoint_rejects_port_above_u16() {
        // 70000 is past u16::MAX (65535) — must reject, not panic.
        assert!(parse_endpoint("localhost:70000").is_none());
    }

    #[test]
    fn parse_endpoint_with_127_addr() {
        let (h, p) = parse_endpoint("http://127.0.0.1:9222/json/list").unwrap();
        assert_eq!(h, "127.0.0.1");
        assert_eq!(p, 9222);
    }

    #[test]
    fn parse_endpoint_strips_trailing_path_components() {
        // Endpoint URL may carry /json/version etc. — only host:port
        // matters for connection.
        let (h, p) = parse_endpoint("http://localhost:9222/json/version").unwrap();
        assert_eq!(h, "localhost");
        assert_eq!(p, 9222);
    }

    #[test]
    fn parse_endpoint_with_low_port() {
        // Ephemeral / unprivileged ports start at 1024; lower values
        // are valid u16s and must round-trip cleanly.
        let (_, p) = parse_endpoint("localhost:1").unwrap();
        assert_eq!(p, 1);
    }

    #[test]
    fn tab_info_struct_has_required_fields() {
        // Pin the public shape — extension authors rely on these field
        // names if they ever consume `cdp::list_tabs` directly.
        let t = TabInfo {
            title: "Tab".into(),
            url: "https://example.com".into(),
            ws_debugger_url: "ws://localhost:9222/devtools/page/ABC".into(),
        };
        assert_eq!(t.title, "Tab");
        assert!(t.url.starts_with("http"));
        assert!(t.ws_debugger_url.starts_with("ws"));
    }
}
