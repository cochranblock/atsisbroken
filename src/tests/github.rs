// SPDX-License-Identifier: Unlicense

//! Tests for `crate::github` (Phase 7).

use std::collections::BTreeMap;

use crate::github::{
    chrono_rfc3339_now, epoch_seconds_to_ymdhms, extract_excerpts, load_github_token,
    parse_commits, parse_languages, parse_readme_excerpts, parse_repos_array, parse_user_info,
    save_github_token, to_snapshot, FileSummary, GithubInventory, License, ReadmeParseError,
    RepoApiResponse, RepoOwner, RepoSnapshot, RECENT_COMMITS_CAP,
};
use crate::{paths, RepoRef};

use super::{case, check, check_eq, TestResult};

pub fn run() -> Vec<TestResult> {
    let mut out = vec![
        case("github::github_inventory_default_is_empty", github_inventory_default_is_empty),
        case("github::github_inventory_default_json_shape_is_stable",
             github_inventory_default_json_shape_is_stable),
        case("github::repo_snapshot_default_json_shape_is_stable",
             repo_snapshot_default_json_shape_is_stable),
        case("github::file_summary_round_trip", file_summary_round_trip),
        case("github::github_inventory_full_round_trip", github_inventory_full_round_trip),
        case("github::load_missing_file_yields_default", load_missing_file_yields_default),
        case("github::save_then_load_round_trip", save_then_load_round_trip),
        case("github::save_atomically_via_tmp_rename", save_atomically_via_tmp_rename),
        case("github::save_overwrites_prior_content", save_overwrites_prior_content),
        case("github::load_corrupt_json_fails_loudly", load_corrupt_json_fails_loudly),
        case("github::save_token_then_load_round_trip", save_token_then_load_round_trip),
        case("github::load_token_missing_file_yields_none", load_token_missing_file_yields_none),
        case("github::save_token_trims_whitespace", save_token_trims_whitespace),
        case("github::load_token_treats_whitespace_only_as_none",
             load_token_treats_whitespace_only_as_none),
        case("github::paths_module_resolves_canonical_locations",
             paths_module_resolves_canonical_locations),
        case("github::parse_user_info_extracts_login_and_counts",
             parse_user_info_extracts_login_and_counts),
        case("github::parse_repos_array_extracts_subset_fields",
             parse_repos_array_extracts_subset_fields),
        case("github::parse_repos_array_empty_returns_empty_vec",
             parse_repos_array_empty_returns_empty_vec),
        case("github::parse_readme_excerpts_decodes_base64_paragraphs",
             parse_readme_excerpts_decodes_base64_paragraphs),
        case("github::parse_readme_excerpts_rejects_unknown_encoding",
             parse_readme_excerpts_rejects_unknown_encoding),
        case("github::extract_excerpts_respects_byte_cap", extract_excerpts_respects_byte_cap),
        case("github::parse_commits_extracts_messages_in_order",
             parse_commits_extracts_messages_in_order),
        case("github::parse_commits_caps_at_recent_limit", parse_commits_caps_at_recent_limit),
        case("github::parse_languages_returns_sorted_descending",
             parse_languages_returns_sorted_descending),
        case("github::parse_languages_empty_object_yields_empty_vec",
             parse_languages_empty_object_yields_empty_vec),
        case("github::parse_languages_ties_break_alphabetically",
             parse_languages_ties_break_alphabetically),
        case("github::to_snapshot_composes_repo_with_all_inputs",
             to_snapshot_composes_repo_with_all_inputs),
        case("github::to_snapshot_handles_missing_optional_fields",
             to_snapshot_handles_missing_optional_fields),
        case("github::recompute_aggregates_sums_languages_and_commits",
             recompute_aggregates_sums_languages_and_commits),
        case("github::recompute_aggregates_zero_bytes_yields_empty_distribution",
             recompute_aggregates_zero_bytes_yields_empty_distribution),
        case("github::epoch_seconds_to_ymdhms_unix_epoch", epoch_seconds_to_ymdhms_unix_epoch),
        case("github::epoch_seconds_to_ymdhms_known_date", epoch_seconds_to_ymdhms_known_date),
        case("github::rfc3339_now_has_expected_shape", rfc3339_now_has_expected_shape),
    ];
    #[cfg(unix)]
    {
        out.push(case("github::save_token_sets_mode_0600", save_token_sets_mode_0600));
        out.push(case(
            "github::save_token_overwrite_preserves_chmod_0600",
            save_token_overwrite_preserves_chmod_0600,
        ));
    }
    out
}

