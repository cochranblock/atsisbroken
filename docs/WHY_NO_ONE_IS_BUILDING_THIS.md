<!-- Unlicense — cochranblock.org -->
<!-- Contributors: GotEmCoach, KOVA, Claude Opus 4.7 -->

# Why no one is building this

**Date:** 2026-05-03
**Scope:** Honest analysis of why a free, local-first, verbatim-source
ATS autofiller doesn't exist commercially despite obvious demand —
and why The Cochran Block, LLC is uniquely positioned to ship it.
**Companion:** `PLAN.md` (especially §5 Funnels) and
`HIRING_MANAGER_ANALYSIS.md`.

---

## The puzzle

The market wants this. Every Reddit r/cscareerquestions thread about
job hunts is half complaining about ATS. Simplify.us has 2M+ users
on a paid product that does *worse* than what we're building. The
technical pieces (CDP, on-device inference, MV3 extensions) have
been mature for 2+ years.

So why isn't there an open-source, local-first, verbatim-source
ATS autofiller already on every dev's laptop?

Ten honest reasons, grouped.

---

## A. Commercial incentives are inverted

### A1. SaaS LTV math kills the free local product

Simplify.us monetizes free → paid conversion at ~3% per user. A
local-first, no-account, no-cloud version of their product **deletes
their LTV** without replacing it. You can't pivot a SaaS to free-
local; you have to start over.

A free product loses to a paid product in only one place: the
balance sheet of whoever's building it. And the balance sheet is
who decides what gets built.

### A2. There's no VC story

