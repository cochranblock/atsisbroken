// SPDX-License-Identifier: Unlicense
// Unlicense — public domain — cochranblock.org

//! ProductGraph — the indexed corpus of everything the user has
//! made, pulled from connectors. Eventually queried by the
//! composer; the composer hasn't landed yet, so today the graph
//! is written by connectors and read only by tests + future
//! consumers.
//!
//! This is the central data structure of the application. Every
//! Product has a stable id, a public URL (so citations stay
//! verifiable), an Excerpt set capturing the substantive text the
//! composer will quote from, and source metadata. The eventual
//! citation invariant — every emitted token traces back to a
//! public URL the user already wrote — is enforced by the
//! composer (Phase K, not yet shipped). Connectors and manual
//! user edits are the only writers.
//!
//! On-disk: `~/.atsisbroken/product_graph.json` — atomic write,
//! same pattern as the GitHubInventory and FeedbackQueue.

#![cfg(feature = "gui")]
// The composer (Phase K) and per-connector implementations
// haven't landed yet, so today every type here is constructed
// only by tests. cargo's view: dead code. The schema + the
// save/load round-trip + the wire-format key contract pinned by
// audit-fix-#9 are the actual product of this file — once
// connectors and the composer ship, this allow goes away. See
// browser/connector.rs for the same rationale.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

use super::connector::ConnectorKind;

/// Stable identifier for a product. Format:
/// `<connector>:<host>:<path>` so it survives source reorgs as
/// long as the public URL holds. Example:
/// `github:cochranblock:atsisbroken`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProductId(pub String);

impl ProductId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl std::fmt::Display for ProductId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// One thing the user has made. Repo / blog post / SO answer /
/// patent / paper / package / talk / commit-cluster — all
/// normalize into this shape. Connector-specific richness lives
/// in `metadata` as a free-form key/value map; the composer reads
/// the structured fields; UI surfaces can read metadata for
/// connector-specific badges.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Product {
    pub id: ProductId,
    pub source: ConnectorKind,
    pub kind: ProductKind,
    pub title: String,
    /// Canonical public URL. Citation surface — every emitted
    /// quote points back here. Empty only for ManualPaste sources
    /// where the user explicitly attests the content is theirs.
    pub url: String,
    /// RFC3339. When the product was first published.
    #[serde(default)]
    pub published: String,
    /// RFC3339. Most-recent activity (commit / edit / reply).
    #[serde(default)]
    pub updated: String,
    /// Verbatim excerpts with byte offsets into the source. The
    /// composer quotes from these directly; no preprocessing
    /// allowed. Citation precision = byte_offset + byte_length.
    #[serde(default)]
    pub excerpts: Vec<Excerpt>,
    /// Free-form keywords / tags / language identifiers.
    #[serde(default)]
    pub topics: Vec<String>,
    /// Source-dependent statistics rolled into a generic shape so
    /// downstream code doesn't grow per-connector branches.
    #[serde(default)]
    pub stats: ProductStats,
    #[serde(default)]
    pub authorship: Authorship,
    /// Connector-specific extras. Read by UI for badges; not by
    /// the composer (composer only trusts structured fields).
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProductKind {
    /// A code repository — owned, contributed to, or forked.
    Repo,
    /// A blog post / longform article.
    Post,
    /// A Q&A answer (Stack Overflow, Quora, etc.).
    Answer,
    /// A short post (HN comment, Bluesky / Mastodon / Reddit
    /// post, tweet).
    Microblog,
    /// A published software package (npm / crates.io / PyPI / Docker).
    Package,
    /// A patent — granted or pending.
    Patent,
    /// An academic paper.
    Paper,
    /// A talk / presentation / video / podcast appearance.
    Talk,
    /// A design portfolio piece.
    Design,
    /// A commit cluster — many commits in a single repo treated
    /// as one product when they're tightly themed.
    CommitCluster,
    /// User-pasted content the user attests is theirs.
    ManualPaste,
    /// Catch-all for connectors that grow beyond the above.
    Other,
}

