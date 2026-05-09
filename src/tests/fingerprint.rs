// SPDX-License-Identifier: Unlicense

//! Tests for `crate::browser::fingerprint` — converted from
//! `#[cfg(test)] mod tests {}` to the cochranblock exopack
//! pattern (Phase 2). The atsisbroken-test binary calls
//! `super::run()` via [`super::run_all`].

use crate::browser::fingerprint::{matches_glob, FingerprintOverrides, HostOverride};
use crate::browser::FingerprintProfile;

use super::{case, check, check_eq, TestResult};

pub fn run() -> Vec<TestResult> {
    vec![
        case("fingerprint::balanced_default_has_no_overrides", balanced_default_has_no_overrides),
        case("fingerprint::paranoid_sets_every_signal", paranoid_sets_every_signal),
        case("fingerprint::webdriver_default_false_for_every_preset",
             webdriver_default_false_for_every_preset),
        case("fingerprint::round_trips_through_toml", round_trips_through_toml),
        case("fingerprint::host_override_resolves_first_match", host_override_resolves_first_match),
        case("fingerprint::host_override_no_match_returns_none", host_override_no_match_returns_none),
        case("fingerprint::glob_matches_simple_cases", glob_matches_simple_cases),
        case("fingerprint::glob_edge_cases", glob_edge_cases),
        case("fingerprint::host_override_realistic_patterns", host_override_realistic_patterns),
    ]
}

fn balanced_default_has_no_overrides() -> Result<(), String> {
    let p = FingerprintProfile::balanced();
    check(p.user_agent.is_none(), "user_agent should be None")?;
    check(!p.navigator_webdriver, "navigator_webdriver should be false")?;
    check(!p.canvas_noise, "canvas_noise should be false")?;
    check(!p.webgl_noise, "webgl_noise should be false")
}

fn paranoid_sets_every_signal() -> Result<(), String> {
    let p = FingerprintProfile::paranoid();
    check(p.user_agent.is_some(), "user_agent should be set")?;
    check(p.navigator_platform.is_some(), "navigator_platform should be set")?;
    check(p.canvas_noise, "canvas_noise should be true")?;
    check(p.webgl_noise, "webgl_noise should be true")?;
    check_eq(
        p.languages,
        vec!["en-US".to_string(), "en".to_string()],
        "languages",
    )
}

fn webdriver_default_false_for_every_preset() -> Result<(), String> {
    // Pins the struct default across every preset. This is a
    // configuration-shape contract, not a runtime one — the JS
    // engine that would actually expose the value to a page
    // (mozjs) hasn't landed yet.
    for p in [
        FingerprintProfile::balanced(),
        FingerprintProfile::paranoid(),
        FingerprintProfile::natural(),
    ] {
        check(!p.navigator_webdriver, "preset default flipped")?;
    }
    Ok(())
}

fn round_trips_through_toml() -> Result<(), String> {
    let p = FingerprintProfile::paranoid();
    let toml_text = toml::to_string(&p).map_err(|e| format!("serialize: {e}"))?;
    let back: FingerprintProfile = toml::from_str(&toml_text).map_err(|e| format!("parse: {e}"))?;
    check_eq(p, back, "toml round-trip")
}

fn host_override_resolves_first_match() -> Result<(), String> {
    let overrides = FingerprintOverrides {
        by_host: vec![
            HostOverride {
                pattern: "*.workday.com".into(),
                profile: FingerprintProfile::paranoid(),
            },
            HostOverride {
                pattern: "*".into(),
                profile: FingerprintProfile::balanced(),
            },
        ],
    };
    let p = overrides
        .resolve("acme.workday.com")
        .ok_or_else(|| "no match for acme.workday.com".to_string())?;
    check(p.canvas_noise, "workday should have canvas_noise (paranoid)")?;
    let p = overrides
        .resolve("boards.greenhouse.io")
        .ok_or_else(|| "no match for boards.greenhouse.io".to_string())?;
    check(!p.canvas_noise, "greenhouse should fall through to balanced")
}