fn unique_tmpdir(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "atsisbroken_github_{}_{}",
        tag,
        std::process::id()
    ))
}

fn github_inventory_default_is_empty() -> Result<(), String> {
    let g = GithubInventory::default();
    check(g.handle.is_empty(), "handle empty")?;
    check(g.last_synced.is_empty(), "last_synced empty")?;
    check(g.public_repos.is_empty(), "public_repos empty")?;
    check(g.contributed_to.is_empty(), "contributed_to empty")?;
    check(g.language_distribution.is_empty(), "language_distribution empty")?;
    check_eq(g.total_public_commits, 0u32, "total_public_commits")?;
    check_eq(g.follower_count, 0u32, "follower_count")?;
    check_eq(g.starred_count, 0u32, "starred_count")
}

fn github_inventory_default_json_shape_is_stable() -> Result<(), String> {
    let g = GithubInventory::default();
    let got = serde_json::to_string(&g).map_err(|e| format!("{e}"))?;
    let want = r#"{"handle":"","last_synced":"","public_repos":[],"contributed_to":[],"language_distribution":{},"total_public_commits":0,"follower_count":0,"starred_count":0}"#;
    check_eq(got, want.to_string(), "default json shape")
}

fn repo_snapshot_default_json_shape_is_stable() -> Result<(), String> {
    let r = RepoSnapshot::default();
    let got = serde_json::to_string(&r).map_err(|e| format!("{e}"))?;
    let want = r#"{"owner":"","name":"","description":"","primary_language":"","languages":[],"stars":0,"forks":0,"open_issues":0,"closed_issues":0,"last_commit":"","total_commits":0,"topics":[],"license":"","is_fork":false,"readme_excerpts":[],"recent_commit_messages":[],"top_files":[]}"#;
    check_eq(got, want.to_string(), "default json shape")
}

fn file_summary_round_trip() -> Result<(), String> {
    let f = FileSummary {
        path: "src/lib.rs".into(),
        language: "Rust".into(),
        lines: 1234,
    };
    let s = serde_json::to_string(&f).map_err(|e| format!("{e}"))?;
    let back: FileSummary = serde_json::from_str(&s).map_err(|e| format!("{e}"))?;
    check_eq(f, back, "round-trip")
}

fn github_inventory_full_round_trip() -> Result<(), String> {
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
    let s = serde_json::to_string(&g).map_err(|e| format!("{e}"))?;
    let back: GithubInventory = serde_json::from_str(&s).map_err(|e| format!("{e}"))?;
    check_eq(g, back, "full round-trip")
}

fn load_missing_file_yields_default() -> Result<(), String> {
    let dir = unique_tmpdir("load_missing");
    let path = dir.join("nope.json");
    let g = GithubInventory::load_from(&path).map_err(|e| format!("{e}"))?;
    check_eq(g, GithubInventory::default(), "missing file → default")
}

