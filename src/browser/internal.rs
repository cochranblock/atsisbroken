// SPDX-License-Identifier: Unlicense
// Unlicense — public domain — cochranblock.org

//! Internal pages — the `atsisbroken://` scheme.
//!
//! The browser has a built-in router for URLs whose scheme is
//! `atsisbroken://`. Each page is a Rust function that returns
//! a title + body string. The body is multi-line text that the
//! glyphon-based renderer paints into the window. When stylo +
//! WebRender land, these pages get upgraded to real HTML/CSS
//! layouts with clickable tiles; today the surface is text-with-
//! structure (using ascii box characters and indentation) so the
//! page can be navigated and read while we build out the visual
//! richness.
//!
//! Naming convention: every routable page maps to a stable URL
//! the user can type or share. The map of URL → page lives in
//! [`render`] as one match arm per page.

#![cfg(feature = "gui")]

use super::Url;

/// What the browser renders for one internal URL. Title shown
/// big at top of the window; body shown below as a flowing
/// paragraph (glyphon handles `\n` as line breaks inside a single
/// text run).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternalPage {
    pub title: String,
    pub body: String,
}

/// Resolve an internal URL to its rendered page. Unknown
/// internal URLs return a "page not found" page that includes
/// the requested path so the user can see what was attempted.
pub fn render(url: &Url) -> InternalPage {
    match url.host.as_str() {
        "home" => home(),
        "connections" => connections(),
        "summary" => summary(),
        "discover" => discover(),
        "queue" => queue(),
        "applications" => applications(),
        "responses" => responses(),
        "identities" => identities(),
        "sources" => sources(),
        "shield" => shield(),
        "fingerprint" => fingerprint(),
        "network" => network(),
        "settings" => settings(),
        "audit-bar" => audit_bar(),
        "onboarding" => onboarding(),
        other => not_found(other, &url.path),
    }
}

fn home() -> InternalPage {
    InternalPage {
        title: "atsisbroken".to_string(),
        body: "\
The browser for filling job applications.

Connect what you've made. The plan: write your applications from
it, anchoring every freetext sentence to a public URL you already
wrote — GitHub, blog, Stack Overflow, the rest. Nothing leaves this
device unless you say so. The composer that does the anchoring
ships in a later phase; today's binary covers the connectors and
the local schema.

Where to start

    atsisbroken://connections     your sources
    atsisbroken://summary         what we say about you
    atsisbroken://discover        public job boards, aggregated
    atsisbroken://applications    your submission ledger
    atsisbroken://network         every outbound call, live
    atsisbroken://settings        modes, identities, privacy

The address bar at the top accepts any URL — internal or web.\
"
        .to_string(),
    }
}

fn connections() -> InternalPage {
    InternalPage {
        title: "Your connections".to_string(),
        body: "\
Active (0)

    No connections yet. Pick a source below to start.

Available

  Public-handle services (no auth, just your username)

    GitHub               Code, READMEs, recent commit messages.
                         atsisbroken://connect/github

    Stack Overflow       Q&A history, vote-weighted answers.
                         atsisbroken://connect/stackoverflow

    Hacker News          Comments + submissions, karma-weighted.
                         atsisbroken://connect/hn

    crates.io            Published Rust packages.
                         atsisbroken://connect/cratesio

    npm                  Published Node packages.
                         atsisbroken://connect/npm

    Bluesky              AT Protocol — public posts.
                         atsisbroken://connect/bluesky

    Mastodon             ActivityPub — public posts.
                         atsisbroken://connect/mastodon

    arXiv                Academic papers by author.
                         atsisbroken://connect/arxiv

    USPTO Patents        Granted + pending patents.
                         atsisbroken://connect/uspto

    Personal blog        Auto-discover via RSS / Atom / sitemap.
                         atsisbroken://connect/blog

  OAuth services (sign in via this browser, no system handoff)

    LinkedIn             Profile + positions.
                         atsisbroken://connect/linkedin

    Reddit               Vote-weighted comment + submission history.
                         atsisbroken://connect/reddit

  Session-cookie services (sign in normally, we capture the cookie)

    Substack             Your paid newsletter content.
                         atsisbroken://connect/substack

    Medium               Member-only stories.
                         atsisbroken://connect/medium

  More

    16 additional services in the full list:
    atsisbroken://sources

Each connection's data lives in ~/.atsisbroken/. Disconnect any time
to delete the local cache. Network audit is live at
atsisbroken://network — every byte we fetch is visible.\
"
        .to_string(),
    }
}