fn host_override_no_match_returns_none() -> Result<(), String> {
    let overrides = FingerprintOverrides {
        by_host: vec![HostOverride {
            pattern: "*.workday.com".into(),
            profile: FingerprintProfile::paranoid(),
        }],
    };
    check(
        overrides.resolve("greenhouse.io").is_none(),
        "no override should resolve",
    )
}

fn glob_matches_simple_cases() -> Result<(), String> {
    let cases: &[(&str, &str, bool)] = &[
        ("foo", "foo", true),
        ("foo", "bar", false),
        ("*.foo", "x.foo", true),
        ("*.foo", "x.y.foo", true),
        ("*.foo", "x.bar", false),
        ("*", "anything", true),
        ("a*c", "abc", true),
        ("a*c", "axyzc", true),
        ("a*c", "abd", false),
    ];
    for (p, t, want) in cases {
        let got = matches_glob(p, t);
        check_eq(got, *want, &format!("matches_glob({p:?}, {t:?})"))?;
    }
    Ok(())
}

fn glob_edge_cases() -> Result<(), String> {
    let cases: &[(&str, &str, bool)] = &[
        // Empty pattern only matches empty target.
        ("", "anything", false),
        ("", "", true),
        // Single star matches everything including empty.
        ("*", "anything", true),
        ("*", "", true),
        // Multiple stars are equivalent to one.
        ("**", "anything", true),
        ("***", "anything", true),
        // Wrap-with-stars: matches when the literal appears anywhere.
        ("*foo*", "abfooxy", true),
        ("*foo*", "foo", true),
        ("*foo*", "ab", false),
        // Leading + trailing literal must align.
        ("foo*", "foobar", true),
        ("foo*", "xfoobar", false),
        ("*foo", "barfoo", true),
        ("*foo", "barfooz", false),
        // Repeated literal — audit-fix-#4 corner.
        ("a*a", "ab", false),
        ("a*a", "aba", true),
        ("a*a", "a", false),
        // Multi-segment pattern.
        ("a*b*c", "abc", true),
        ("a*b*c", "axbyc", true),
        ("a*b*c", "axb", false),
        // Hostname patterns (the actual production use case).
        ("*.workday.com", "acme.workday.com", true),
        ("*.workday.com", "workday.com", false),
        ("*.workday.com", "myworkday.com", false),
        ("*.icims.com", "subdomain.icims.com", true),
    ];
    for (p, t, want) in cases {
        let got = matches_glob(p, t);
        check_eq(got, *want, &format!("matches_glob({p:?}, {t:?})"))?;
    }
    Ok(())
}

fn host_override_realistic_patterns() -> Result<(), String> {
    let overrides = FingerprintOverrides {
        by_host: vec![
            HostOverride {
                pattern: "*.workday.com".into(),
                profile: FingerprintProfile::paranoid(),
            },
            HostOverride {
                pattern: "*.icims.com".into(),
                profile: FingerprintProfile::balanced(),
            },
            HostOverride {
                pattern: "boards.greenhouse.io".into(),
                profile: FingerprintProfile::balanced(),
            },
        ],
    };
    // Workday tenants → paranoid.
    check(
        overrides.resolve("acme.workday.com").is_some(),
        "workday should match",
    )?;
    check(
        overrides.resolve("acme.workday.com").unwrap().canvas_noise,
        "workday should have canvas_noise",
    )?;
    // iCIMS tenants → balanced (no canvas noise).
    check(
        overrides.resolve("careers.icims.com").is_some(),
        "icims should match",
    )?;
    check(
        !overrides.resolve("careers.icims.com").unwrap().canvas_noise,
        "icims should NOT have canvas_noise",
    )?;
    // Exact-host pattern (no wildcards) requires exact match.
    check(
        overrides.resolve("boards.greenhouse.io").is_some(),
        "exact host should match",
    )?;
    check(
        overrides.resolve("boards-staging.greenhouse.io").is_none(),
        "near-miss should NOT match exact",
    )?;
    // Unknown host → None (falls through to global default).
    check(
        overrides.resolve("jobs.example.com").is_none(),
        "unknown host should fall through",
    )
}
