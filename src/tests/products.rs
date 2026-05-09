// SPDX-License-Identifier: Unlicense

//! Tests for `crate::browser::products` — converted from
//! `#[cfg(test)] mod tests {}` to the cochranblock exopack
//! pattern (Phase 2). The atsisbroken-test binary calls
//! `super::run()` via [`super::run_all`].

use std::collections::BTreeMap;

use crate::browser::products::{
    Authorship, Excerpt, ExcerptKind, Product, ProductGraph, ProductId, ProductKind, ProductStats,
};
use crate::browser::ConnectorKind;

use super::{case, check, check_eq, TestResult};

pub fn run() -> Vec<TestResult> {
    vec![
        case("products::empty_graph_default_pins_current_schema_version",
             empty_graph_default_pins_current_schema_version),
        case("products::default_save_load_is_identity", default_save_load_is_identity),
        case("products::upsert_new_product_appends", upsert_new_product_appends),
        case("products::upsert_existing_product_replaces", upsert_existing_product_replaces),
        case("products::from_source_filters_by_connector", from_source_filters_by_connector),
        case("products::record_sync_and_lookup_round_trip", record_sync_and_lookup_round_trip),
        case("products::last_synced_serializes_with_canonical_wire_keys",
             last_synced_serializes_with_canonical_wire_keys),
        case("products::last_synced_round_trips_through_json", last_synced_round_trips_through_json),
        case("products::product_round_trips_through_json", product_round_trips_through_json),
        case("products::graph_save_load_round_trip", graph_save_load_round_trip),
        case("products::load_missing_file_returns_default", load_missing_file_returns_default),
        case("products::excerpt_text_round_trips", excerpt_text_round_trips),
        case("products::product_kind_serializes_snake_case", product_kind_serializes_snake_case),
        case("products::authorship_variants_round_trip", authorship_variants_round_trip),
    ]
}

fn sample_product() -> Product {
    Product {
        id: ProductId::new("github:cochranblock:atsisbroken"),
        source: ConnectorKind::GitHub,
        kind: ProductKind::Repo,
        title: "atsisbroken".into(),
        url: "https://github.com/cochranblock/atsisbroken".into(),
        published: "2026-04-01T00:00:00Z".into(),
        updated: "2026-05-09T12:00:00Z".into(),
        excerpts: vec![Excerpt {
            text: "ATS is broken.".into(),
            kind: ExcerptKind::Intro,
        }],
        topics: vec!["rust".into(), "ats".into()],
        stats: ProductStats {
            popularity: 42,
            ..Default::default()
        },
        authorship: Authorship::Sole,
        metadata: BTreeMap::new(),
    }
}

/// Per-test temp dir scoped to (process pid, test name) so two
/// tests in the same run don't collide on the same path.
fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "atsisbroken_pg_{}_{name}",
        std::process::id()
    ));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn empty_graph_default_pins_current_schema_version() -> Result<(), String> {
    // Manual Default impl makes ProductGraph::default() return
    // version = CURRENT_SCHEMA_VERSION (1), matching what
    // serde-default produces for old-shape JSON missing the
    // version field. The save → load round-trip is the identity
    // function on a default graph; it didn't used to be.
    let g = ProductGraph::default();
    check_eq(g.version, 1u32, "default version")?;

    // Save → load preserves version exactly.
    let json = serde_json::to_string(&g).map_err(|e| format!("{e}"))?;
    let back: ProductGraph = serde_json::from_str(&json).map_err(|e| format!("{e}"))?;
    check_eq(back, g.clone(), "round-trip identity")?;

    // v0-shaped JSON (no version field) still loads as v1 via
    // the serde-default fallback. Forward-compat for any pre-
    // versioned graph that was already on disk.
    let v0_json = r#"{"products":[],"last_synced":{}}"#;
    let v0_loaded: ProductGraph = serde_json::from_str(v0_json).map_err(|e| format!("{e}"))?;
    check_eq(v0_loaded.version, 1u32, "v0 fallback version")?;
    check_eq(v0_loaded, g, "v0 loads as default")?;
    Ok(())
}

