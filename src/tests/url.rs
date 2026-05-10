// SPDX-License-Identifier: Unlicense

//! Tests for `crate::browser::url` — Phase 10. Includes a
//! hand-rolled fuzz runner via [`super::fuzz`] that replaces the
//! cargo-test proptest cases. Determinism: each fuzz call is
//! seeded with a fixed u64; runs reproduce across machines.

use crate::browser::Url;

use super::{case, check, check_eq, fuzz, TestResult};

pub fn run() -> Vec<TestResult> {
    vec![
        case("url::parse_full_https", parse_full_https),
        case("url::parse_with_port", parse_with_port),
        case("url::parse_atsisbroken_internal", parse_atsisbroken_internal),
        case("url::parse_bare_hostname_defaults_https", parse_bare_hostname_defaults_https),
        case("url::round_trip_via_display", round_trip_via_display),
        case("url::empty_host_is_error", empty_host_is_error),
        case("url::home_is_network_landing_page", home_is_network_landing_page),
        case("url::internal_home_is_atsisbroken_scheme", internal_home_is_atsisbroken_scheme),
        case("url::invalid_port_is_error", invalid_port_is_error),
        case("url::fuzz_parser_does_not_panic_on_arbitrary_bytes",
             fuzz_parser_does_not_panic_on_arbitrary_bytes),
        case("url::fuzz_parser_does_not_panic_on_random_ascii",
             fuzz_parser_does_not_panic_on_random_ascii),
        case("url::fuzz_valid_https_round_trips", fuzz_valid_https_round_trips),
        case("url::fuzz_arbitrary_port_round_trips", fuzz_arbitrary_port_round_trips),
        case("url::fuzz_atsisbroken_internal_url_round_trips",
             fuzz_atsisbroken_internal_url_round_trips),
    ]
}

fn parse_full_https() -> Result<(), String> {
    let u: Url = "https://boards.greenhouse.io/example/jobs/12345?src=foo#about"
        .parse()
        .map_err(|e| format!("{e}"))?;
    check_eq(u.scheme(), "https", "scheme")?;
    check_eq(u.host(), "boards.greenhouse.io", "host")?;
    check_eq(u.port(), None, "port")?;
    check_eq(u.path(), "/example/jobs/12345", "path")?;
    check_eq(u.query(), "src=foo", "query")?;
    check_eq(u.fragment(), "about", "fragment")
}

fn parse_with_port() -> Result<(), String> {
    let u: Url = "http://localhost:8080/test".parse().map_err(|e| format!("{e}"))?;
    check_eq(u.host(), "localhost", "host")?;
    check_eq(u.port(), Some(8080u16), "port")?;
    check_eq(u.path(), "/test", "path")
}

fn parse_atsisbroken_internal() -> Result<(), String> {
    let u: Url = "atsisbroken://settings".parse().map_err(|e| format!("{e}"))?;
    check_eq(u.scheme(), "atsisbroken", "scheme")?;
    check_eq(u.host(), "settings", "host")?;
    check_eq(u.path(), "", "path")?;
    check(u.is_internal(), "is_internal")?;
    check(!u.is_network(), "!is_network")
}

fn parse_bare_hostname_defaults_https() -> Result<(), String> {
    let u: Url = "example.com".parse().map_err(|e| format!("{e}"))?;
    check_eq(u.scheme(), "https", "scheme defaults to https")?;
    check_eq(u.host(), "example.com", "host")
}

fn round_trip_via_display() -> Result<(), String> {
    let original = "https://example.com:443/path?q=1#frag";
    let u: Url = original.parse().map_err(|e| format!("{e}"))?;
    check_eq(u.to_string().as_str(), original, "round-trip via Display")
}

fn empty_host_is_error() -> Result<(), String> {
    check("https:///path".parse::<Url>().is_err(), "empty host should error")
}

fn home_is_network_landing_page() -> Result<(), String> {
    let h = Url::home();
    check(h.is_network(), "home is network")?;
    check(!h.is_internal(), "home is not internal")?;
    check_eq(
        h.to_string(),
        "https://atsisbroken.cochranblock.org/".to_string(),
        "home URL",
    )
}

fn internal_home_is_atsisbroken_scheme() -> Result<(), String> {
    let h = Url::internal_home();
    check(h.is_internal(), "internal_home is internal")?;
    check_eq(h.to_string(), "atsisbroken://home".to_string(), "internal_home URL")
}

fn invalid_port_is_error() -> Result<(), String> {
    let r = "http://host:99999/".parse::<Url>();
    check(r.is_err(), "port > u16::MAX should error")
}

// ─── Hand-rolled fuzz tests (replace proptest) ──────────────────────

fn fuzz_parser_does_not_panic_on_arbitrary_bytes() -> Result<(), String> {
    fuzz(
        0xA753_15B3_0FAB_1ED5,
        500,
        |rng| rng.bytes(128),
        |bytes| {
            let s = String::from_utf8_lossy(bytes);
            let _ = s.parse::<Url>();
            Ok(())
        },
    )
}

fn fuzz_parser_does_not_panic_on_random_ascii() -> Result<(), String> {
    fuzz(
        0xC0FF_EE00_DEAD_BEEF,
        500,
        |rng| rng.printable_ascii(128),
        |s| {
            let _ = s.parse::<Url>();
            Ok(())
        },
    )
}

fn fuzz_valid_https_round_trips() -> Result<(), String> {
    fuzz(
        0xBADD_F00D_BABE_CAFE,
        500,
        |rng| {
            let host = format!("{}.{}", rng.lower_ascii(1, 16), rng.lower_ascii(2, 4));
            let path_seg = rng.lower_ascii(0, 32);
            // proptest's "/[a-z0-9/]{0,32}" allowed slashes inside;
            // we approximate with a flat lowercase segment plus
            // optional leading slash. The contract pinned is "what
            // gets parsed round-trips through Display."
            let path = format!("/{path_seg}");
            (host, path)
        },
        |(host, path)| {
            let original = format!("https://{host}{path}");
            let url: Url = original.parse().map_err(|e| format!("parse: {e}"))?;
            check_eq(url.scheme(), "https", "scheme")?;
            check_eq(url.host(), host.as_str(), "host")?;
            check_eq(url.path(), path.as_str(), "path")?;
            check_eq(url.to_string(), original, "round-trip")
        },
    )
}

fn fuzz_arbitrary_port_round_trips() -> Result<(), String> {
    fuzz(
        0xFEED_FACE_C0DE_1234,
        500,
        |rng| {
            let host = rng.lower_ascii(1, 16);
            let port = (rng.range(1, 65536)) as u16;
            (host, port)
        },
        |(host, port)| {
            let original = format!("http://{host}:{port}/");
            let url: Url = original.parse().map_err(|e| format!("parse: {e}"))?;
            check_eq(url.host(), host.as_str(), "host")?;
            check_eq(url.port(), Some(*port), "port")?;
            check_eq(url.to_string(), original, "round-trip")
        },
    )
}

fn fuzz_atsisbroken_internal_url_round_trips() -> Result<(), String> {
    fuzz(
        0x1337_DEAD_C0FF_EE42,
        500,
        |rng| rng.lower_ascii(1, 16),
        |page| {
            let original = format!("atsisbroken://{page}");
            let url: Url = original.parse().map_err(|e| format!("parse: {e}"))?;
            check_eq(url.scheme(), "atsisbroken", "scheme")?;
            check_eq(url.host(), page.as_str(), "host (page)")?;
            check(url.is_internal(), "is_internal")?;
            check_eq(url.to_string(), original, "round-trip")
        },
    )
}
