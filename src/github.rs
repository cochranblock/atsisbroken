// SPDX-License-Identifier: Unlicense
// Unlicense — public domain — cochranblock.org
// Contributors: GotEmCoach, KOVA, Claude Opus 4.7

//! GitHub inventory — Phase I.
//!
//! Per-user snapshot of public-repo metadata, README excerpts, and
//! recent commit messages. Phase K's answer composer will consume
//! this as one of its source streams for free-form ATS prompts
//! ("describe a project," "biggest technical challenge," etc.) —
//! the composer itself hasn't landed yet. This module's job is
//! to fetch and persist the data with enough fidelity that the
//! composer can quote from it later.
//!
//! ## What we deliberately do NOT store
//! - Source code (only file structure + line counts via FileSummary)
//! - Issue / PR bodies (only counts)
//! - Email addresses scraped from commits
//! - Private repo content of any kind
//!
//! ## On-disk layout
//! - `~/.atsisbroken/github_inventory.json` — the inventory itself
//! - `~/.atsisbroken/github_token` — optional Personal Access Token,
//!   chmod 600 on Unix. Without it, the sync is rate-limited to 60
//!   req/h by GitHub; with it, 5000 req/h.
//!
//! ## Network posture
//! Outbound only, to `api.github.com`. The inventory itself stays
//! local — no telemetry, no sync to atsisbroken's authors. The
//! HTTP fetch logic + CLI `connect-github` / `sync-github`
//! subcommands land in Phase I.b; this module ships the schema +
//! JSON I/O + token storage so the foundation is bug-pinned before
//! a network dep is wired in.

use crate::RepoRef;
use crate::paths;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// GitHub REST API base URL.
pub const GITHUB_API_BASE: &str = "https://api.github.com";

/// Identifying User-Agent. GitHub's API requires one; using the crate
/// name + version + repo URL keeps it auditable.
pub const USER_AGENT: &str = concat!(
    "atsisbroken/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/cochranblock/atsisbroken)"
);

/// Maximum repos to deep-fetch (readme + commits + languages) per
/// sync. Sorted by stars descending so the top-N most-visible repos
/// get the rich treatment; the rest are listed with metadata only.
/// Caps the API call count to ~3*N + 1 — well under the 60/h
/// anonymous rate limit for typical N=10.
pub const TOP_N_REPOS_FOR_DEEP_FETCH: usize = 10;

/// Maximum readme excerpt total bytes per repo. Plan §3 cap.
pub const README_EXCERPT_CAP_BYTES: usize = 2048;

/// Maximum recent commit messages stored per repo. Plan §3 cap.
pub const RECENT_COMMITS_CAP: usize = 50;

/// One user's GitHub inventory. Top-level on-disk JSON shape at
/// `~/.atsisbroken/github_inventory.json`.
///
/// Eq dropped: `language_distribution` carries f32 fractions, and
/// f32 isn't Eq because of NaN. Same posture as Profile (see lib.rs).
/// PartialEq is enough for assert_eq! in tests.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct GithubInventory {
    /// GitHub login, e.g. "GotEmCoach". Empty when not yet connected.
    pub handle: String,
    /// RFC3339 timestamp of the most recent successful sync. Empty
    /// when the file exists but was never populated.
    pub last_synced: String,
    pub public_repos: Vec<RepoSnapshot>,
    /// External repos the user has contributed to via PRs (only the
    /// (owner, name, url) tuple — not the contributed code).
    #[serde(default)]
    pub contributed_to: Vec<RepoRef>,
    /// Aggregate language distribution across `public_repos`,
    /// language → fraction of bytes [0.0, 1.0]. Populated from the
    /// per-repo `languages` map at sync time.
    #[serde(default)]
    pub language_distribution: BTreeMap<String, f32>,
    /// Sum of `RepoSnapshot::total_commits` across `public_repos`.
    #[serde(default)]
    pub total_public_commits: u32,
    #[serde(default)]
    pub follower_count: u32,
    #[serde(default)]
    pub starred_count: u32,
}

