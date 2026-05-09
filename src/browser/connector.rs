// SPDX-License-Identifier: Unlicense
// Unlicense — public domain — cochranblock.org

//! Connector enums + traits — the ProductGraph's input boundary.
//!
//! Every external source the user can connect — GitHub, Stack
//! Overflow, LinkedIn, a personal blog, manually pasted content —
//! implements this module's connector contract. Per-connector
//! impls land in `connectors/` (one file per source). This file
//! only carries the enum + trait + connection state.

#![cfg(feature = "gui")]
// The composer (Phase K) and per-connector implementations
// haven't landed yet, so today every type here is constructed
// only by tests. cargo's view: dead code. The wire format and
// auth-shape contracts the enum encodes are the actual product
// of this file — once a connector ships, this allow disappears
// on its own. Module-level allow keeps stderr clean during the
// scaffolding phase.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// Identifier for the source of a Product. Adding a connector =
/// adding a variant here. The variant's serde name is what the
/// on-disk ProductGraph stores; renaming a serde name breaks
/// existing user graphs, so additions are append-only.
///
/// Per-variant `#[serde(rename = "...")]` is deliberate and
/// load-bearing. Don't fall back to `rename_all` — `snake_case`
/// splits camelcase variants on case boundaries and produces
/// `"git_hub"`, `"npm_registry"`, `"orc_id"` (Rust audit, bug #2).
/// Each variant pins the canonical wire name explicitly.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ConnectorKind {
    /// Github.com — repos, READMEs, commits, languages.
    #[serde(rename = "github")]
    GitHub,
    #[serde(rename = "gitlab")]
    GitLab,
    #[serde(rename = "bitbucket")]
    Bitbucket,
    #[serde(rename = "codeberg")]
    Codeberg,
    #[serde(rename = "sourcehut")]
    Sourcehut,
    /// Personal blog via RSS / Atom / sitemap.
    #[serde(rename = "personal_blog")]
    PersonalBlog,
    #[serde(rename = "substack")]
    Substack,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "devto")]
    DevTo,
    #[serde(rename = "hashnode")]
    Hashnode,
    /// Q&A history.
    #[serde(rename = "stackoverflow")]
    StackOverflow,
    /// HN comments + submissions.
    #[serde(rename = "hackernews")]
    HackerNews,
    #[serde(rename = "reddit")]
    Reddit,
    /// Microblog identities.
    #[serde(rename = "bluesky")]
    Bluesky,
    #[serde(rename = "mastodon")]
    Mastodon,
    #[serde(rename = "twitter")]
    Twitter,
    /// Published software packages.
    #[serde(rename = "npm")]
    NpmRegistry,
    #[serde(rename = "cratesio")]
    CratesIo,
    #[serde(rename = "pypi")]
    PyPI,
    #[serde(rename = "rubygems")]
    RubyGems,
    #[serde(rename = "dockerhub")]
    DockerHub,
    /// Patents + papers.
    #[serde(rename = "uspto")]
    USPTO,
    #[serde(rename = "arxiv")]
    ArXiv,
    #[serde(rename = "google_scholar")]
    GoogleScholar,
    #[serde(rename = "orcid")]
    OrcID,
    /// Video / audio / talks.
    #[serde(rename = "youtube")]
    YouTube,
    #[serde(rename = "twitch")]
    Twitch,
    /// Design portfolios.
    #[serde(rename = "behance")]
    Behance,
    #[serde(rename = "dribbble")]
    Dribbble,
    #[serde(rename = "artstation")]
    ArtStation,
    /// OAuth-bound services.
    #[serde(rename = "linkedin")]
    LinkedIn,
    /// Federal applicants.
    #[serde(rename = "usajobs")]
    USAJOBS,
    /// User-pasted content. Last resort for sources without
    /// integrations; the user attests authorship.
    #[serde(rename = "manual_paste")]
    ManualPaste,
    /// The legacy resume-parser-derived Profile, treated as a
    /// connector so it composes uniformly with the others.
    #[serde(rename = "manual_resume")]
    ManualResume,
}

impl ConnectorKind {
    /// Every variant in declaration order. Adding a new variant
    /// to the enum and forgetting to add it here is caught by the
    /// `all_slice_covers_every_variant` test below — the test
    /// pattern-matches `self` exhaustively, so a missing arm is
    /// a compile error rather than a silent drop. Tests that
    /// want to assert a property holds across every variant
    /// (display_name, auth_shape, JSON round-trip) iterate over
    /// this slice instead of hand-rolling a sample.
    pub const ALL: &'static [ConnectorKind] = &[
        Self::GitHub,
        Self::GitLab,
        Self::Bitbucket,
        Self::Codeberg,
        Self::Sourcehut,
        Self::PersonalBlog,
        Self::Substack,
        Self::Medium,
        Self::DevTo,
        Self::Hashnode,
        Self::StackOverflow,
        Self::HackerNews,
        Self::Reddit,
        Self::Bluesky,
        Self::Mastodon,
        Self::Twitter,
        Self::NpmRegistry,
        Self::CratesIo,
        Self::PyPI,
        Self::RubyGems,
        Self::DockerHub,
        Self::USPTO,
        Self::ArXiv,
        Self::GoogleScholar,
        Self::OrcID,
        Self::YouTube,
        Self::Twitch,
        Self::Behance,
        Self::Dribbble,
        Self::ArtStation,
        Self::LinkedIn,
        Self::USAJOBS,
        Self::ManualPaste,
        Self::ManualResume,
    ];

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