impl Default for ProductKind {
    fn default() -> Self {
        ProductKind::Other
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Excerpt {
    /// The excerpt text. Today's producers sometimes preprocess
    /// (e.g. README paragraph excerpts join consecutive non-blank
    /// lines with a single space for readability); other producers
    /// pass through bytes untouched (commit messages from GitHub).
    /// See each producer's doc for which.
    ///
    /// The eventual citation invariant is: every emitted token
    /// traces back to a public URL the user already wrote. The
    /// in-flight byte-precision contract (offsets pointing into
    /// the original source) was removed when it became clear no
    /// current producer keeps offsets honest. It will land back
    /// on this struct, alongside a producer that does, when the
    /// composer ships and needs the precision.
    pub text: String,
    /// Where in the source body this came from. Routing hint
    /// for the composer (a README intro is "this is the project's
    /// pitch"; a commit message is "this is the work"; a quote is
    /// "the user said this").
    pub kind: ExcerptKind,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExcerptKind {
    /// First paragraph of a README / blog intro / paper abstract.
    Intro,
    /// Body of the artifact — most of the substance.
    Body,
    /// A heading + the paragraph immediately following.
    Section,
    /// A commit message (verbatim).
    CommitMessage,
    /// An answer to a Q&A question.
    AnswerBody,
    /// A short post / comment in its entirety.
    ShortForm,
    /// A patent claim or abstract.
    Claim,
    /// A talk title + abstract.
    TalkAbstract,
    /// A package's README or description field.
    PackageDescription,
    /// User-pasted content that doesn't fit above.
    ManualPaste,
}

impl Default for ExcerptKind {
    fn default() -> Self {
        ExcerptKind::Body
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductStats {
    /// Stars / upvotes / claps / vote count — whatever the
    /// source's "popularity" metric is.
    #[serde(default)]
    pub popularity: u32,
    /// View / download / read count.
    #[serde(default)]
    pub views: u32,
    /// Number of comments / replies / co-conversation entries.
    #[serde(default)]
    pub conversation: u32,
    /// Citation count for academic papers / patents.
    #[serde(default)]
    pub cited_by: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Authorship {
    /// User is the sole author. Cited bullets read as "I built /
    /// I wrote / I designed".
    Sole,
    /// User is the primary author with co-authors. Cited bullets
    /// read as "I led / I co-built". Other authors named.
    PrimaryWith {
        co_authors: Vec<String>,
    },
    /// User is a contributor to a project they didn't lead. Cited
    /// bullets read as "I contributed to". Lead named when known.
    Contributor {
        lead: Option<String>,
    },
    /// Unknown authorship — connector couldn't determine. Composer
    /// emits cautious phrasing ("from <project>").
    Unknown,
}

impl Default for Authorship {
    fn default() -> Self {
        Authorship::Unknown
    }
}

/// Full graph of the user's products + connection state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductGraph {
    /// Schema version anchor. Bumped when the wire format changes
    /// in a way that needs explicit migration. Today: 1.
    #[serde(default = "default_version")]
    pub version: u32,
    /// All products from all connectors, deduplicated by ProductId.
    pub products: Vec<Product>,
    /// Per-connector last-sync timestamp (RFC3339). Keyed by
    /// [`ConnectorKind`] so the wire-format key matches the canonical
    /// name pinned by audit-fix-#1's `#[serde(rename = "...")]`
    /// (e.g. `"github"`, not the Debug-format `"GitHub"`). serde_json
    /// uses the unit-variant's serialized form as the JSON object key.
    #[serde(default)]
    pub last_synced: BTreeMap<ConnectorKind, String>,
}

/// Manual `Default` impl. Don't use `derive(Default)` here:
/// `u32::default()` gives 0, but `#[serde(default = ...)]` on the
/// version field produces 1 for missing-version JSON. Mismatch
/// means `default → save → load → default` is non-identity:
/// the round-trip would mutate 0 → 1 silently. (Rust audit bug #3.)
/// Manual impl pins both paths to the same canonical value.
impl Default for ProductGraph {
    fn default() -> Self {
        Self {
            version: default_version(),
            products: Vec::new(),
            last_synced: BTreeMap::new(),
        }
    }
}

const CURRENT_SCHEMA_VERSION: u32 = 1;

fn default_version() -> u32 {
    CURRENT_SCHEMA_VERSION
}

impl ProductGraph {
    /// Read from disk. Missing file → empty graph (first-run is
    /// not an error). Corrupt file is fatal — the user's connector
    /// state must not be silently dropped.
    pub fn load_from(path: &Path) -> std::io::Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e),
        }
    }

    /// Atomic write via .tmp rename. Same pattern as
    /// `GithubInventory::save_to` and `FeedbackQueue::save_to`.
    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("json.tmp");
        let body = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(&tmp, body)?;
        std::fs::rename(&tmp, path)
    }

    /// Insert or replace a product by id. Connector sync flows
    /// build a Vec<Product> per source then call this to merge.
    pub fn upsert(&mut self, product: Product) {
        match self.products.iter().position(|p| p.id == product.id) {
            Some(i) => self.products[i] = product,
            None => self.products.push(product),
        }
    }

    /// All products from a given connector kind. Composer can
    /// query "what GitHub repos does the user have."
    pub fn from_source(&self, kind: ConnectorKind) -> impl Iterator<Item = &Product> {
        self.products.iter().filter(move |p| p.source == kind)
    }

    /// Mark a connector as synced now. RFC3339 timestamp. The key
    /// stored on disk is the connector's canonical wire name
    /// (`"github"` etc.), not the Rust Debug format — see the
    /// field doc on `last_synced`.
    pub fn record_sync(&mut self, kind: ConnectorKind, when: impl Into<String>) {
        self.last_synced.insert(kind, when.into());
    }

    pub fn last_sync(&self, kind: ConnectorKind) -> Option<&str> {
        self.last_synced.get(&kind).map(|s| s.as_str())
    }
}