fn save_then_load_round_trip() -> Result<(), String> {
    let dir = unique_tmpdir("save_load");
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {e}"))?;
    let path = dir.join("inventory.json");
    let g = GithubInventory {
        handle: "GotEmCoach".into(),
        last_synced: "2026-05-06T00:00:00Z".into(),
        ..Default::default()
    };
    g.save_to(&path).map_err(|e| format!("save: {e}"))?;
    let back = GithubInventory::load_from(&path).map_err(|e| format!("load: {e}"))?;
    check_eq(g, back, "save/load round-trip")?;
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

fn save_atomically_via_tmp_rename() -> Result<(), String> {
    let dir = unique_tmpdir("atomic");
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {e}"))?;
    let path = dir.join("inventory.json");
    GithubInventory::default().save_to(&path).map_err(|e| format!("save: {e}"))?;
    let tmp = path.with_extension("json.tmp");
    check(!tmp.exists(), ".tmp must be renamed away")?;
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

fn save_overwrites_prior_content() -> Result<(), String> {
    let dir = unique_tmpdir("overwrite");
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {e}"))?;
    let path = dir.join("inventory.json");
    let g1 = GithubInventory {
        handle: "first".into(),
        ..Default::default()
    };
    g1.save_to(&path).map_err(|e| format!("save1: {e}"))?;
    let g2 = GithubInventory {
        handle: "second".into(),
        ..Default::default()
    };
    g2.save_to(&path).map_err(|e| format!("save2: {e}"))?;
    let back = GithubInventory::load_from(&path).map_err(|e| format!("load: {e}"))?;
    check_eq(back.handle, "second".to_string(), "overwrite")?;
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

fn load_corrupt_json_fails_loudly() -> Result<(), String> {
    let dir = unique_tmpdir("corrupt");
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {e}"))?;
    let path = dir.join("inventory.json");
    std::fs::write(&path, "{ not json }").map_err(|e| format!("write: {e}"))?;
    let result = GithubInventory::load_from(&path);
    check(result.is_err(), "corrupt json should error")?;
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

fn save_token_then_load_round_trip() -> Result<(), String> {
    let dir = unique_tmpdir("token_rt");
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {e}"))?;
    let path = dir.join("github_token");
    save_github_token("ghp_examplePAT123", &path).map_err(|e| format!("save: {e}"))?;
    let back = load_github_token(&path).map_err(|e| format!("load: {e}"))?;
    check_eq(back, Some("ghp_examplePAT123".to_string()), "token round-trip")?;
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

fn load_token_missing_file_yields_none() -> Result<(), String> {
    let dir = unique_tmpdir("token_miss");
    let path = dir.join("nope");
    let t = load_github_token(&path).map_err(|e| format!("{e}"))?;
    check(t.is_none(), "missing file → None")
}

fn save_token_trims_whitespace() -> Result<(), String> {
    let dir = unique_tmpdir("token_trim");
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {e}"))?;
    let path = dir.join("github_token");
    save_github_token("\n  ghp_token  \n\n", &path).map_err(|e| format!("save: {e}"))?;
    let back = std::fs::read_to_string(&path).map_err(|e| format!("read: {e}"))?;
    check_eq(back, "ghp_token".to_string(), "trimmed token")?;
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

fn load_token_treats_whitespace_only_as_none() -> Result<(), String> {
    let dir = unique_tmpdir("token_blank");
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {e}"))?;
    let path = dir.join("github_token");
    std::fs::write(&path, "   \n  \n").map_err(|e| format!("write: {e}"))?;
    let t = load_github_token(&path).map_err(|e| format!("{e}"))?;
    check(t.is_none(), "whitespace-only → None")?;
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

#[cfg(unix)]
fn save_token_sets_mode_0600() -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let dir = unique_tmpdir("token_chmod");
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {e}"))?;
    let path = dir.join("github_token");
    save_github_token("ghp_secret", &path).map_err(|e| format!("save: {e}"))?;
    let perms = std::fs::metadata(&path).map_err(|e| format!("meta: {e}"))?.permissions();
    check_eq(perms.mode() & 0o777, 0o600, "perms 0o600")?;
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

#[cfg(unix)]
fn save_token_overwrite_preserves_chmod_0600() -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let dir = unique_tmpdir("token_rotate");
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {e}"))?;
    let path = dir.join("github_token");
    save_github_token("ghp_first", &path).map_err(|e| format!("save1: {e}"))?;
    save_github_token("ghp_second", &path).map_err(|e| format!("save2: {e}"))?;
    let perms = std::fs::metadata(&path).map_err(|e| format!("meta: {e}"))?.permissions();
    check_eq(perms.mode() & 0o777, 0o600, "perms 0o600 preserved")?;
    let back = load_github_token(&path).map_err(|e| format!("load: {e}"))?;
    check_eq(back, Some("ghp_second".to_string()), "rotated token")?;
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

fn paths_module_resolves_canonical_locations() -> Result<(), String> {
    let inv = paths::github_inventory_path();
    let tok = paths::github_token_path();
    check(inv.starts_with(paths::atsisbroken_dir()), "inventory under root")?;
    check(tok.starts_with(paths::atsisbroken_dir()), "token under root")?;
    check_eq(
        inv.file_name().and_then(|s| s.to_str()),
        Some("github_inventory.json"),
        "inventory filename",
    )?;
    check_eq(
        tok.file_name().and_then(|s| s.to_str()),
        Some("github_token"),
        "token filename",
    )
}

fn parse_user_info_extracts_login_and_counts() -> Result<(), String> {
    let json = r#"{
        "login": "GotEmCoach",
        "id": 12345,
        "node_id": "MDQ6VXNlcjEy",
        "public_repos": 42,
        "public_gists": 3,
        "followers": 7,
        "following": 5,
        "created_at": "2018-01-01T00:00:00Z"
    }"#;
    let u = parse_user_info(json).map_err(|e| format!("{e}"))?;
    check_eq(u.login, "GotEmCoach".to_string(), "login")?;
    check_eq(u.public_repos, 42u32, "public_repos")?;
    check_eq(u.followers, 7u32, "followers")
}

fn parse_repos_array_extracts_subset_fields() -> Result<(), String> {
    let json = r#"[
        {
            "name": "atsisbroken",
            "owner": {"login": "cochranblock", "id": 1},
            "html_url": "https://github.com/cochranblock/atsisbroken",
            "description": "ATS is broken.",
            "language": "Rust",
            "stargazers_count": 12,
            "forks_count": 1,
            "open_issues_count": 0,
            "pushed_at": "2026-05-06T11:00:00Z",
            "topics": ["ats", "automation"],
            "license": {"key": "unlicense", "spdx_id": "Unlicense"},
            "fork": false
        },
        {
            "name": "fork-of-something",
            "owner": {"login": "cochranblock"},
            "html_url": "https://github.com/cochranblock/fork-of-something",
            "description": null,
            "language": null,
            "stargazers_count": 0,
            "forks_count": 0,
            "open_issues_count": 0,
            "topics": [],
            "license": null,
            "fork": true
        }
    ]"#;
    let repos = parse_repos_array(json).map_err(|e| format!("{e}"))?;
    check_eq(repos.len(), 2usize, "repos count")?;
    check_eq(repos[0].name.clone(), "atsisbroken".to_string(), "name")?;
    check_eq(repos[0].stargazers_count, 12u32, "stargazers")?;
    check_eq(
        repos[0].topics.clone(),
        vec!["ats".to_string(), "automation".to_string()],
        "topics",
    )?;
    check_eq(
        repos[0].license.as_ref().and_then(|l| l.spdx_id.as_deref()),
        Some("Unlicense"),
        "license",
    )?;
    check(repos[1].fork, "fork=true")?;
    check(repos[1].description.is_none(), "description None")?;
    check(repos[1].language.is_none(), "language None")
}

fn parse_repos_array_empty_returns_empty_vec() -> Result<(), String> {
    let v = parse_repos_array("[]").map_err(|e| format!("{e}"))?;
    check(v.is_empty(), "empty array → empty vec")
}

fn parse_readme_excerpts_decodes_base64_paragraphs() -> Result<(), String> {
    use base64::Engine;
    let body = "# atsisbroken\n\nATS is broken.\n\nA Rust autopilot.";
    let b64 = base64::engine::general_purpose::STANDARD.encode(body);
    let wrapped: String = b64
        .chars()
        .enumerate()
        .flat_map(|(i, c)| {
            if i > 0 && i % 60 == 0 {
                vec!['\n', c]
            } else {
                vec![c]
            }
        })
        .collect();
    let json = serde_json::json!({
        "content": wrapped,
        "encoding": "base64"
    })
    .to_string();
    let excerpts = parse_readme_excerpts(&json).map_err(|e| format!("{e}"))?;
    check_eq(excerpts.len(), 3usize, "excerpts count")?;
    check(excerpts[0].contains("atsisbroken"), "first contains atsisbroken")?;
    check(excerpts[1].contains("broken"), "second contains broken")?;
    check(excerpts[2].contains("Rust"), "third contains Rust")
}

fn parse_readme_excerpts_rejects_unknown_encoding() -> Result<(), String> {
    let json = r#"{"content": "irrelevant", "encoding": "rot13"}"#;
    let r = parse_readme_excerpts(json);
    check(
        matches!(r, Err(ReadmeParseError::UnknownEncoding(_))),
        "unknown encoding rejected",
    )
}

fn extract_excerpts_respects_byte_cap() -> Result<(), String> {
    let text = "First paragraph here.\n\nSecond paragraph extends much longer than thirty bytes total.\n\nThird paragraph never reached.";
    let cap = 30;
    let out = extract_excerpts(text, cap);
    check_eq(out.len(), 2usize, "excerpts count")?;
    check(out[0].starts_with("First"), "first starts with First")?;
    check(out[1].starts_with("Second"), "second starts with Second")
}

fn parse_commits_extracts_messages_in_order() -> Result<(), String> {
    let json = r#"[
        {"sha": "a1b2", "commit": {"message": "fix: edge case", "author": {"name": "Jane"}}},
        {"sha": "c3d4", "commit": {"message": "refactor: simplify run loop"}},
        {"sha": "e5f6", "commit": {"message": "docs: PROOF_OF_ARTIFACTS"}}
    ]"#;
    let msgs = parse_commits(json).map_err(|e| format!("{e}"))?;
    check_eq(msgs.len(), 3usize, "commits count")?;
    check_eq(msgs[0].clone(), "fix: edge case".to_string(), "msg 0")?;
    check_eq(msgs[1].clone(), "refactor: simplify run loop".to_string(), "msg 1")?;
    check_eq(msgs[2].clone(), "docs: PROOF_OF_ARTIFACTS".to_string(), "msg 2")
}

fn parse_commits_caps_at_recent_limit() -> Result<(), String> {
    let mut entries: Vec<String> = Vec::new();
    for i in 0..(RECENT_COMMITS_CAP + 5) {
        entries.push(format!(
            r#"{{"sha":"sha{i}","commit":{{"message":"msg {i}"}}}}"#
        ));
    }
    let json = format!("[{}]", entries.join(","));
    let msgs = parse_commits(&json).map_err(|e| format!("{e}"))?;
    check_eq(msgs.len(), RECENT_COMMITS_CAP, "capped at limit")?;
    check_eq(msgs[0].clone(), "msg 0".to_string(), "first")?;
    check_eq(
        msgs[RECENT_COMMITS_CAP - 1].clone(),
        format!("msg {}", RECENT_COMMITS_CAP - 1),
        "last",
    )
}

fn parse_languages_returns_sorted_descending() -> Result<(), String> {
    let json = r#"{"Rust": 10000, "HTML": 500, "JavaScript": 2000}"#;
    let langs = parse_languages(json).map_err(|e| format!("{e}"))?;
    check_eq(
        langs,
        vec![
            ("Rust".to_string(), 10000),
            ("JavaScript".to_string(), 2000),
            ("HTML".to_string(), 500),
        ],
        "sorted desc",
    )
}

fn parse_languages_empty_object_yields_empty_vec() -> Result<(), String> {
    let v = parse_languages("{}").map_err(|e| format!("{e}"))?;
    check(v.is_empty(), "empty → empty")
}

fn parse_languages_ties_break_alphabetically() -> Result<(), String> {
    let json = r#"{"Zig": 1000, "Awk": 1000, "Rust": 1000}"#;
    let langs = parse_languages(json).map_err(|e| format!("{e}"))?;
    check_eq(
        langs,
        vec![
            ("Awk".to_string(), 1000),
            ("Rust".to_string(), 1000),
            ("Zig".to_string(), 1000),
        ],
        "alphabetical tie-break",
    )
}

fn to_snapshot_composes_repo_with_all_inputs() -> Result<(), String> {
    let api = RepoApiResponse {
        name: "atsisbroken".to_string(),
        owner: RepoOwner { login: "cochranblock".to_string() },
        html_url: "https://github.com/cochranblock/atsisbroken".to_string(),
        description: Some("ATS is broken.".to_string()),
        language: Some("Rust".to_string()),
        stargazers_count: 12,
        forks_count: 1,
        open_issues_count: 0,
        pushed_at: Some("2026-05-06T11:00:00Z".to_string()),
        topics: vec!["ats".to_string()],
        license: Some(License { spdx_id: Some("Unlicense".to_string()) }),
        fork: false,
    };
    let readme = vec!["ATS is broken.".to_string()];
    let commits = vec!["Phase G done".to_string(), "Phase I.a done".to_string()];
    let languages = vec![("Rust".to_string(), 12345)];
    let snap = to_snapshot(api, readme.clone(), commits.clone(), languages.clone());
    check_eq(snap.owner.clone(), "cochranblock".to_string(), "owner")?;
    check_eq(snap.name.clone(), "atsisbroken".to_string(), "name")?;
    check_eq(snap.description.clone(), "ATS is broken.".to_string(), "description")?;
    check_eq(snap.primary_language.clone(), "Rust".to_string(), "primary_language")?;
    check_eq(snap.languages.clone(), languages, "languages")?;
    check_eq(snap.stars, 12u32, "stars")?;
    check_eq(snap.last_commit.clone(), "2026-05-06T11:00:00Z".to_string(), "last_commit")?;
    check_eq(snap.license.clone(), "Unlicense".to_string(), "license")?;
    check_eq(snap.readme_excerpts.clone(), readme, "readme_excerpts")?;
    check_eq(snap.recent_commit_messages.clone(), commits, "recent_commit_messages")?;
    check_eq(snap.closed_issues, 0u32, "closed_issues stub")?;
    check_eq(snap.total_commits, 2u32, "total_commits derived")?;
    check(snap.top_files.is_empty(), "top_files empty stub")
}

fn to_snapshot_handles_missing_optional_fields() -> Result<(), String> {
    let api = RepoApiResponse {
        name: "minimal".to_string(),
        owner: RepoOwner { login: "x".to_string() },
        html_url: "https://github.com/x/minimal".to_string(),
        description: None,
        language: None,
        stargazers_count: 0,
        forks_count: 0,
        open_issues_count: 0,
        pushed_at: None,
        topics: vec![],
        license: None,
        fork: false,
    };
    let snap = to_snapshot(api, vec![], vec![], vec![]);
    check_eq(snap.description.clone(), "".to_string(), "description")?;
    check_eq(snap.primary_language.clone(), "".to_string(), "primary_language")?;
    check_eq(snap.license.clone(), "".to_string(), "license")?;
    check_eq(snap.last_commit.clone(), "".to_string(), "last_commit")?;
    check(snap.topics.is_empty(), "topics empty")
}

fn recompute_aggregates_sums_languages_and_commits() -> Result<(), String> {
    let mut inv = GithubInventory {
        public_repos: vec![
            RepoSnapshot {
                languages: vec![("Rust".into(), 8000), ("HTML".into(), 2000)],
                total_commits: 30,
                ..Default::default()
            },
            RepoSnapshot {
                languages: vec![("Rust".into(), 2000), ("CSS".into(), 1000)],
                total_commits: 12,
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    inv.recompute_aggregates();
    let rust_share = inv.language_distribution.get("Rust").copied().unwrap_or(0.0);
    check(
        (rust_share - 10000.0 / 13000.0).abs() < 1e-3,
        format!("rust share: {rust_share}"),
    )?;
    check_eq(inv.total_public_commits, 42u32, "total_public_commits")
}

fn recompute_aggregates_zero_bytes_yields_empty_distribution() -> Result<(), String> {
    let mut inv = GithubInventory::default();
    inv.recompute_aggregates();
    check(inv.language_distribution.is_empty(), "empty distribution")
}

fn epoch_seconds_to_ymdhms_unix_epoch() -> Result<(), String> {
    let (y, m, d, hh, mm, ss) = epoch_seconds_to_ymdhms(0);
    check_eq((y, m, d, hh, mm, ss), (1970, 1, 1, 0, 0, 0), "unix epoch")
}

fn epoch_seconds_to_ymdhms_known_date() -> Result<(), String> {
    let (y, m, d, hh, mm, ss) = epoch_seconds_to_ymdhms(1778070896);
    check_eq((y, m, d), (2026, 5, 6), "ymd")?;
    check_eq((hh, mm, ss), (12, 34, 56), "hms")
}

fn rfc3339_now_has_expected_shape() -> Result<(), String> {
    let s = chrono_rfc3339_now();
    check_eq(s.len(), 20usize, "rfc3339 length")?;
    check(s.ends_with('Z'), "ends with Z")?;
    check(s.chars().all(|c| c.is_ascii()), "ascii only")?;
    check_eq(s.chars().nth(10), Some('T'), "T at position 10")
}