/// Per-repo metadata. Design intent for the composer (Phase K,
/// not yet implemented): each emitted freetext token traces back
/// to a `(owner, name, commit_sha, readme_section)` citation in
/// one of `readme_excerpts`, `recent_commit_messages`, or a
/// structural fact (name / primary_language / stars). This struct
/// is the data side of that contract; the composer that actually
/// enforces "every token has a citation" lands later.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct RepoSnapshot {
    pub owner: String,
    pub name: String,
    /// Repo description from the GitHub /repos endpoint. Empty when
    /// the user hasn't set one.
    #[serde(default)]
    pub description: String,
    /// GitHub-detected primary language. Empty for repos with no
    /// detectable language (e.g. docs-only).
    #[serde(default)]
    pub primary_language: String,
    /// Per-language byte count from /repos/{owner}/{name}/languages.
    /// Stored as Vec<(lang, bytes)> rather than a BTreeMap so the
    /// on-disk order stays deterministic (sorted by bytes desc).
    #[serde(default)]
    pub languages: Vec<(String, u64)>,
    #[serde(default)]
    pub stars: u32,
    #[serde(default)]
    pub forks: u32,
    /// Open issues — proxy for "live project."
    #[serde(default)]
    pub open_issues: u32,
    /// Closed issues — proxy for "problems solved."
    #[serde(default)]
    pub closed_issues: u32,
    /// RFC3339 timestamp of the most recent commit. Empty for
    /// freshly-created repos with no commits.
    #[serde(default)]
    pub last_commit: String,
    /// Total commit count on the default branch.
    #[serde(default)]
    pub total_commits: u32,
    /// GitHub topics ("rust", "machine-learning", etc.). Useful for
    /// the answer composer's per-prompt repo selection.
    #[serde(default)]
    pub topics: Vec<String>,
    /// SPDX license identifier ("MIT", "Unlicense", "Apache-2.0").
    /// Empty for unlicensed repos (legally important per ATS forms
    /// asking about open-source contributions under permissive licenses).
    #[serde(default)]
    pub license: String,
    /// Whether this repo is a fork of another. Composer skips forks
    /// for "describe a project" answers (forks are not authored work
    /// in the answer-quality sense).
    #[serde(default)]
    pub is_fork: bool,
    /// Paragraph-shaped excerpts from README.md, capped at 2 KiB
    /// per repo. Each excerpt is the contiguous block of non-blank
    /// lines between blank-line boundaries, line-joined with a
    /// single space — readable for citation prose like "the README
    /// describes it as: «<excerpt>»" but NOT a byte-for-byte copy
    /// of the README (interior newlines + indentation collapse to
    /// single spaces). When the composer needs byte-precision
    /// citations into the original README, [`extract_excerpts`]
    /// will need to grow byte-offset metadata; today it doesn't.
    /// Empty for repos with no README or with README content over
    /// the cap (truncated).
    #[serde(default)]
    pub readme_excerpts: Vec<String>,
    /// Recent commit messages, byte-for-byte from the GitHub API
    /// (no preprocessing on our end), capped at 50 entries.
    /// Used as evidence in technical-challenge answers.
    #[serde(default)]
    pub recent_commit_messages: Vec<String>,
    /// Top files by line count (path, language, lines). Purely
    /// structural — no source code stored.
    #[serde(default)]
    pub top_files: Vec<FileSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct FileSummary {
    pub path: String,
    /// GitHub-detected language for this file. Empty when not
    /// detectable (e.g. plain-text docs).
    #[serde(default)]
    pub language: String,
    pub lines: u32,
}

impl GithubInventory {
    /// Read the inventory from disk. A missing file yields an empty
    /// inventory (first-run is not an error). A corrupt file is
    /// fatal — the user's connect-github state must not be silently
    /// dropped.
    pub fn load_from(path: &Path) -> std::io::Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e),
        }
    }

    /// Atomically write the inventory to disk. Writes to `<path>.tmp`
    /// then renames; a crash mid-write leaves the previous file
    /// intact. Same atomic-write pattern as FeedbackQueue::save_to.
    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("json.tmp");
        let body = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(&tmp, body)?;
        std::fs::rename(&tmp, path)
    }

    /// Convenience: load the inventory from the canonical path.
    pub fn load() -> std::io::Result<Self> {
        Self::load_from(&paths::github_inventory_path())
    }

    /// Convenience: save the inventory to the canonical path.
    pub fn save(&self) -> std::io::Result<()> {
        self.save_to(&paths::github_inventory_path())
    }
}

