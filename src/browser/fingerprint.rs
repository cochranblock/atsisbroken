// SPDX-License-Identifier: Unlicense

//! Anti-fingerprint controls. Because we own the engine, the
//! signals an ATS vendor uses to identify automation can become
//! config knobs: User-Agent, navigator.webdriver, canvas hash,
//! font list, plugins enumeration, screen dimensions, timezone,
//! WebGL vendor. This module defines the schema + per-domain
//! override matcher; the values aren't enforced against a live
//! page yet (the JS engine that exposes them — mozjs — lands in
//! a later phase). The configuration shape is stable and the
//! TOML round-trips, so once the engine arrives the wiring is
//! a connection, not a redesign.
//!
//! The browser ships with three preset profiles and accepts a
//! `~/.atsisbroken/fingerprints.toml` for power users who want
//! per-domain overrides.

use serde::{Deserialize, Serialize};

/// What we tell the page about ourselves. None means "use the
/// engine default for the host platform"; Some pins a specific
/// value. Per-domain overrides live separately in
/// [`FingerprintOverrides`].
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct FingerprintProfile {
    /// User-Agent string. None → "looks like our actual platform's
    /// stock Firefox-current build."
    #[serde(default)]
    pub user_agent: Option<String>,
    /// `navigator.webdriver`. Default false (we are NOT advertising
    /// automation). The CDP-driven path is forced to expose
    /// `webdriver=true` by Chromium; we don't have that constraint.
    #[serde(default)]
    pub navigator_webdriver: bool,
    /// `navigator.platform`. None → host default.
    #[serde(default)]
    pub navigator_platform: Option<String>,
    /// Locale list ("en-US", "en"). None → host default.
    #[serde(default)]
    pub languages: Vec<String>,
    /// IANA timezone identifier ("America/New_York"). None → host
    /// default. Spoofing this means lying about the user's
    /// location, which has consequences (ATS forms sometimes ask
    /// for location and use this to pre-populate). Default leaves
    /// it true to host.
    #[serde(default)]
    pub timezone: Option<String>,
    /// Logical screen width × height (CSS pixels, not device).
    /// None → host default. Useful for "look like a 1366×768
    /// laptop" when the real machine is 4K.
    #[serde(default)]
    pub screen: Option<(u32, u32)>,
    /// Hardware concurrency advertised to JS. None → host default.
    /// Some vendors fingerprint by "this number is unusually high
    /// for a real laptop" — the dev box at 12 cores is a tell.
    #[serde(default)]
    pub hardware_concurrency: Option<u8>,
    /// Whether to include a Canvas2D fingerprint randomization
    /// shim. When true, every `getImageData` / `toDataURL` call
    /// gets a small per-session noise added. Off by default
    /// because some ATS forms (Workday's image CAPTCHAs) need
    /// pixel-stable canvas; on for vendors who fingerprint via
    /// canvas.
    #[serde(default)]
    pub canvas_noise: bool,
    /// Whether to randomize WebGL vendor / renderer strings per
    /// session. Off by default for the same reason as canvas.
    #[serde(default)]
    pub webgl_noise: bool,
}

impl FingerprintProfile {
    /// Default preset: "look like a stock Firefox install on the
    /// user's actual platform." Most ATS vendors don't fingerprint
    /// hard; this passes most checks without requiring noise
    /// shims that break image CAPTCHAs.
    pub fn balanced() -> Self {
        Self::default()
    }

    /// Aggressive preset: noise + spoof every signal we can reach.
    /// Use for ATS vendors that aggressively fingerprint (Workday
    /// on the bad days, vendors behind Cloudflare Bot Management).
    /// Risk: canvas-noise breaks image CAPTCHAs.
    pub fn paranoid() -> Self {
        Self {
            user_agent: Some(
                "Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0".into(),
            ),
            navigator_webdriver: false,
            navigator_platform: Some("Linux x86_64".into()),
            languages: vec!["en-US".into(), "en".into()],
            timezone: None,
            screen: Some((1920, 1080)),
            hardware_concurrency: Some(8),
            canvas_noise: true,
            webgl_noise: true,
        }
    }

    /// Minimal preset: don't spoof anything. The engine's natural
    /// fingerprint goes out. Useful for debugging when you need
    /// to know what the page is seeing without our overrides.
    pub fn natural() -> Self {
        Self {
            user_agent: None,
            navigator_webdriver: false,
            navigator_platform: None,
            languages: Vec::new(),
            timezone: None,
            screen: None,
            hardware_concurrency: None,
            canvas_noise: false,
            webgl_noise: false,
        }
    }
}

/// Per-domain overrides. Power-user concept; lives at
/// `~/.atsisbroken/fingerprints.toml`. Wins over the global
/// profile when the host matches.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct FingerprintOverrides {
    /// Glob-shaped host patterns. "*.workday.com" matches every
    /// Workday tenant. The first pattern that matches wins.
    pub by_host: Vec<HostOverride>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostOverride {
    pub pattern: String,
    pub profile: FingerprintProfile,
}

impl FingerprintOverrides {
    /// Resolve which profile applies to a given host. Returns the
    /// first matching override, or `None` for "use the global."
    pub fn resolve(&self, host: &str) -> Option<&FingerprintProfile> {
        for entry in &self.by_host {
            if matches_glob(&entry.pattern, host) {
                return Some(&entry.profile);
            }
        }
        None
    }
}

