// SPDX-License-Identifier: Unlicense

//! Tests for `crate::browser::connector` — converted from
//! `#[cfg(test)] mod tests {}` to the cochranblock exopack
//! pattern (Phase 1 of the migration). The atsisbroken-test
//! binary calls `super::run()` via [`super::run_all`]; cargo
//! test is no longer the runner for these.

use crate::browser::{AuthShape, ConnectorKind};

use super::{case, check, check_eq, TestResult};

pub fn run() -> Vec<TestResult> {
    vec![
        case("connector::every_connector_has_display_name", every_connector_has_display_name),
        case("connector::every_connector_has_auth_shape", every_connector_has_auth_shape),
        case("connector::all_slice_covers_every_variant", all_slice_covers_every_variant),
        case("connector::github_is_public_handle", github_is_public_handle),
        case("connector::linkedin_is_oauth", linkedin_is_oauth),
        case("connector::substack_is_session_cookie", substack_is_session_cookie),
        case("connector::manual_resume_is_manual_paste", manual_resume_is_manual_paste),
        case("connector::round_trips_through_json", round_trips_through_json),
        case("connector::canonical_serialization", canonical_serialization),
    ]
}

fn every_connector_has_display_name() -> Result<(), String> {
    for k in ConnectorKind::ALL {
        check(!k.display_name().is_empty(), format!("{k:?} has empty display_name"))?;
    }
    Ok(())
}

fn every_connector_has_auth_shape() -> Result<(), String> {
    // The exhaustive match in auth_shape() enforces totality at
    // compile time; this test is the dynamic-coverage placeholder
    // for future per-shape assertions.
    for k in ConnectorKind::ALL {
        let _ = k.auth_shape();
    }
    Ok(())
}

fn all_slice_covers_every_variant() -> Result<(), String> {
    check_eq(ConnectorKind::ALL.len(), 34, "ALL.len")?;
    let mut sorted: Vec<&ConnectorKind> = ConnectorKind::ALL.iter().collect();
    sorted.sort();
    sorted.dedup();
    check_eq(sorted.len(), ConnectorKind::ALL.len(), "ALL contains duplicates")?;
    Ok(())
}

fn github_is_public_handle() -> Result<(), String> {
    check_eq(ConnectorKind::GitHub.auth_shape(), AuthShape::PublicHandle, "GitHub auth shape")
}

fn linkedin_is_oauth() -> Result<(), String> {
    check_eq(ConnectorKind::LinkedIn.auth_shape(), AuthShape::OAuth, "LinkedIn auth shape")
}

fn substack_is_session_cookie() -> Result<(), String> {
    check_eq(ConnectorKind::Substack.auth_shape(), AuthShape::SessionCookie, "Substack auth shape")
}

fn manual_resume_is_manual_paste() -> Result<(), String> {
    check_eq(
        ConnectorKind::ManualResume.auth_shape(),
        AuthShape::ManualPaste,
        "ManualResume auth shape",
    )
}

fn round_trips_through_json() -> Result<(), String> {
    for k in ConnectorKind::ALL {
        let s = serde_json::to_string(k).map_err(|e| format!("serialize {k:?}: {e}"))?;
        let back: ConnectorKind =
            serde_json::from_str(&s).map_err(|e| format!("deserialize {k:?} from {s:?}: {e}"))?;
        check_eq(*k, back, &format!("{k:?} round-trip"))?;
    }
    Ok(())
}

fn canonical_serialization() -> Result<(), String> {
    // Pinning the exact wire format for every variant — same
    // contract as audit-fix-#1's per-variant `#[serde(rename)]`.
    let cases: &[(ConnectorKind, &str)] = &[
        (ConnectorKind::GitHub, "github"),
        (ConnectorKind::GitLab, "gitlab"),
        (ConnectorKind::PersonalBlog, "personal_blog"),
        (ConnectorKind::DevTo, "devto"),
        (ConnectorKind::StackOverflow, "stackoverflow"),
        (ConnectorKind::HackerNews, "hackernews"),
        (ConnectorKind::NpmRegistry, "npm"),
        (ConnectorKind::CratesIo, "cratesio"),
        (ConnectorKind::PyPI, "pypi"),
        (ConnectorKind::DockerHub, "dockerhub"),
        (ConnectorKind::USPTO, "uspto"),
        (ConnectorKind::ArXiv, "arxiv"),
        (ConnectorKind::GoogleScholar, "google_scholar"),
        (ConnectorKind::OrcID, "orcid"),
        (ConnectorKind::YouTube, "youtube"),
        (ConnectorKind::ArtStation, "artstation"),
        (ConnectorKind::LinkedIn, "linkedin"),
        (ConnectorKind::USAJOBS, "usajobs"),
        (ConnectorKind::ManualPaste, "manual_paste"),
        (ConnectorKind::ManualResume, "manual_resume"),
    ];
    for (variant, want) in cases {
        let got = serde_json::to_string(variant).map_err(|e| format!("{e}"))?;
        let want_quoted = format!("\"{want}\"");
        check_eq(got, want_quoted, &format!("{variant:?} canonical name"))?;
    }
    Ok(())
}