// ─── token storage ────────────────────────────────────────────────────────

/// Save a GitHub Personal Access Token to disk with owner-only
/// permissions.
///
/// On Unix, sets mode 0600 after write. On Windows, relies on the
/// default per-user filesystem ACL (the `~/.atsisbroken/` directory
/// inherits from the user profile dir, which is owner-only by
/// default; a determined attacker with the user's session can read
/// any of their files anyway, which is the same threat model as
/// stored browser cookies).
///
/// The token is whitespace-trimmed before write so a copy-paste with
/// trailing newline is normalized.
pub fn save_github_token(token: &str, path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let trimmed = token.trim();
    let tmp = path.with_extension("token.tmp");
    std::fs::write(&tmp, trimmed.as_bytes())?;
    std::fs::rename(&tmp, path)?;
    set_owner_only_perms(path)
}

/// Read the GitHub token from disk. Returns Ok(None) when the file
/// doesn't exist (anonymous mode is supported), Err on I/O failures.
/// Trims whitespace on read, same as on save, so save→load is
/// idempotent across editors that auto-append newlines.
pub fn load_github_token(path: &Path) -> std::io::Result<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                Ok(Some(trimmed.to_string()))
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// Set 0600 (rw-------) on Unix; no-op on Windows where the file
/// inherits user-profile ACL. Returns Ok on platforms where chmod
/// doesn't apply.
#[cfg(unix)]
fn set_owner_only_perms(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o600);
    std::fs::set_permissions(path, perms)
}

#[cfg(not(unix))]
fn set_owner_only_perms(_path: &Path) -> std::io::Result<()> {
    // Best effort on non-Unix; the user-profile ACL inherits.
    Ok(())
}

// ─── API parsers (fixture-testable; no network) ────────────────────────────

/// Subset of GitHub's /users/{user} response we care about. Only
/// fields atsisbroken stores. Other fields are ignored.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct UserInfo {
    pub login: String,
    #[serde(default)]
    pub public_repos: u32,
    #[serde(default)]
    pub followers: u32,
}