/// Naive glob match: `*` matches any run of chars; everything
/// else is literal. Sufficient for host patterns; not a full
/// regex. We deliberately don't pull a regex dep into the
/// browser shell for this.
fn matches_glob(pattern: &str, target: &str) -> bool {
    // Fast path: no wildcards.
    if !pattern.contains('*') {
        return pattern == target;
    }
    let parts: Vec<&str> = pattern.split('*').collect();
    let mut idx = 0;
    let target = target.as_bytes();
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 && !target[idx..].starts_with(part.as_bytes()) {
            return false;
        }
        if i == parts.len() - 1 && !target.ends_with(part.as_bytes()) {
            return false;
        }
        match find_subslice(&target[idx..], part.as_bytes()) {
            Some(found) => idx += found + part.len(),
            None => return false,
        }
    }
    true
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack
        .windows(needle.len())
        .position(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balanced_default_has_no_overrides() {
        let p = FingerprintProfile::balanced();
        assert!(p.user_agent.is_none());
        assert!(!p.navigator_webdriver);
        assert!(!p.canvas_noise);
        assert!(!p.webgl_noise);
    }

    #[test]
    fn paranoid_sets_every_signal() {
        let p = FingerprintProfile::paranoid();
        assert!(p.user_agent.is_some());
        assert!(p.navigator_platform.is_some());
        assert!(p.canvas_noise);
        assert!(p.webgl_noise);
        assert_eq!(p.languages, vec!["en-US".to_string(), "en".to_string()]);
    }

    #[test]
    fn webdriver_default_false_for_every_preset() {
        // Pins the struct default across every preset. This is a
        // configuration-shape contract, not a runtime one — the JS
        // engine that would actually expose the value to a page
        // (mozjs) hasn't landed yet. The pin still has value: it's
        // what the JS layer will read when it arrives, and is also
        // the answer the headed CDP fallback uses while it's the
        // active engine path. Chromium-via-CDP fights its own
        // webdriver=true default; we don't have that constraint.
        for p in [
            FingerprintProfile::balanced(),
            FingerprintProfile::paranoid(),
            FingerprintProfile::natural(),
        ] {
            assert!(!p.navigator_webdriver, "preset default flipped");
        }
    }

    #[test]
    fn fingerprint_round_trips_through_toml() {
        let p = FingerprintProfile::paranoid();
        let toml_text = toml::to_string(&p).unwrap();
        let back: FingerprintProfile = toml::from_str(&toml_text).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn host_override_resolves_first_match() {
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
        let p = overrides.resolve("acme.workday.com").unwrap();
        assert!(p.canvas_noise);
        let p = overrides.resolve("boards.greenhouse.io").unwrap();
        assert!(!p.canvas_noise);
    }

    #[test]
    fn host_override_no_match_returns_none() {
        let overrides = FingerprintOverrides {
            by_host: vec![HostOverride {
                pattern: "*.workday.com".into(),
                profile: FingerprintProfile::paranoid(),
            }],
        };
        assert!(overrides.resolve("greenhouse.io").is_none());
    }

    #[test]
    fn glob_matches_simple_cases() {
        assert!(matches_glob("foo", "foo"));
        assert!(!matches_glob("foo", "bar"));
        assert!(matches_glob("*.foo", "x.foo"));
        assert!(matches_glob("*.foo", "x.y.foo"));
        assert!(!matches_glob("*.foo", "x.bar"));
        assert!(matches_glob("*", "anything"));
        assert!(matches_glob("a*c", "abc"));
        assert!(matches_glob("a*c", "axyzc"));
        assert!(!matches_glob("a*c", "abd"));
    }

    /// Edge cases the Rust audit raised concerns about. Pinning
    /// the actual behavior so future refactors don't regress
    /// these specific surfaces. The audit (#4) claimed
    /// `matches_glob("", anything)` returned true; tracing the
    /// code shows the fast path `!pattern.contains('*')` catches
    /// empty pattern first and returns `"" == target` — false for
    /// any non-empty target. The bug doesn't exist as described.
    /// But the broader concern — sparse coverage of glob edge
    /// cases — is valid, so the test ships anyway.
    #[test]
    fn glob_edge_cases() {
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
            // Wrap-with-stars: matches when the literal appears
            // anywhere.
            ("*foo*", "abfooxy", true),
            ("*foo*", "foo", true),
            ("*foo*", "ab", false),
            // Leading + trailing literal must align.
            ("foo*", "foobar", true),
            ("foo*", "xfoobar", false),
            ("*foo", "barfoo", true),
            ("*foo", "barfooz", false),
            // Repeated literal — the audit flagged this as a
            // potential corner. Empirically: works correctly.
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
            assert_eq!(
                got, *want,
                "matches_glob({p:?}, {t:?}) = {got}, want {want}"
            );
        }
    }

    /// Hostname-pattern usage is the actual production surface
    /// for matches_glob. Pin the by-host resolver behavior with
    /// realistic patterns the user might write in
    /// fingerprints.toml.
    #[test]
    fn host_override_realistic_patterns() {
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
        assert!(overrides.resolve("acme.workday.com").is_some());
        assert!(overrides.resolve("acme.workday.com").unwrap().canvas_noise);
        // iCIMS tenants → balanced (no canvas noise).
        assert!(overrides.resolve("careers.icims.com").is_some());
        assert!(!overrides.resolve("careers.icims.com").unwrap().canvas_noise);
        // Exact-host pattern (no wildcards) requires exact match.
        assert!(overrides.resolve("boards.greenhouse.io").is_some());
        assert!(overrides.resolve("boards-staging.greenhouse.io").is_none());
        // Unknown host → None (falls through to global default).
        assert!(overrides.resolve("jobs.example.com").is_none());
    }
}