A pitch deck for atsisbroken would say:
- No SaaS revenue.
- No user data to sell.
- No network effect (each user's data stays on their machine).
- No platform play.
- "Forever free" Unlicense.

Translation: no VC will fund this. Which means no full-time team
of 5–10 engineers will work on it. Which means it has to come from
an indie or a side-project shop with conviction and other revenue
streams.

### A3. Privacy-hawk segment is small *and* unprofitable

P2 from `USER_STORY_ANALYSIS.md` (Mara, security engineer) is the
ideological core user — the one who'll tweet about it, audit the
source, and tell every dev she knows. She's also the worst customer
for a SaaS: refuses subscriptions, reads every TOS, distrusts
analytics. SaaS companies route around her.

A free product can serve P2 as a feature, not a bug. Different
business model unlocks her segment.

---

## B. Engineering cost is real

### B1. Local-first is 10× the engineering of "Chrome extension hits cloud API"

Look at what shipping atsisbroken takes:
- Cross-compile for 4 OS triples + 3 mobile ABIs.
- Ship a model that trains on the user's machine in seconds on CPU.
- Detect the user's default browser per OS (no clean Rust crate).
- Optionally **install** Chromium when missing.
- Native Messaging bridge between extension and binary.
- 7-tier strategy ladder for every environment.
- TUI for terminal users.
- Audit log + verbatim-source composer.
- 156+ tests + TRIPLE SIMS determinism gate.

Compare to "Simplify Pro": MV3 extension + REST calls to a Node
backend with OpenAI as a dependency. Maybe a week to MVP. Maybe a
month to ship.

The engineering cost asymmetry is **the** reason commercial
competitors picked the cloud-LLM path. It's not that they didn't
think of local-first. It's that local-first is hard and the unit
economics don't justify it for a SaaS.

### B2. Native Messaging is unfashionable

Chrome's `nativeMessaging` API has existed since 2014. It's the
right transport for "extension speaks to local binary." Almost
nobody uses it because:
- Setup requires the user to install a host manifest in a per-OS
  path (we automated this with `install-bridge`, but only after
  acknowledging the friction).
- The Chrome team doesn't promote it (it's not a feature that
  helps Google's data flywheel).
- Most "extension + backend" tutorials assume the backend is a
  cloud HTTP API.

Result: a powerful primitive most teams forget exists. We use it
because we have to (no cloud); the rest of the world doesn't
because they don't have to.

### B3. Cross-compiling Rust for mobile + desktop + WASM is a chore

Rust's mobile + cross-compile story is *good* but never *easy*.
Most indie products that try this hit a wall on the third target
and abandon. Cochran Block has shipped this stack across kova /
pixel-forge / cochranblock already, so atsisbroken can stand on
that infrastructure rather than discover it.

---

## C. Industry ideology is in the way

### C1. The "AI cover letter" hype eclipsed verbatim-source

From 2023 through 2025, every job-search startup pivoted to "we use
GPT to write your cover letter." That's the **opposite** of what
hiring managers want (`HIRING_MANAGER_ANALYSIS.md` §3): hallucinated
prose, no source attribution, voice that doesn't match the resume.

But it was easy to demo, easy to monetize per-token, and it caught
the attention of the press. The right answer (verbatim-source, no
generation) was unfashionable for two years and got crowded out.

The hype cycle is now turning — recruiters are explicitly flagging
"AI-generated" applications. atsisbroken's thesis ("we don't
generate; we *quote*") is now the differentiated bet.

### C2. The verbatim-source thesis is contrarian

Most engineers in 2026 still believe the future is "LLMs writing
for you." atsisbroken's premise — that LLMs writing for you is the
*problem* — is hard to recruit a team around. Conviction is required
to build something that disagrees with the consensus, and conviction
is rarer than it looks.

The bet here: when the hype cycle finishes, the only ATS-autofiller
that doesn't make hiring managers angry will be the one that doesn't
generate prose. atsisbroken arrives at that future already shipping.

---

## D. Cochran Block has unfair advantages

### D1. The supporting stack is already built

To ship atsisbroken alone, you'd need:
- A local inference engine. (Cochran Block has `kova-engine`.)
- A model-training pipeline that produces small `.safetensors`. (`pixel-forge`.)
- A determinism gate. (`exopack`'s TRIPLE SIMS, now in kova.)
- A Diamond build profile for size + speed tradeoffs. (`cochranblock/diamond-profile.toml`.)
- A multi-node test infrastructure. (IRONHIVE cluster: lf/gd/bt/st.)
- A header-writer for licensing across all files.
- A handoff CLI for inter-agent collaboration.

Every one of those is a multi-month project. They already exist.
atsisbroken's incremental engineering cost is "wire them together
and add the ATS-specific layer" — maybe 3–4 weeks of focused work
for the v0.1 milestone, because the foundation isn't being built
*for atsisbroken*; it was built for the broader Cochran Block
portfolio and atsisbroken is one of many products that compose it.

A startup launching cold would need to build all of the above
**before** they could even start on atsisbroken proper.

### D2. The "ATS is broken" name requires conviction

A founder at Simplify, Greenhouse, or Workday cannot ship a product
literally named after the brokenness of the industry that pays them.
Cochran Block has no such constraint. The name is the thesis; the
thesis is the marketing; the marketing is the product. That's a
luxury most companies can't afford.

### D3. Unlicense is a one-way commitment

Once code is dedicated to the public domain, there's no walk-back.
A future board can't "monetize the IP." A future investor can't
demand a moat. The forever-free property is enforced at the legal
level, not the policy level. This is unattractive to most founders
because it removes a lever; it's attractive to atsisbroken's users
because it removes the only failure mode they've been burned by.

---

## E. The timing is recent

### E1. On-device tiny classifiers are practical only since ~2024

For decades, "ML in the browser / on the laptop" meant either
TensorFlow.js (heavy, slow) or sending to the cloud. The
combination of:
- candle (Rust ML framework, 2023)
- burn (Rust ML framework, 2024)
- WASM with relaxed-SIMD support
- safetensors as a portable format

…is what makes "the user trains their own model on their laptop in
seconds" possible. Before 2024, that wasn't a real option. atsisbroken
is genuinely a now-is-the-time idea, not a sat-on-it-for-years one.

---

## F. Defensibility — what happens when someone else figures it out?

The "moat" question. None of the moats below are absolute, but in
combination they're a real lead:

| Moat                                        | Strength | Decay |
|---------------------------------------------|----------|-------|
| First-mover on the verbatim-source thesis   | Medium   | 6–12 months |
| Cochran Block stack (kova, pixel-forge, etc.)| High     | Slow — re-creating the stack is years |
| Unlicense + brand split                     | Strong   | Doesn't decay |
| User-contributed seed corpus (network effect)| Compounds| Improves over time |
| GitHub-driven verbatim composer             | Medium   | Easy to copy once seen |
| Strategy ladder + Native Messaging bridge   | Medium   | Easy to copy once seen |
| The atsisbroken brand under Cochran Block   | Strong   | Doesn't decay |
| 156+ test suite + TRIPLE SIMS gate          | Soft     | Quality bar competitors avoid |

The *durable* moats are the brand, the Cochran Block portfolio
synergy, and the user-contributed corpus. Everything else is
copyable — and that's fine, because the Unlicense invites copying.
The win condition is being the **canonical implementation**, not
being the only one.

---

## G. Risks (what could close the window)

| Risk                                                          | Mitigation |
|---------------------------------------------------------------|------------|
| Chrome ships a built-in form-fill that adapts to ATS forms    | Chrome already has Autofill; the ATS-specific gap remains. We are differentiated by free-text generation + GitHub composer, not by basic key-value autofill |
| Simplify.us pivots to local-first                             | They lose 90% of their LTV doing it. Unlikely. If they do, we welcome them — public domain |
| A well-funded competitor copies the architecture              | They'd be a paid version of an Unlicense product. Hard pitch. |
| Apple ships a system-level ATS autofiller                     | Plausible on macOS only. Doesn't address Linux / Windows / mobile. We stay relevant on the other 75% of the market |
| Hiring becomes AI-mediated end-to-end                         | The verbatim-source approach is *more* relevant when AI screens AI; honesty becomes the differentiator |
| Cochran Block scope expands and atsisbroken is starved        | atsisbroken is small enough to maintain at low cost; the big-picture bet is Funnel A in PLAN.md (lead-gen for the author / portfolio) |

---

## H. So why is Cochran Block building it?

Because:
1. The supporting stack already exists (D1).
2. The free-product → interested-parties model (`PLAN.md` §5) makes
   the economics work without SaaS revenue (A1, A2).
3. The author has been on the receiving end of ATS forms enough to
   build with conviction (C2).
4. The verbatim-source thesis becomes more valuable as AI-generated
   applications become more common (G).
5. Indie tools that the press writes about generate adjacent
   opportunities for the author and the Cochran Block portfolio
   (`PLAN.md` Funnels A through F).

The product is free. The product is the marketing. The marketing
unlocks adjacent commerce. That's the whole loop, and it only works
when (a) the product is genuinely good and (b) the author has a
multi-product portfolio that benefits from the spotlight.

Both are true. So we ship.

---

## I. Forward-link

Follow-ups in the broader plan:
- `PLAN.md` §1 (engineering phases — the "we ship" part)
- `PLAN.md` §5 (Funnels A–F — the "interested parties" part)
- `HIRING_MANAGER_ANALYSIS.md` — the "this lands well with the
  receiving side" part
- `PLAN_PROFILE_AND_GITHUB.md` — the "verbatim-source composer"
  proof