/// Subset of GitHub's /users/{user}/repos response per element.
/// Other fields ignored.
#[derive(Debug, Clone, Deserialize)]
pub struct RepoApiResponse {
    pub name: String,
    pub owner: RepoOwner,
    pub html_url: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub stargazers_count: u32,
    #[serde(default)]
    pub forks_count: u32,
    #[serde(default)]
    pub open_issues_count: u32,
    #[serde(default)]
    pub pushed_at: Option<String>,
    #[serde(default)]
    pub topics: Vec<String>,
    #[serde(default)]
    pub license: Option<License>,
    #[serde(default)]
    pub fork: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RepoOwner {
    pub login: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct License {
    /// SPDX identifier ("MIT", "Apache-2.0", "Unlicense").
    /// Sometimes null in GitHub responses; serde_json::Value handles it.
    #[serde(default)]
    pub spdx_id: Option<String>,
}

/// Parse the /users/{user} response.
pub fn parse_user_info(json: &str) -> serde_json::Result<UserInfo> {
    serde_json::from_str(json)
}

/// Parse the /users/{user}/repos response (an array).
pub fn parse_repos_array(json: &str) -> serde_json::Result<Vec<RepoApiResponse>> {
    serde_json::from_str(json)
}

/// Decode a /repos/{owner}/{repo}/readme response and extract
/// paragraph-shaped excerpts capped at `README_EXCERPT_CAP_BYTES`.
/// The API returns `{"content": "<base64>", "encoding": "base64"}`
/// with the README body. We base64-decode, then split into
/// non-empty paragraphs and keep them in order until the byte cap
/// is hit. Within a paragraph, consecutive non-blank lines are
/// joined with a single space — convenient for citation prose,
/// but it does mean the excerpt is NOT a byte-for-byte slice of
/// the original README. See [`extract_excerpts`] for the join
/// behavior.
///
/// Returns `Vec<String>` because the composer (Phase K) cites
/// "the README describes it as: «<excerpt>»" and prefers
/// paragraph-shaped quotes over a single blob.
pub fn parse_readme_excerpts(json: &str) -> Result<Vec<String>, ReadmeParseError> {
    #[derive(Deserialize)]
    struct ReadmeBody {
        content: String,
        #[serde(default)]
        encoding: String,
    }
    let body: ReadmeBody =
        serde_json::from_str(json).map_err(ReadmeParseError::Json)?;
    if body.encoding != "base64" {
        return Err(ReadmeParseError::UnknownEncoding(body.encoding));
    }
    // GitHub returns the content with embedded newlines every 60
    // chars; base64::Engine::decode tolerates whitespace if we use
    // the LENIENT engine. Use STANDARD (which doesn't allow
    // whitespace) but strip newlines first for determinism.
    let cleaned: String = body.content.chars().filter(|c| !c.is_whitespace()).collect();
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(&cleaned)
        .map_err(ReadmeParseError::Base64)?;
    let text = String::from_utf8_lossy(&decoded).into_owned();
    Ok(extract_excerpts(&text, README_EXCERPT_CAP_BYTES))
}

/// Split README text into paragraph-shaped excerpts up to byte cap.
/// Paragraph = run of non-empty lines separated by a blank line.
/// Lines within a paragraph are joined with a single space (so a
/// 3-line paragraph "alpha\nbeta\ngamma" becomes "alpha beta
/// gamma"); we keep paragraphs in source order. Cap is enforced
/// *after* including the next paragraph, so a single paragraph
/// longer than the cap is included and we stop.
///
/// The excerpts are NOT byte-for-byte slices of `text` — interior
/// newlines and indentation collapse to spaces. If a future
/// caller needs byte-precision into the original README, this
/// function needs to grow `(byte_offset, byte_length)` outputs
/// alongside the joined string.
pub(crate) fn extract_excerpts(text: &str, cap_bytes: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut bytes_used: usize = 0;
    let mut current = String::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            if !current.is_empty() {
                let p = std::mem::take(&mut current).trim().to_string();
                bytes_used += p.len();
                out.push(p);
                if bytes_used >= cap_bytes {
                    return out;
                }
            }
        } else {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(line.trim());
        }
    }
    if !current.is_empty() {
        out.push(current.trim().to_string());
    }
    out
}

/// Parse the /repos/{owner}/{repo}/commits response (array of commit
/// objects). Returns up to `RECENT_COMMITS_CAP` verbatim commit
/// messages in API order (most-recent first).
pub fn parse_commits(json: &str) -> serde_json::Result<Vec<String>> {
    #[derive(Deserialize)]
    struct CommitOuter {
        commit: CommitInner,
    }
    #[derive(Deserialize)]
    struct CommitInner {
        message: String,
    }
    let arr: Vec<CommitOuter> = serde_json::from_str(json)?;
    Ok(arr
        .into_iter()
        .take(RECENT_COMMITS_CAP)
        .map(|c| c.commit.message)
        .collect())
}

/// Parse the /repos/{owner}/{repo}/languages response. The endpoint
/// returns an object with language → bytes; we sort by bytes desc
/// so the on-disk format is deterministic.
pub fn parse_languages(json: &str) -> serde_json::Result<Vec<(String, u64)>> {
    let map: BTreeMap<String, u64> = serde_json::from_str(json)?;
    let mut v: Vec<(String, u64)> = map.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    Ok(v)
}

