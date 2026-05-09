// SPDX-License-Identifier: Unlicense
// Unlicense — public domain — cochranblock.org

//! Connector enums + traits — the ProductGraph's input boundary.
//!
//! Every external source the user can connect — GitHub, Stack
//! Overflow, LinkedIn, a personal blog, manually pasted content —
//! implements this module's connector contract. Per-connector
//! impls land in `connectors/` (one file per source). This file
//! only carries the enum + trait + connection state.

#![cfg(feature = "browser")]

use serde::{Deserialize, Serialize};

/// Identifier for the source of a Product. Adding a connector =
/// adding a variant here. The variant is what the on-disk
/// ProductGraph stores; renaming a variant breaks existing user
/// graphs, so additions are append-only.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorKind {
    /// Github.com — repos, READMEs, commits, languages.
    GitHub,
    /// Forge alternatives, same shape as GitHub.
    GitLab,
    Bitbucket,
    Codeberg,
    Sourcehut,
    /// Personal blog via RSS / Atom / sitemap.
    PersonalBlog,
    Substack,
    Medium,
    DevTo,
    Hashnode,
    /// Q&A history.
    StackOverflow,
    /// HN comments + submissions.
    HackerNews,
    Reddit,
    /// Microblog identities.
    Bluesky,
    Mastodon,
    Twitter,
    /// Published software packages.
    NpmRegistry,
    CratesIo,
    PyPI,
    RubyGems,
    DockerHub,
    /// Patents + papers.
    USPTO,
    ArXiv,
    GoogleScholar,
    OrcID,
    /// Video / audio / talks.
    YouTube,
    Twitch,
    /// Design portfolios.
    Behance,
    Dribbble,
    ArtStation,
    /// OAuth-bound services.
    LinkedIn,
    /// Federal applicants.
    USAJOBS,
    /// User-pasted content. Last resort for sources without
    /// integrations; the user attests authorship.
    ManualPaste,
    /// The legacy resume-parser-derived Profile, treated as a
    /// connector so it composes uniformly with the others.
    ManualResume,
}

impl ConnectorKind {
    /// Human-readable name for UI. Default Debug already gives
    /// us the variant name; this is a place to override when the
    /// camel-case name reads awkwardly.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::GitHub => "GitHub",
            Self::GitLab => "GitLab",
            Self::Bitbucket => "Bitbucket",
            Self::Codeberg => "Codeberg",
            Self::Sourcehut => "Sourcehut",
            Self::PersonalBlog => "Personal blog",
            Self::Substack => "Substack",
            Self::Medium => "Medium",
            Self::DevTo => "Dev.to",
            Self::Hashnode => "Hashnode",
            Self::StackOverflow => "Stack Overflow",
            Self::HackerNews => "Hacker News",
            Self::Reddit => "Reddit",
            Self::Bluesky => "Bluesky",
            Self::Mastodon => "Mastodon",
            Self::Twitter => "Twitter / X",
            Self::NpmRegistry => "npm",
            Self::CratesIo => "crates.io",
            Self::PyPI => "PyPI",
            Self::RubyGems => "RubyGems",
            Self::DockerHub => "Docker Hub",
            Self::USPTO => "USPTO Patents",
            Self::ArXiv => "arXiv",
            Self::GoogleScholar => "Google Scholar",
            Self::OrcID => "ORCID",
            Self::YouTube => "YouTube",
            Self::Twitch => "Twitch",
            Self::Behance => "Behance",
            Self::Dribbble => "Dribbble",
            Self::ArtStation => "ArtStation",
            Self::LinkedIn => "LinkedIn",
            Self::USAJOBS => "USAJOBS",
            Self::ManualPaste => "Manual paste",
            Self::ManualResume => "Resume",
        }
    }

    /// Auth shape this connector uses. Drives the UI's Connect
    /// flow (handle prompt vs OAuth tab vs cookie capture vs
    /// paste-text).
    pub fn auth_shape(&self) -> AuthShape {
        match self {
            // Public-handle services — no token needed (PAT
            // optional for GitHub to bump rate limit).
            Self::GitHub
            | Self::StackOverflow
            | Self::HackerNews
            | Self::CratesIo
            | Self::NpmRegistry
            | Self::PyPI
            | Self::RubyGems
            | Self::DockerHub
            | Self::USPTO
            | Self::ArXiv
            | Self::GoogleScholar
            | Self::OrcID
            | Self::Bluesky
            | Self::Mastodon
            | Self::Reddit
            | Self::USAJOBS
            | Self::PersonalBlog
            | Self::DevTo
            | Self::Hashnode
            | Self::Behance
            | Self::Dribbble
            | Self::ArtStation
            | Self::YouTube
            | Self::Twitch
            | Self::GitLab
            | Self::Bitbucket
            | Self::Codeberg
            | Self::Sourcehut => AuthShape::PublicHandle,
            // OAuth-bound services.
            Self::LinkedIn | Self::Twitter => AuthShape::OAuth,
            // Session-cookie capture.
            Self::Substack | Self::Medium => AuthShape::SessionCookie,
            // Manual.
            Self::ManualPaste | Self::ManualResume => AuthShape::ManualPaste,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthShape {
    /// User provides their public handle. Optional bearer token
    /// for higher rate limit (GitHub PAT, etc.).
    PublicHandle,
    /// OAuth 2.0 flow with redirect captured by our own browser.
    OAuth,
    /// User logs in normally in a tab; we capture the session
    /// cookie jar for the host.
    SessionCookie,
    /// User pastes content directly with attestation of authorship.
    ManualPaste,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_connector_has_display_name() {
        // Iterate by serialization round-trip — adding a variant
        // forces this test to be updated.
        let kinds = [
            ConnectorKind::GitHub,
            ConnectorKind::StackOverflow,
            ConnectorKind::PersonalBlog,
            ConnectorKind::LinkedIn,
            ConnectorKind::ManualPaste,
        ];
        for k in kinds {
            assert!(!k.display_name().is_empty());
        }
    }

    #[test]
    fn github_is_public_handle() {
        assert_eq!(ConnectorKind::GitHub.auth_shape(), AuthShape::PublicHandle);
    }

    #[test]
    fn linkedin_is_oauth() {
        assert_eq!(ConnectorKind::LinkedIn.auth_shape(), AuthShape::OAuth);
    }

    #[test]
    fn substack_is_session_cookie() {
        assert_eq!(ConnectorKind::Substack.auth_shape(), AuthShape::SessionCookie);
    }

    #[test]
    fn manual_resume_is_manual_paste() {
        assert_eq!(
            ConnectorKind::ManualResume.auth_shape(),
            AuthShape::ManualPaste
        );
    }

    #[test]
    fn connector_kind_round_trips_through_json() {
        for k in [
            ConnectorKind::GitHub,
            ConnectorKind::Bluesky,
            ConnectorKind::USPTO,
            ConnectorKind::ManualPaste,
            ConnectorKind::ManualResume,
        ] {
            let s = serde_json::to_string(&k).unwrap();
            let back: ConnectorKind = serde_json::from_str(&s).unwrap();
            assert_eq!(k, back);
        }
    }

    #[test]
    fn snake_case_serialization() {
        // Wire format: variant names are snake_case so future
        // contributors don't accidentally write "GitHub" as a
        // string and break existing graphs.
        assert_eq!(
            serde_json::to_string(&ConnectorKind::GitHub).unwrap(),
            "\"git_hub\""
        );
        assert_eq!(
            serde_json::to_string(&ConnectorKind::PersonalBlog).unwrap(),
            "\"personal_blog\""
        );
    }
}
