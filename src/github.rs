// SPDX-License-Identifier: Unlicense
// Unlicense — public domain — cochranblock.org
// Contributors: GotEmCoach, KOVA, Claude Opus 4.7

//! GitHub inventory — Phase I.
//!
//! Per-user snapshot of public-repo metadata, README excerpts, and
//! recent commit messages, used by Phase K's answer composer as a
//! verbatim source for free-form ATS prompts ("describe a project,"
//! "biggest technical challenge," etc.).
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
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

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

/// Per-repo metadata. The composer (Phase K) cites
/// (owner, name, commit_sha, readme_section) for every
/// answer it produces from this struct; every emitted token must
/// trace verbatim to one of `readme_excerpts`, `recent_commit_messages`,
/// or a structural fact (name / primary_language / stars).
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
    /// Verbatim sentences from README.md, capped at 2 KiB per repo.
    /// The composer quotes from these directly with citation. No
    /// paraphrasing; no model-generated text. Empty for repos with
    /// no README or with README content over the cap (truncated).
    #[serde(default)]
    pub readme_excerpts: Vec<String>,
    /// Recent commit messages, verbatim, capped at 50 entries.
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

// ─── tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_tmpdir(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "atsisbroken_github_{}_{}",
            tag,
            std::process::id()
        ))
    }

    #[test]
    fn github_inventory_default_is_empty() {
        let g = GithubInventory::default();
        assert!(g.handle.is_empty());
        assert!(g.last_synced.is_empty());
        assert!(g.public_repos.is_empty());
        assert!(g.contributed_to.is_empty());
        assert!(g.language_distribution.is_empty());
        assert_eq!(g.total_public_commits, 0);
        assert_eq!(g.follower_count, 0);
        assert_eq!(g.starred_count, 0);
    }

    #[test]
    fn github_inventory_default_json_shape_is_stable() {
        // The all-empty shape pins the wire format for first-run.
        // Adding a new field will FAIL this and force an explicit
        // migration decision.
        let g = GithubInventory::default();
        let got = serde_json::to_string(&g).unwrap();
        let want = r#"{"handle":"","last_synced":"","public_repos":[],"contributed_to":[],"language_distribution":{},"total_public_commits":0,"follower_count":0,"starred_count":0}"#;
        assert_eq!(got, want);
    }

    #[test]
    fn repo_snapshot_default_json_shape_is_stable() {
        let r = RepoSnapshot::default();
        let got = serde_json::to_string(&r).unwrap();
        let want = r#"{"owner":"","name":"","description":"","primary_language":"","languages":[],"stars":0,"forks":0,"open_issues":0,"closed_issues":0,"last_commit":"","total_commits":0,"topics":[],"license":"","is_fork":false,"readme_excerpts":[],"recent_commit_messages":[],"top_files":[]}"#;
        assert_eq!(got, want);
    }

    #[test]
    fn file_summary_round_trip() {
        let f = FileSummary {
            path: "src/lib.rs".into(),
            language: "Rust".into(),
            lines: 1234,
        };
        let s = serde_json::to_string(&f).unwrap();
        let back: FileSummary = serde_json::from_str(&s).unwrap();
        assert_eq!(f, back);
    }

    #[test]
    fn github_inventory_full_round_trip() {
        let mut lang = BTreeMap::new();
        lang.insert("Rust".to_string(), 0.82);
        lang.insert("TypeScript".to_string(), 0.18);
        let g = GithubInventory {
            handle: "GotEmCoach".into(),
            last_synced: "2026-05-06T12:34:56Z".into(),
            public_repos: vec![RepoSnapshot {
                owner: "cochranblock".into(),
                name: "atsisbroken".into(),
                description: "ATS is broken.".into(),
                primary_language: "Rust".into(),
                languages: vec![("Rust".into(), 12345), ("HTML".into(), 678)],
                stars: 42,
                forks: 3,
                open_issues: 1,
                closed_issues: 7,
                last_commit: "2026-05-06T11:00:00Z".into(),
                total_commits: 100,
                topics: vec!["ats".into(), "automation".into()],
                license: "Unlicense".into(),
                is_fork: false,
                readme_excerpts: vec!["ATS is broken.".into()],
                recent_commit_messages: vec!["Phase G done".into()],
                top_files: vec![FileSummary {
                    path: "src/lib.rs".into(),
                    language: "Rust".into(),
                    lines: 1500,
                }],
            }],
            contributed_to: vec![RepoRef {
                owner: "rust-lang".into(),
                name: "rust".into(),
                url: "https://github.com/rust-lang/rust".into(),
            }],
            language_distribution: lang,
            total_public_commits: 100,
            follower_count: 5,
            starred_count: 12,
        };
        let s = serde_json::to_string(&g).unwrap();
        let back: GithubInventory = serde_json::from_str(&s).unwrap();
        assert_eq!(g, back);
    }

    #[test]
    fn load_missing_file_yields_default() {
        let dir = unique_tmpdir("load_missing");
        let path = dir.join("nope.json");
        let g = GithubInventory::load_from(&path).unwrap();
        assert_eq!(g, GithubInventory::default());
    }

    #[test]
    fn save_then_load_round_trip() {
        let dir = unique_tmpdir("save_load");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("inventory.json");
        let g = GithubInventory {
            handle: "GotEmCoach".into(),
            last_synced: "2026-05-06T00:00:00Z".into(),
            ..Default::default()
        };
        g.save_to(&path).unwrap();
        let back = GithubInventory::load_from(&path).unwrap();
        assert_eq!(g, back);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_atomically_via_tmp_rename() {
        // After save, the .tmp file must not exist (it was renamed).
        // Same atomicity contract as FeedbackQueue.
        let dir = unique_tmpdir("atomic");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("inventory.json");
        GithubInventory::default().save_to(&path).unwrap();
        let tmp = path.with_extension("json.tmp");
        assert!(!tmp.exists(), ".tmp must be renamed away");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_overwrites_prior_content() {
        let dir = unique_tmpdir("overwrite");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("inventory.json");
        let g1 = GithubInventory {
            handle: "first".into(),
            ..Default::default()
        };
        g1.save_to(&path).unwrap();
        let g2 = GithubInventory {
            handle: "second".into(),
            ..Default::default()
        };
        g2.save_to(&path).unwrap();
        let back = GithubInventory::load_from(&path).unwrap();
        assert_eq!(back.handle, "second");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_corrupt_json_fails_loudly() {
        // A corrupt file means the user's connect-github state has
        // drifted; better to surface it than silently default.
        let dir = unique_tmpdir("corrupt");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("inventory.json");
        std::fs::write(&path, "{ not json }").unwrap();
        let result = GithubInventory::load_from(&path);
        assert!(result.is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_token_then_load_round_trip() {
        let dir = unique_tmpdir("token_rt");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("github_token");
        save_github_token("ghp_examplePAT123", &path).unwrap();
        let back = load_github_token(&path).unwrap();
        assert_eq!(back, Some("ghp_examplePAT123".to_string()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_token_missing_file_yields_none() {
        let dir = unique_tmpdir("token_miss");
        let path = dir.join("nope");
        let t = load_github_token(&path).unwrap();
        assert!(t.is_none());
    }

    #[test]
    fn save_token_trims_whitespace() {
        // Copy-paste from a website often has trailing newline; the
        // GitHub API rejects tokens with whitespace. Normalize on save.
        let dir = unique_tmpdir("token_trim");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("github_token");
        save_github_token("\n  ghp_token  \n\n", &path).unwrap();
        let back = std::fs::read_to_string(&path).unwrap();
        assert_eq!(back, "ghp_token");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_token_treats_whitespace_only_as_none() {
        // A file that exists but contains only whitespace should
        // behave like a missing file (no token), not like an
        // empty-string token (which would fail an API call).
        let dir = unique_tmpdir("token_blank");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("github_token");
        std::fs::write(&path, "   \n  \n").unwrap();
        let t = load_github_token(&path).unwrap();
        assert!(t.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn save_token_sets_mode_0600() {
        // Pinned: token file must be owner-only on Unix. A regression
        // here would let any local user (CI sandbox neighbor, group
        // member) read the user's PAT.
        use std::os::unix::fs::PermissionsExt;
        let dir = unique_tmpdir("token_chmod");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("github_token");
        save_github_token("ghp_secret", &path).unwrap();
        let perms = std::fs::metadata(&path).unwrap().permissions();
        // 0o600 = read+write for owner, nothing for group/other.
        // Mask off the file-type bits (S_IFMT) — only mode bits matter.
        assert_eq!(perms.mode() & 0o777, 0o600);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn save_token_overwrite_preserves_chmod_0600() {
        // A re-save after the user rotates their token must NOT loosen
        // permissions. The atomic rename + post-rename chmod handles
        // this; pinned so it can't drift.
        use std::os::unix::fs::PermissionsExt;
        let dir = unique_tmpdir("token_rotate");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("github_token");
        save_github_token("ghp_first", &path).unwrap();
        save_github_token("ghp_second", &path).unwrap();
        let perms = std::fs::metadata(&path).unwrap().permissions();
        assert_eq!(perms.mode() & 0o777, 0o600);
        let back = load_github_token(&path).unwrap();
        assert_eq!(back, Some("ghp_second".to_string()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn paths_module_resolves_canonical_locations() {
        // Sanity: canonical paths resolve under ~/.atsisbroken/ as
        // documented; renaming either helper would be a UX break for
        // existing users.
        let inv = paths::github_inventory_path();
        let tok = paths::github_token_path();
        assert!(inv.starts_with(paths::atsisbroken_dir()));
        assert!(tok.starts_with(paths::atsisbroken_dir()));
        assert_eq!(
            inv.file_name().and_then(|s| s.to_str()),
            Some("github_inventory.json")
        );
        assert_eq!(
            tok.file_name().and_then(|s| s.to_str()),
            Some("github_token")
        );
    }
}