/// Compose a `RepoSnapshot` from one /users/.../repos array element
/// plus the per-repo readme/commits/languages results. Pure function;
/// no network. Tested in isolation with fixture JSON.
pub fn to_snapshot(
    api: RepoApiResponse,
    readme_excerpts: Vec<String>,
    recent_commit_messages: Vec<String>,
    languages: Vec<(String, u64)>,
) -> RepoSnapshot {
    RepoSnapshot {
        owner: api.owner.login,
        name: api.name,
        description: api.description.unwrap_or_default(),
        primary_language: api.language.unwrap_or_default(),
        languages,
        stars: api.stargazers_count,
        forks: api.forks_count,
        open_issues: api.open_issues_count,
        // closed_issues isn't in the /repos endpoint; deferred until
        // /search/issues integration lands. 0 for now.
        closed_issues: 0,
        last_commit: api.pushed_at.unwrap_or_default(),
        // total_commits has no cheap GitHub endpoint (would require
        // /repos/{o}/{r}/contributors with anon=true OR Link-header
        // pagination of /commits). Use the count of recent_commit_
        // messages as a lower bound; document the limitation.
        total_commits: recent_commit_messages.len() as u32,
        topics: api.topics,
        license: api
            .license
            .and_then(|l| l.spdx_id)
            .unwrap_or_default(),
        is_fork: api.fork,
        readme_excerpts,
        recent_commit_messages,
        // top_files requires /git/trees recursive walk; deferred.
        top_files: Vec::new(),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReadmeParseError {
    #[error("readme json parse: {0}")]
    Json(serde_json::Error),
    #[error("readme base64 decode: {0}")]
    Base64(base64::DecodeError),
    #[error("readme encoding {0:?} not supported (expected base64)")]
    UnknownEncoding(String),
}

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON parse: {0}")]
    Json(#[from] serde_json::Error),
    #[error("readme parse: {0}")]
    Readme(#[from] ReadmeParseError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

// ─── HTTP fetcher (live network; orchestrator) ─────────────────────────────

/// Build a configured async HTTP client. Sets the User-Agent and a
/// 30-second per-request timeout.
fn build_client() -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(30))
        .build()
}

/// GET helper that returns the response body as a String. Sets the
/// `Accept: application/vnd.github+json` header and the optional
/// `Authorization: Bearer <token>` when provided.
async fn get_text(
    client: &reqwest::Client,
    url: &str,
    token: Option<&str>,
) -> reqwest::Result<String> {
    let mut req = client
        .get(url)
        .header("Accept", "application/vnd.github+json");
    if let Some(t) = token {
        req = req.header("Authorization", format!("Bearer {t}"));
    }
    let resp = req.send().await?.error_for_status()?;
    resp.text().await
}

/// Orchestrator: sync the inventory for `handle`. Calls the user
/// endpoint, then the repos endpoint, then for the top-N
/// most-starred repos calls /readme + /commits + /languages.
/// Returns a fresh `GithubInventory` ready to save.
///
/// On any individual repo fetch error (e.g. 404 on /readme for a
/// repo with no README), that one repo's deep data is left empty
/// but the sync continues. Hard errors (network failure, auth)
/// surface as `SyncError`.
pub async fn sync_user_inventory(
    handle: &str,
    token: Option<&str>,
) -> Result<GithubInventory, SyncError> {
    let client = build_client()?;
    let now = chrono_rfc3339_now();

    // 1. user info
    let user_url = format!("{GITHUB_API_BASE}/users/{handle}");
    let user_text = get_text(&client, &user_url, token).await?;
    let user = parse_user_info(&user_text)?;

    // 2. repos (paginated; up to 100/page, GitHub default).
    // Only fetches first page for now — most users have fewer than
    // 100 repos, and rate-limit politeness matters for anonymous
    // mode. Future: paginate via Link header when the user has
    // 100+ public repos.
    let repos_url = format!(
        "{GITHUB_API_BASE}/users/{handle}/repos?per_page=100&sort=updated&type=owner"
    );
    let repos_text = get_text(&client, &repos_url, token).await?;
    let mut repos_api = parse_repos_array(&repos_text)?;

    // 3. sort by stars desc, deep-fetch top N
    repos_api.sort_by(|a, b| b.stargazers_count.cmp(&a.stargazers_count));
    let mut public_repos: Vec<RepoSnapshot> = Vec::with_capacity(repos_api.len());
    for (i, r) in repos_api.into_iter().enumerate() {
        let owner = r.owner.login.clone();
        let name = r.name.clone();
        let (readme, commits, languages) = if i < TOP_N_REPOS_FOR_DEEP_FETCH {
            let readme = fetch_repo_readme(&client, &owner, &name, token).await
                .unwrap_or_default();
            let commits = fetch_repo_commits(&client, &owner, &name, token).await
                .unwrap_or_default();
            let languages = fetch_repo_languages(&client, &owner, &name, token).await
                .unwrap_or_default();
            (readme, commits, languages)
        } else {
            (Vec::new(), Vec::new(), Vec::new())
        };
        public_repos.push(to_snapshot(r, readme, commits, languages));
    }

    let mut inv = GithubInventory {
        handle: user.login,
        last_synced: now,
        public_repos,
        contributed_to: Vec::new(),
        language_distribution: BTreeMap::new(),
        total_public_commits: 0,
        follower_count: user.followers,
        starred_count: 0,
    };
    inv.recompute_aggregates();
    Ok(inv)
}

/// Deep-fetch helpers — Ok(empty) on per-call failure so a missing
/// README on one repo doesn't tank the whole sync. The caller logs
/// these but doesn't surface them as hard errors.
async fn fetch_repo_readme(
    client: &reqwest::Client,
    owner: &str,
    name: &str,
    token: Option<&str>,
) -> Result<Vec<String>, SyncError> {
    let url = format!("{GITHUB_API_BASE}/repos/{owner}/{name}/readme");
    let text = get_text(client, &url, token).await?;
    Ok(parse_readme_excerpts(&text)?)
}

async fn fetch_repo_commits(
    client: &reqwest::Client,
    owner: &str,
    name: &str,
    token: Option<&str>,
) -> Result<Vec<String>, SyncError> {
    let url =
        format!("{GITHUB_API_BASE}/repos/{owner}/{name}/commits?per_page={RECENT_COMMITS_CAP}");
    let text = get_text(client, &url, token).await?;
    Ok(parse_commits(&text)?)
}

async fn fetch_repo_languages(
    client: &reqwest::Client,
    owner: &str,
    name: &str,
    token: Option<&str>,
) -> Result<Vec<(String, u64)>, SyncError> {
    let url = format!("{GITHUB_API_BASE}/repos/{owner}/{name}/languages");
    let text = get_text(client, &url, token).await?;
    Ok(parse_languages(&text)?)
}

impl GithubInventory {
    /// Recompute language_distribution + total_public_commits from
    /// the per-repo data. Called by sync_user_inventory after the
    /// top-N deep fetch lands.
    pub fn recompute_aggregates(&mut self) {
        let mut totals: BTreeMap<String, u64> = BTreeMap::new();
        let mut grand: u64 = 0;
        for r in &self.public_repos {
            for (lang, bytes) in &r.languages {
                *totals.entry(lang.clone()).or_insert(0) += bytes;
                grand += bytes;
            }
        }
        let mut dist: BTreeMap<String, f32> = BTreeMap::new();
        if grand > 0 {
            for (lang, bytes) in totals {
                dist.insert(lang, (bytes as f64 / grand as f64) as f32);
            }
        }
        self.language_distribution = dist;
        self.total_public_commits =
            self.public_repos.iter().map(|r| r.total_commits).sum();
    }
}

/// Best-effort RFC3339 timestamp without pulling in chrono. We use
/// the system clock + a hand-formatted "YYYY-MM-DDTHH:MM:SSZ" via
/// the SystemTime → seconds-since-epoch path. Good enough for an
/// audit timestamp; not authoritative.
pub(crate) fn chrono_rfc3339_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    // Calendar math: days since 1970-01-01.
    let (y, m, d, hh, mm, ss) = epoch_seconds_to_ymdhms(secs);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, m, d, hh, mm, ss)
}

pub(crate) fn epoch_seconds_to_ymdhms(secs: u64) -> (u32, u32, u32, u32, u32, u32) {
    let days = (secs / 86400) as i64;
    let rem = (secs % 86400) as u32;
    let hh = rem / 3600;
    let mm = (rem % 3600) / 60;
    let ss = rem % 60;
    // Rata Die / civil_from_days (Howard Hinnant) — well-known
    // public-domain epoch-to-ymd algorithm.
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = y + if m <= 2 { 1 } else { 0 };
    (y as u32, m, d, hh, mm, ss)
}