fn default_save_load_is_identity() -> Result<(), String> {
    let g = ProductGraph::default();
    let dir = temp_dir("identity");
    let path = dir.join("identity.json");
    g.save_to(&path).map_err(|e| format!("save: {e}"))?;
    let loaded = ProductGraph::load_from(&path).map_err(|e| format!("load: {e}"))?;
    check_eq(loaded.clone(), g, "save→load identity")?;
    check_eq(loaded.version, 1u32, "loaded version")?;
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

fn upsert_new_product_appends() -> Result<(), String> {
    let mut g = ProductGraph::default();
    g.upsert(sample_product());
    check_eq(g.products.len(), 1usize, "products.len after upsert")
}

fn upsert_existing_product_replaces() -> Result<(), String> {
    let mut g = ProductGraph::default();
    g.upsert(sample_product());
    let mut updated = sample_product();
    updated.title = "atsisbroken (updated)".into();
    g.upsert(updated);
    check_eq(g.products.len(), 1usize, "products.len after replace")?;
    check_eq(
        g.products[0].title.clone(),
        "atsisbroken (updated)".to_string(),
        "title after replace",
    )
}

fn from_source_filters_by_connector() -> Result<(), String> {
    let mut g = ProductGraph::default();
    g.upsert(sample_product());
    let mut so = sample_product();
    so.id = ProductId::new("stackoverflow:user/123:answer/456");
    so.source = ConnectorKind::StackOverflow;
    g.upsert(so);
    let github_count = g.from_source(ConnectorKind::GitHub).count();
    let so_count = g.from_source(ConnectorKind::StackOverflow).count();
    check_eq(github_count, 1usize, "github count")?;
    check_eq(so_count, 1usize, "stackoverflow count")
}

fn record_sync_and_lookup_round_trip() -> Result<(), String> {
    let mut g = ProductGraph::default();
    g.record_sync(ConnectorKind::GitHub, "2026-05-09T13:00:00Z");
    check_eq(
        g.last_sync(ConnectorKind::GitHub),
        Some("2026-05-09T13:00:00Z"),
        "github lookup",
    )?;
    check_eq(g.last_sync(ConnectorKind::StackOverflow), None, "absent lookup")
}

fn last_synced_serializes_with_canonical_wire_keys() -> Result<(), String> {
    // The on-disk key is the connector's canonical wire name
    // (audit-fix-#1: `#[serde(rename = "github")]`), NOT the
    // Rust Debug format (`"GitHub"`).
    let mut g = ProductGraph::default();
    g.record_sync(ConnectorKind::GitHub, "2026-05-09T13:00:00Z");
    g.record_sync(ConnectorKind::StackOverflow, "2026-05-09T14:00:00Z");
    g.record_sync(ConnectorKind::NpmRegistry, "2026-05-09T15:00:00Z");
    let json = serde_json::to_string(&g).map_err(|e| format!("{e}"))?;
    check(
        json.contains("\"github\":"),
        format!("expected canonical 'github' key, got: {json}"),
    )?;
    check(json.contains("\"stackoverflow\":"), "expected 'stackoverflow' key")?;
    check(
        json.contains("\"npm\":"),
        "expected 'npm' key (NpmRegistry's rename)",
    )?;
    check(!json.contains("\"GitHub\":"), "old Debug-format 'GitHub' key leaked")?;
    check(!json.contains("\"NpmRegistry\":"), "Debug-format 'NpmRegistry' leaked")
}

fn last_synced_round_trips_through_json() -> Result<(), String> {
    let mut g = ProductGraph::default();
    g.record_sync(ConnectorKind::GitHub, "2026-05-09T13:00:00Z");
    g.record_sync(ConnectorKind::USPTO, "2026-05-09T14:00:00Z");
    let json = serde_json::to_string(&g).map_err(|e| format!("{e}"))?;
    let back: ProductGraph = serde_json::from_str(&json).map_err(|e| format!("{e}"))?;
    check_eq(
        back.last_sync(ConnectorKind::GitHub),
        Some("2026-05-09T13:00:00Z"),
        "github after round-trip",
    )?;
    check_eq(
        back.last_sync(ConnectorKind::USPTO),
        Some("2026-05-09T14:00:00Z"),
        "uspto after round-trip",
    )?;
    check_eq(back, g, "graph identity")
}

fn product_round_trips_through_json() -> Result<(), String> {
    let p = sample_product();
    let s = serde_json::to_string(&p).map_err(|e| format!("{e}"))?;
    let back: Product = serde_json::from_str(&s).map_err(|e| format!("{e}"))?;
    check_eq(p, back, "product identity")
}

fn graph_save_load_round_trip() -> Result<(), String> {
    let dir = temp_dir("savload");
    let path = dir.join("product_graph.json");
    let mut g = ProductGraph::default();
    g.upsert(sample_product());
    g.record_sync(ConnectorKind::GitHub, "2026-05-09T13:00:00Z");
    g.save_to(&path).map_err(|e| format!("save: {e}"))?;
    let back = ProductGraph::load_from(&path).map_err(|e| format!("load: {e}"))?;
    check_eq(g, back, "graph identity")?;
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

fn load_missing_file_returns_default() -> Result<(), String> {
    let path = std::env::temp_dir().join(format!(
        "atsisbroken_nonexistent_pg_{}.json",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path); // ensure absent
    let g = ProductGraph::load_from(&path).map_err(|e| format!("load: {e}"))?;
    check_eq(g, ProductGraph::default(), "missing file → default")
}

fn excerpt_text_round_trips() -> Result<(), String> {
    let p = sample_product();
    let s = serde_json::to_string(&p).map_err(|e| format!("{e}"))?;
    let back: Product = serde_json::from_str(&s).map_err(|e| format!("{e}"))?;
    check_eq(
        back.excerpts[0].text.clone(),
        "ATS is broken.".to_string(),
        "excerpt text",
    )?;
    check_eq(back.excerpts[0].kind, ExcerptKind::Intro, "excerpt kind")
}

fn product_kind_serializes_snake_case() -> Result<(), String> {
    let cases: &[(ProductKind, &str)] = &[
        (ProductKind::Repo, "\"repo\""),
        (ProductKind::CommitCluster, "\"commit_cluster\""),
        (ProductKind::ManualPaste, "\"manual_paste\""),
    ];
    for (kind, want) in cases {
        let got = serde_json::to_string(kind).map_err(|e| format!("{e}"))?;
        check_eq(got, want.to_string(), &format!("{kind:?} serialization"))?;
    }
    Ok(())
}

fn authorship_variants_round_trip() -> Result<(), String> {
    for a in [
        Authorship::Sole,
        Authorship::PrimaryWith {
            co_authors: vec!["Pat Doe".into()],
        },
        Authorship::Contributor {
            lead: Some("Acme Corp".into()),
        },
        Authorship::Contributor { lead: None },
        Authorship::Unknown,
    ] {
        let s = serde_json::to_string(&a).map_err(|e| format!("{e}"))?;
        let back: Authorship = serde_json::from_str(&s).map_err(|e| format!("{e}"))?;
        check_eq(a, back, "authorship variant round-trip")?;
    }
    Ok(())
}
