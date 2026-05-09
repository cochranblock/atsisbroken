// SPDX-License-Identifier: Unlicense

//! Tests for `crate::cdp` (Phase 4).

use crate::cdp::{parse_endpoint, TabInfo};

use super::{case, check, check_eq, TestResult};

pub fn run() -> Vec<TestResult> {
    vec![
        case("cdp::parse_endpoint_with_http_scheme", parse_endpoint_with_http_scheme),
        case("cdp::parse_endpoint_with_no_scheme", parse_endpoint_with_no_scheme),
        case("cdp::parse_endpoint_rejects_garbage", parse_endpoint_rejects_garbage),
        case("cdp::parse_endpoint_with_https_scheme", parse_endpoint_with_https_scheme),
        case("cdp::parse_endpoint_rejects_port_above_u16", parse_endpoint_rejects_port_above_u16),
        case("cdp::parse_endpoint_with_127_addr", parse_endpoint_with_127_addr),
        case("cdp::parse_endpoint_strips_trailing_path_components",
             parse_endpoint_strips_trailing_path_components),
        case("cdp::parse_endpoint_with_low_port", parse_endpoint_with_low_port),
        case("cdp::tab_info_struct_has_required_fields", tab_info_struct_has_required_fields),
    ]
}

fn parse_endpoint_with_http_scheme() -> Result<(), String> {
    let (h, p) = parse_endpoint("http://localhost:9222/json/version")
        .ok_or_else(|| "parse failed".to_string())?;
    check_eq(h, "localhost".to_string(), "host")?;
    check_eq(p, 9222u16, "port")
}

fn parse_endpoint_with_no_scheme() -> Result<(), String> {
    let (h, p) = parse_endpoint("localhost:9222").ok_or_else(|| "parse failed".to_string())?;
    check_eq(h, "localhost".to_string(), "host")?;
    check_eq(p, 9222u16, "port")
}

fn parse_endpoint_rejects_garbage() -> Result<(), String> {
    check(parse_endpoint("not an endpoint").is_none(), "garbage rejected")?;
    check(parse_endpoint("localhost").is_none(), "no port rejected")?;
    check(parse_endpoint("localhost:notaport").is_none(), "non-numeric port rejected")
}

fn parse_endpoint_with_https_scheme() -> Result<(), String> {
    // We don't actually use HTTPS for CDP (it's local), but the
    // parser shouldn't choke on the scheme.
    let (h, p) = parse_endpoint("https://localhost:9222/").ok_or_else(|| "parse failed".to_string())?;
    check_eq(h, "localhost".to_string(), "host")?;
    check_eq(p, 9222u16, "port")
}

fn parse_endpoint_rejects_port_above_u16() -> Result<(), String> {
    // 70000 is past u16::MAX (65535) — must reject, not panic.
    check(parse_endpoint("localhost:70000").is_none(), "port above u16 rejected")
}

fn parse_endpoint_with_127_addr() -> Result<(), String> {
    let (h, p) = parse_endpoint("http://127.0.0.1:9222/json/list")
        .ok_or_else(|| "parse failed".to_string())?;
    check_eq(h, "127.0.0.1".to_string(), "host")?;
    check_eq(p, 9222u16, "port")
}

fn parse_endpoint_strips_trailing_path_components() -> Result<(), String> {
    let (h, p) = parse_endpoint("http://localhost:9222/json/version")
        .ok_or_else(|| "parse failed".to_string())?;
    check_eq(h, "localhost".to_string(), "host")?;
    check_eq(p, 9222u16, "port")
}

fn parse_endpoint_with_low_port() -> Result<(), String> {
    let (_, p) = parse_endpoint("localhost:1").ok_or_else(|| "parse failed".to_string())?;
    check_eq(p, 1u16, "port")
}

fn tab_info_struct_has_required_fields() -> Result<(), String> {
    // Pin the public shape — extension authors rely on these field
    // names if they ever consume `cdp::list_tabs` directly.
    let t = TabInfo {
        title: "Tab".into(),
        url: "https://example.com".into(),
        ws_debugger_url: "ws://localhost:9222/devtools/page/ABC".into(),
    };
    check_eq(t.title, "Tab".to_string(), "title")?;
    check(t.url.starts_with("http"), "url starts with http")?;
    check(t.ws_debugger_url.starts_with("ws"), "ws_debugger_url starts with ws")
}
