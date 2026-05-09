// SPDX-License-Identifier: Unlicense

//! Tests for `crate::browser::internal` — converted from
//! `#[cfg(test)] mod tests {}` (Phase 3).

use std::str::FromStr;

use crate::browser::internal::render;
use crate::browser::Url;

use super::{case, check, check_eq, TestResult};

pub fn run() -> Vec<TestResult> {
    vec![
        case("internal::home_renders_with_url_menu", internal_home_renders_with_url_menu),
        case("internal::connections_lists_known_services", connections_lists_known_services),
        case("internal::summary_links_to_connections_when_empty",
             summary_links_to_connections_when_empty),
        case("internal::unknown_url_returns_not_found_with_attempted_path",
             unknown_internal_url_returns_not_found_with_attempted_path),
        case("internal::unknown_url_with_path_includes_path",
             unknown_internal_url_with_path_includes_path),
        case("internal::every_known_page_returns_non_empty_title_and_body",
             every_known_page_returns_non_empty_title_and_body),
        case("internal::page_bodies_have_no_trailing_whitespace_per_line",
             page_bodies_have_no_trailing_whitespace_per_line),
        case("internal::parse_internal_url_routes_correctly", parse_internal_url_routes_correctly),
        case("internal::shield_documents_blocked_categories", shield_documents_blocked_categories),
        case("internal::fingerprint_page_documents_webdriver_default",
             fingerprint_page_documents_webdriver_default),
    ]
}

fn internal_home_renders_with_url_menu() -> Result<(), String> {
    // The in-process home page (still reachable via
    // atsisbroken://home, even though Url::home() now points at
    // the live landing page).
    let page = render(&Url::internal_home());
    check_eq(page.title.clone(), "atsisbroken".to_string(), "home title")?;
    check(page.body.contains("atsisbroken://connections"), "home menu has connections")?;
    check(page.body.contains("atsisbroken://summary"), "home menu has summary")?;
    check(page.body.contains("atsisbroken://discover"), "home menu has discover")
}

fn connections_lists_known_services() -> Result<(), String> {
    let page = render(&Url::internal("connections"));
    check(page.title.contains("connections"), "title mentions connections")?;
    // Public-handle services
    check(page.body.contains("GitHub"), "lists GitHub")?;
    check(page.body.contains("Stack Overflow"), "lists Stack Overflow")?;
    check(page.body.contains("Hacker News"), "lists Hacker News")?;
    check(page.body.contains("crates.io"), "lists crates.io")?;
    // OAuth services
    check(page.body.contains("LinkedIn"), "lists LinkedIn")?;
    check(page.body.contains("Reddit"), "lists Reddit")?;
    // Session-cookie services
    check(page.body.contains("Substack"), "lists Substack")?;
    // Connect targets
    check(page.body.contains("atsisbroken://connect/github"), "github connect target")
}

fn summary_links_to_connections_when_empty() -> Result<(), String> {
    let page = render(&Url::internal("summary"));
    check(
        page.body.contains("atsisbroken://connections"),
        "summary directs to connections when empty",
    )
}

fn unknown_internal_url_returns_not_found_with_attempted_path() -> Result<(), String> {
    let url: Url = "atsisbroken://garbage".parse().map_err(|e| format!("{e}"))?;
    let page = render(&url);
    check_eq(page.title.clone(), "Page not found".to_string(), "404 title")?;
    check(page.body.contains("atsisbroken://garbage"), "404 body cites attempted URL")
}

fn unknown_internal_url_with_path_includes_path() -> Result<(), String> {
    let url: Url = "atsisbroken://garbage/some/path"
        .parse()
        .map_err(|e| format!("{e}"))?;
    let page = render(&url);
    check(
        page.body.contains("atsisbroken://garbage/some/path"),
        "404 body cites full path",
    )
}

fn every_known_page_returns_non_empty_title_and_body() -> Result<(), String> {
    // Pin: every routed page has both title and body. An empty
    // surface is worse than a "not yet implemented" placeholder.
    for host in [
        "home",
        "connections",
        "summary",
        "discover",
        "queue",
        "applications",
        "responses",
        "identities",
        "sources",
        "shield",
        "fingerprint",
        "network",
        "settings",
        "audit-bar",
        "onboarding",
    ] {
        let page = render(&Url::internal(host));
        check(!page.title.is_empty(), format!("{host}: empty title"))?;
        check(!page.body.is_empty(), format!("{host}: empty body"))?;
    }
    Ok(())
}

fn page_bodies_have_no_trailing_whitespace_per_line() -> Result<(), String> {
    // Discipline: text-rendering keeps lines clean. Trailing
    // whitespace can produce phantom spaces in the rendered
    // glyph layout.
    for host in ["home", "connections", "summary"] {
        let page = render(&Url::internal(host));
        for (i, line) in page.body.lines().enumerate() {
            check_eq(
                line,
                line.trim_end(),
                &format!("{host} line {i} has trailing whitespace: {line:?}"),
            )?;
        }
    }
    Ok(())
}

fn parse_internal_url_routes_correctly() -> Result<(), String> {
    let url = Url::from_str("atsisbroken://applications").map_err(|e| format!("{e}"))?;
    let page = render(&url);
    check(
        page.title.to_lowercase().contains("applications"),
        format!("title should contain applications: {}", page.title),
    )
}

fn shield_documents_blocked_categories() -> Result<(), String> {
    let page = render(&Url::internal("shield"));
    check(page.body.contains("Analytics"), "shield mentions Analytics")?;
    check(page.body.contains("Cookie walls"), "shield mentions Cookie walls")?;
    check(page.body.contains("ATS telemetry"), "shield mentions ATS telemetry")
}

fn fingerprint_page_documents_webdriver_default() -> Result<(), String> {
    // Pins the fingerprint page text — it advertises the planned
    // webdriver=false default. This test pins the page text, not
    // the runtime guarantee (no JS engine yet).
    let page = render(&Url::internal("fingerprint"));
    check(page.body.contains("navigator.webdriver"), "page mentions navigator.webdriver")?;
    check(page.body.contains("false"), "page mentions false")
}