fn summary() -> InternalPage {
    InternalPage {
        title: "Your auto-summary".to_string(),
        body: "\
Bio
    Connect a source first. We need products to summarize from.
    [atsisbroken://connections]

Themes
    No themes detected — graph is empty.

Coverage gaps
    Frontend / UX work — connect a portfolio source.
    Speaking / talks — connect YouTube or paste manually.
    Long-form writing — connect a blog or Substack.

Test against a JD
    Paste a job description here once your sources are connected;
    we'll preview what your application would say.\
"
        .to_string(),
    }
}

fn discover() -> InternalPage {
    InternalPage {
        title: "Discover".to_string(),
        body: "\
Job aggregation from public boards.

Sources we'll poll (when implemented):
    LinkedIn Jobs RSS feed (per saved search)
    Indeed RSS feed
    HN Who-is-hiring threads
    USAJOBS public API
    Greenhouse vendor public boards
    Lever vendor public boards
    Ashby vendor public boards

Filter by keyword, salary range, remote/onsite, visa sponsorship.
Click 'Add to queue' to drop a posting into atsisbroken://queue.

Feed-polling implementation lands in a follow-up commit. Today
this page is a placeholder so the URL works.\
"
        .to_string(),
    }
}

fn queue() -> InternalPage {
    InternalPage {
        title: "Application queue".to_string(),
        body: "\
The queue page processes pending applications in batch. Each
posting in the queue runs through: navigate → detect ATS form →
classify fields → compose freetext → review against deny-list →
submit (in Vapor mode) or pause for user (Vault).

Per-domain rate limiter prevents IP bans across queue runs.

Queue runtime + state machine implementation lands once the engine
can drive real ATS pages (stylo + mozjs). For now this page is a
placeholder.\
"
        .to_string(),
    }
}

fn applications() -> InternalPage {
    InternalPage {
        title: "Your applications".to_string(),
        body: "\
Local ledger of every submission. Per-application audit page shows
each emitted token with its source citation — for your own
verification of what the tool did on your behalf.

No applications yet. The ledger populates when the run loop
detects a successful submit on an ATS form.\
"
        .to_string(),
    }
}

fn responses() -> InternalPage {
    InternalPage {
        title: "Application responses".to_string(),
        body: "\
Auto-detected status updates from your inbox.

Email integration: not connected.
    Connect via atsisbroken://settings to enable IMAP polling.
    Credentials stay local (chmod 600); no cloud relay.

Pattern detection (when enabled):
    Acknowledged: 'received' + 'thank you' + 'review'
    Interview: 'schedule' / 'next steps' / 'phone call'
    Rejected: 'regret' / 'moved forward' / 'not the right fit'
    Ghosted: 14+ days from submit, no email\
"
        .to_string(),
    }
}

fn identities() -> InternalPage {
    InternalPage {
        title: "Identities".to_string(),
        body: "\
Multiple narratives built from the same products.

Active: Default (no other identities created yet)

    Same products → different ranking. A 'Technical IC' identity
    weights GitHub heavily; an 'Engineering Manager' identity
    weights LinkedIn + leadership-tagged blog posts.

To add an identity: edit ~/.atsisbroken/identities.toml. UI for
creating + editing identities lands when the address bar input
layer ships.\
"
        .to_string(),
    }
}

fn sources() -> InternalPage {
    InternalPage {
        title: "All available sources".to_string(),
        body: "\
Full list of integrations, by auth shape.

Public-handle (no token required)
    GitHub, GitLab, Bitbucket, Codeberg, Sourcehut
    Stack Overflow, Hacker News, Reddit
    crates.io, npm, PyPI, RubyGems, Docker Hub
    arXiv, USPTO, Google Scholar, ORCID
    Bluesky, Mastodon
    YouTube, Twitch
    Behance, Dribbble, ArtStation
    Dev.to, Hashnode
    Personal blog (RSS/Atom/sitemap)
    USAJOBS

OAuth (browser-internal redirect capture)
    LinkedIn, Twitter / X, Reddit, Notion, Patreon, Spotify

Session-cookie (sign in normally, we capture the cookie)
    Substack, Medium, LinkedIn (beyond OAuth scopes)

Manual paste
    Resume / arbitrary content (you attest authorship)\
"
        .to_string(),
    }
}

fn shield() -> InternalPage {
    InternalPage {
        title: "Tracker shield".to_string(),
        body: "\
Default rule list ships with the binary. Distilled from EasyList
+ EasyPrivacy + ATS-vendor-specific analytics surfaces.

Categories blocked by default:
    Analytics      Google, Mixpanel, Amplitude, Heap, Segment
    Behavioral fp  FullStory, Hotjar, ContentSquare
    Ad tech        DoubleClick, Taboola, Outbrain
    Social pixels  Facebook, LinkedIn, Twitter
    Cookie walls   Auto-dismissed (Reject All)
    ATS telemetry  Greenhouse Segment, Lever Heap, Workday

Live blocking implementation comes with the JS-capable engine.
Today this page documents what shield will do.\
"
        .to_string(),
    }
}

fn fingerprint() -> InternalPage {
    InternalPage {
        title: "Anti-fingerprint".to_string(),
        body: "\
Default profile: Paranoid

    User-Agent              Firefox/128 on Linux x86_64 (planned)
    navigator.webdriver     false (planned default)
    Canvas hash             per-session noise (planned)
    WebGL vendor            per-session randomization (planned)
    Font enumeration        common-fonts subset (planned)
    Hardware concurrency    reports 8 (planned)
    Timezone                host (we don't lie about location)
    Languages               host
    Screen size             host

Per-domain overrides
    *.workday.com           Paranoid (default)
    *.icims.com             Balanced
    *.greenhouse.io         Balanced
    Add overrides via ~/.atsisbroken/fingerprints.toml

Status: the FingerprintProfile struct + per-domain override matcher
are implemented and tested. The browser doesn't have a JS engine
yet, so the values aren't enforced against a real page — that
ships when mozjs lands. Until then this page documents intent and
the configuration shape, not a live guarantee.

Note: Canvas/WebGL noise breaks image CAPTCHAs on some vendors.
If a CAPTCHA wall trips, switch that domain to Balanced.\
"
        .to_string(),
    }
}

fn network() -> InternalPage {
    InternalPage {
        title: "Network audit".to_string(),
        body: "\
Live outbound network audit (this session).

Total calls so far: 0
    Target ATS pages:           0
    Connected source endpoints: 0
    atsisbroken core:           0
    Blocked by tracker shield:  0

The privacy hawk audits with this page. Every call is real. No
telemetry to atsisbroken's authors. Ever.

Real-time updating implementation comes with the engine's network
layer. Today this page is a static placeholder.\
"
        .to_string(),
    }
}

fn settings() -> InternalPage {
    InternalPage {
        title: "Settings".to_string(),
        body: "\
Configurable surfaces (each links to its own page when input lands)

    Mode default              atsisbroken://settings/mode
    Identity default          atsisbroken://identities
    Input timing              atsisbroken://settings/typing
    Fingerprint preset        atsisbroken://fingerprint
    Tracker rules             atsisbroken://shield
    Source connections        atsisbroken://connections
    Email integration         atsisbroken://settings/email
    Network audit             atsisbroken://network
    Factory reset             atsisbroken://settings/reset

Hardware
    Profile path: ~/.atsisbroken/profile.toml
    Config path:  ~/.atsisbroken/config.toml
    Tokens dir:   ~/.atsisbroken/tokens/  (chmod 600)\
"
        .to_string(),
    }
}

fn audit_bar() -> InternalPage {
    InternalPage {
        title: "Audit bar".to_string(),
        body: "\
Single-purpose view: 'I am proving I'm filling this honestly.'

Design intent: keep this page open during an interview to show
the audit story live — as applications get filled, the audit bar
will list every emitted freetext token alongside the public URL
the composer pulled it from. The composer + token-to-URL ledger
land in a later phase; today this page documents the shape.\
"
        .to_string(),
    }
}

fn onboarding() -> InternalPage {
    InternalPage {
        title: "Welcome to atsisbroken".to_string(),
        body: "\
First-run wizard.

Step 1 — Paste a resume (optional)
    We'll extract the basics: name, email, phone, LinkedIn,
    GitHub. Skip if you'd rather build from connectors.

Step 2 — Connect your sources
    The more we have, the better your auto-summary. GitHub +
    one writing source covers most candidates. Add more as
    they apply.

Step 3 — Pick a fill mode
    TrainingWheels (recommended for first 5 applications)
    Shadow         (highlight + click)
    Chaos          (auto-fill on page load, you submit)
    Vault          (auto-fill + deny-list, you submit)
    Vapor          (auto-fill + auto-submit, per-session opt-in)

Step 4 — Done
    Land on atsisbroken://home and start applying.

The wizard's interactive flow lands when the address bar input
layer ships.\
"
        .to_string(),
    }
}

fn not_found(host: &str, path: &str) -> InternalPage {
    let attempted = if path.is_empty() {
        format!("atsisbroken://{host}")
    } else {
        format!("atsisbroken://{host}{path}")
    };
    InternalPage {
        title: "Page not found".to_string(),
        body: format!(
            "\
The internal page {attempted} doesn't exist (yet).

Try one of these:

    atsisbroken://home
    atsisbroken://connections
    atsisbroken://summary
    atsisbroken://applications

Or navigate to any web URL via the address bar — atsisbroken
renders pages from the open web too.\
"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn internal_home_renders_with_url_menu() {
        // The in-process home page (still reachable via
        // atsisbroken://home, even though Url::home() now points
        // at the live landing page).
        let page = render(&Url::internal_home());
        assert_eq!(page.title, "atsisbroken");
        assert!(page.body.contains("atsisbroken://connections"));
        assert!(page.body.contains("atsisbroken://summary"));
        assert!(page.body.contains("atsisbroken://discover"));
    }

    #[test]
    fn connections_lists_known_services() {
        let page = render(&Url::internal("connections"));
        assert!(page.title.contains("connections"));
        // Public-handle services
        assert!(page.body.contains("GitHub"));
        assert!(page.body.contains("Stack Overflow"));
        assert!(page.body.contains("Hacker News"));
        assert!(page.body.contains("crates.io"));
        // OAuth services
        assert!(page.body.contains("LinkedIn"));
        assert!(page.body.contains("Reddit"));
        // Session-cookie services
        assert!(page.body.contains("Substack"));
        // Connect targets
        assert!(page.body.contains("atsisbroken://connect/github"));
    }

    #[test]
    fn summary_links_to_connections_when_empty() {
        let page = render(&Url::internal("summary"));
        // When no connections, summary directs the user to connect first.
        assert!(page.body.contains("atsisbroken://connections"));
    }

    #[test]
    fn unknown_internal_url_returns_not_found_with_attempted_path() {
        let url: Url = "atsisbroken://garbage".parse().unwrap();
        let page = render(&url);
        assert_eq!(page.title, "Page not found");
        assert!(page.body.contains("atsisbroken://garbage"));
    }

    #[test]
    fn unknown_internal_url_with_path_includes_path() {
        let url: Url = "atsisbroken://garbage/some/path".parse().unwrap();
        let page = render(&url);
        assert!(page.body.contains("atsisbroken://garbage/some/path"));
    }

    #[test]
    fn every_known_page_returns_non_empty_title_and_body() {
        // Pin: every routed page has both title and body. An empty
        // surface is worse than a "not yet implemented" placeholder.
        for host in [
            "home",
            "connections",
            "summary",
            "discover",
            "queue",
            "applications",
            "responses",
            "identities",
            "sources",
            "shield",
            "fingerprint",
            "network",
            "settings",
            "audit-bar",
            "onboarding",
        ] {
            let page = render(&Url::internal(host));
            assert!(!page.title.is_empty(), "{host}: empty title");
            assert!(!page.body.is_empty(), "{host}: empty body");
        }
    }

    #[test]
    fn page_bodies_have_no_trailing_whitespace_per_line() {
        // Discipline: text-rendering keeps lines clean. Trailing
        // whitespace can produce phantom spaces in the rendered
        // glyph layout.
        for host in ["home", "connections", "summary"] {
            let page = render(&Url::internal(host));
            for (i, line) in page.body.lines().enumerate() {
                assert_eq!(
                    line,
                    line.trim_end(),
                    "{host} line {i} has trailing whitespace: {line:?}"
                );
            }
        }
    }

    #[test]
    fn parse_internal_url_routes_correctly() {
        let url = Url::from_str("atsisbroken://applications").unwrap();
        let page = render(&url);
        assert!(page.title.to_lowercase().contains("applications"));
    }

    #[test]
    fn shield_documents_blocked_categories() {
        let page = render(&Url::internal("shield"));
        assert!(page.body.contains("Analytics"));
        assert!(page.body.contains("Cookie walls"));
        assert!(page.body.contains("ATS telemetry"));
    }

    #[test]
    fn fingerprint_page_documents_webdriver_default() {
        // The fingerprint page should advertise the webdriver=false
        // default so the user can see the planned contract; whether
        // it's actually enforced against a real page depends on the
        // (not-yet-landed) JS engine. This test pins the page text,
        // not the runtime guarantee.
        let page = render(&Url::internal("fingerprint"));
        assert!(page.body.contains("navigator.webdriver"));
        assert!(page.body.contains("false"));
    }
}
