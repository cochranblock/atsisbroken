<!-- Unlicense — cochranblock.org -->
<!-- Contributors: GotEmCoach, KOVA, Claude Opus 4.7 -->

# atsisbroken — Path to a Real Product

**Date:** 2026-05-05
**Premise:** "Pattern-inferred" is not the same as "researched."
"It compiles" is not the same as "it works against the real thing."
Three of five fixtures shipped today are guesswork. The classifier
has never been measured against real DOMs. The fill loop runs
against mocks, not production. This document plans the research
required to take atsisbroken from a defensible scaffold to a tool
that holds up against the real world.

**Operating constraint:** every change is backed by evidence. No
shipping low-confidence components. No "looks right." We measure
or we don't ship.

---

## 1. Definition of "perfect"

A measurable target, not vibes.

| Property                                              | Target          | How measured |
|-------------------------------------------------------|-----------------|--------------|
| Vendor coverage                                       | ≥95% of US engineering applications     | Dimensional census against a sampled set of 200+ recent job postings |
| Classifier per-field accuracy                         | ≥98% on canonical fields (email, phone, name, linkedin, github, address, work_auth) | Held-out test set of real captured DOMs |
| Free-form question routing accuracy                   | ≥90%            | Human-judged sample of 100 questions per vendor |
| Zero-fabrication property                             | 100%            | Every emitted token traces to a verbatim source. Audit log inspection |
| Fill correctness (right value in right field)         | ≥99%            | Live e2e against captured-snapshot fixtures with known-good values |
| Anti-bot pass rate                                    | ≥80% of forms accept the submission without manual intervention | Live testing against real public postings (no actual submission) |
| Install success rate                                  | ≥95% of users complete install without support contact | Beta cohort instrumentation (opt-in only) |
| First-fill latency                                    | <30 seconds from "click run" to "every confident field filled" | Bench |
| Crash rate                                            | <0.1% of sessions | Local crash log, opt-in only |
| Data leakage                                          | 0 bytes leave the device unless `FeedbackDelivery::SendWhenOnline` is set | Auditable in `cargo tree` + grep |

If we don't hit these, we're shipping a worse-than-Simplify product.
The thesis collapses.

---

## 2. Research debt — what we don't know yet

The honest list of "we have not measured this":

| # | Question                                                           | Status |
|---|--------------------------------------------------------------------|--------|
| R1 | What ATS systems do US engineering job-seekers actually encounter, weighted by frequency? | Not measured. We assume Workday/Greenhouse/Lever dominate. **Assumption, not data.** |
| R2 | What does a real Workday application form's DOM actually look like across tenants? | Not captured. We have one Playwright extension's selectors. |
| R3 | What does a real iCIMS / Ashby / Taleo / SuccessFactors / SmartRecruiters form look like? | **No real captures.** Patterns inferred from public markup. |
| R4 | What's the classifier's accuracy on real DOMs (not synthetic)? | Not measured. Unit tests assert against fictional fixtures. |
| R5 | When the classifier is wrong, what's the failure mode distribution? (Off by one slot? `unknown` when shouldn't be? Right key but bad value?) | Not measured. |
| R6 | What anti-bot fingerprints do these vendors use? (Mouse trails, timing, Canvas, WebGL, `--remote-debugging-port` detection?) | Not researched. |
| R7 | What's the false-positive rate on EEO/demographics fields if we naively classify them? | Not measured. **Critical** — we MUST NOT autofill these. |
| R8 | What's the gap between TrainingWheels-mode prompts and real applicant patience? (How many yes/no's before they quit?) | Not measured. |
| R9 | What's the legal/compliance risk per vendor for automation against their forms? | Not researched. ToS-by-vendor analysis required before broad release. |
| R10 | What's the install funnel drop-off at each step? | Not measured. Will need beta cohort. |
| R11 | What do hiring managers think when they see autofill-shaped applications? | Not researched live; we have inference from blog posts (`HIRING_MANAGER_ANALYSIS.md`). |
| R12 | How fast do ATS vendors change their DOM (i.e. selector decay rate)? | Not measured. Historical data via Wayback Machine snapshots. |

Until we close these, "perfect" is a guess.

---

## 3. Phase R1 — ATS market census (target: 1 day)

### Method

1. Pull a sample of **200+ recent (2026-04 to 2026-05) US engineering
   job postings** from public aggregators that disclose the ATS:
   - LinkedIn Jobs (the Apply button URL leaks the ATS host)
   - Indeed (`url=greenhouse.io/...`, `url=lever.co/...`, etc.)
   - Hacker News "Who is hiring?" thread
2. Classify each by ATS host:
   - `boards.greenhouse.io` / `boards-api.greenhouse.io` → Greenhouse
   - `jobs.lever.co` → Lever
   - `*.myworkdayjobs.com` → Workday
   - `careers-*.icims.com` → iCIMS
   - `jobs.ashbyhq.com` / `*.ashbyhq.com` → Ashby
   - `*.taleo.net` → Taleo
   - `careers.*.com` (custom) → likely SuccessFactors / Avature / unknown
3. Count distribution. Plot share. Report.

### Deliverable

`docs/research/ATS_MARKET_SHARE_2026Q2.md` — sourced data with dates,
URLs, sample size. Sample size + selection bias documented.

### Open question this answers

R1. Tells us where to focus. If Workday is 50% and iCIMS is 3%, our
fixture priorities should reflect that.

### Sources to use

- [USE-Public — open ATS list](https://github.com/EmbeddedNature/job-board-list)
- LinkedIn Jobs (rate-limited, requires care)
- Hacker News API for "Who is hiring?" comment threads
- Aggregators: Levels.fyi, Otta, Wellfound (formerly AngelList)

---

## 4. Phase R2 — Real DOM capture (target: 3 days)

### Method

1. For each top-N vendor from R1, identify **3-5 real public job
   postings** that don't require login.
2. For each posting:
   - Open in a clean Chromium with `Page.captureSnapshot` (CDP)
   - Save the resulting MHTML to `tests/fixtures/captured/<vendor>/<sha>.mhtml`
   - Record: vendor, posting URL, capture date, classifier-relevant
     subset
3. Hand-label every form field with the expected classifier key:
   `tests/fixtures/captured/<vendor>/<sha>.expected.toml`

### Constraint

- **Only public postings.** No login-required pages.
- **No real submission.** Capture is read-only.
- **PII redaction step** before commit: scrub any company-internal
  IDs, applicant placeholder data, or session tokens from the MHTML.
- **Tenant attribution** preserved — we cite which company's posting
  the snapshot came from.

### Deliverable

- ~50 captured MHTML files, organized by vendor
- Sibling `.expected.toml` per file
- A `capture.sh` script so anyone can refresh the corpus

### Open question this answers

R2, R3. Replaces inferred fixtures with empirical ones.

### Sources / techniques

- [Chrome DevTools Protocol — Page.captureSnapshot](https://chromedevtools.github.io/devtools-protocol/tot/Page/#method-captureSnapshot)
- [Singlefile](https://github.com/gildas-lormeau/SingleFile) — alternative DOM capture
- Wayback Machine for older snapshots when current postings change

---

## 5. Phase R3 — Classifier accuracy benchmark (target: 2 days)

### Method

1. For each captured `.mhtml` from R2, render it (Chromium can load
   MHTML directly), run the snapshot script, get
   `Vec<FieldDescriptor>`.
2. Run `predict_field_key` on each.
3. Compare to the hand-labelled `expected.toml`.
4. Compute per-vendor accuracy + confusion matrix.
5. Identify the failure modes (R5).

### Deliverable

`docs/research/CLASSIFIER_ACCURACY_2026Q2.md`:
- Per-vendor accuracy %
- Confusion matrix (which key got mistaken for which other key)
- Per-field-type breakdown (email vs. phone vs. address sub-fields)
- Top-10 failure cases with the actual descriptor + expected/predicted

### Decision gate

- If accuracy <80% on canonical fields: classifier needs replacement
  (Phase R4 logistic regression).
- If accuracy ≥98% on canonical: classifier is good enough; focus
  shifts to free-form / anti-bot.

### Open question this answers

R4, R5. Tells us whether we ship the keyword classifier or train a
real model.

---

## 6. Phase R4 — Train a real classifier (target: 4 days)

Triggered by R3 if accuracy <98%.

### Method

1. **Data assembly.** R2 captures + Chromium's autofill heuristics
   test corpus (~185 fixtures, BSD) + USAJOBS public templates.
   Total target: 5,000+ labelled `(FieldDescriptor → key)` pairs.
2. **Feature engineering.** Hand-engineered features over the
   FieldDescriptor:
   - Tokenized label / placeholder / aria_label / name / id
     (lowercased, stripped of brackets)
   - Bigram tokens
   - HTML5 input type one-hot
   - Position features (form depth, sibling count)
3. **Model.** Multinomial logistic regression. Train via `candle` or
   `linfa`. Output: `Vec<f32>` weight matrix saved as `.safetensors`.
4. **Inference.** Sub-microsecond per field.
5. **Online updater.** SGD step per `Feedback` event.

### Constraint

- Must run **on the user's CPU** in <100 ms for full-form classification.
- Model size <500 KB so it ships in the binary.

### Deliverable

- `~/.atsisbroken/model.safetensors` (or baked-in default)
- `predict_field_key_v2(f) -> (String, f32)` returning (key, confidence)
- New unit tests against held-out R2 captures

### Decision gate

- If trained model accuracy <95%: investigate features, add data, retrain.
- If ≥98%: ship.

### Open question this answers

R4 again, but now closes it.

### Sources / techniques

- [candle — Hugging Face's Rust ML lib](https://github.com/huggingface/candle)
- [linfa — Rust ML toolkit](https://github.com/rust-ml/linfa)
- [Chromium autofill heuristics input/output corpus](https://chromium.googlesource.com/chromium/src/+/HEAD/components/test/data/autofill/heuristics) — BSD-3-Clause

---

## 7. Phase R5 — Multi-page Workday wizard (target: 2 days)

Workday is the most common dominant ATS in our R1 sample (per industry
reports). Today we render only the `contactInformationPage`. Real
Workday is 4-6 pages.

### Method

1. From R2 captures, identify Workday postings. Snapshot each page
   of the wizard.
2. Add per-page renderers to `kova::exopack::ats_fixtures::workday`:
   - `contact_information_page` (DONE)
   - `my_experience_page` (work history + education)
   - `application_questions_page` (per-tenant custom)
   - `voluntary_disclosures_page` (EEO)
   - `self_identification_page` (disability, veteran)
3. Render with `bottom-navigation-next-button` actually wired so a
   live test can advance through the wizard end-to-end.

### Deliverable

- Workday fixture renders 4 pages, each gated by its
  `data-automation-id` page wrapper.
- e2e test advances through all 4 with `Next` clicks via CDP.

### Open question this answers

R2 (Workday-specific deepening).

---

## 8. Phase R6 — Anti-bot fingerprint research (target: 3 days)

ATS vendors increasingly fingerprint browsers. We need to know what
defenses exist before we ship a tool that drives Chromium.

### Method

1. Read public research on ATS anti-bot fingerprinting:
   - [creepjs](https://abrahamjuliot.github.io/creepjs/) — fingerprint surface
   - [browserleaks](https://browserleaks.com/) — what's detectable
2. Test against real ATS forms (no submission): does the form load
   normally with a CDP-driven Chromium?
3. Identify which signals would flag automation:
   - `navigator.webdriver === true` (settable)
   - Headless Chrome user agent
   - Missing plugins / weird canvas hash
   - Mouse-move absence
   - Unrealistic typing speed
4. Document defenses and which we'll implement.

### Constraint

- We **do not** spoof user identity, defeat CAPTCHAs, or pretend
  to be human. We do remove automation-specific fingerprints
  that would falsely flag a legitimate user using a tool.

### Deliverable

`docs/research/ANTI_BOT_POSTURE.md`:
- Per-vendor fingerprint surface
- Which fingerprints we mitigate (e.g., set `navigator.webdriver=undefined`)
- Which we explicitly don't (CAPTCHA solving, mouse trail spoofing)
- Recommended chromiumoxide launch flags

### Open question this answers

R6.

---

## 9. Phase R7 — Demographics field safety audit (target: 1 day)

### Method

1. From R2 captures, flag every form field that's an EEO / AAP /
   self-identification field.
2. Verify the classifier's behavior on each: must return `unknown`
   (not classify as anything else, not `freetext`, not `address`).
3. Add explicit deny-list rules if any false positives.
4. Add an integration test that asserts the classifier returns
   `unknown` for every demographics field across all R2 captures.

### Deliverable

A test that REGRESSES if we ever start auto-filling EEO data.

### Open question this answers

R7. Critical for trust.

---

## 10. Phase R8 — Vendor selector decay study (target: 1 day, then ongoing)

### Method

1. From R1 / R2, identify which `data-automation-id` /
   `job_application[*]` selectors we depend on.
2. Use Wayback Machine to compare the same vendor's DOM over time
   (e.g., Workday 2024 vs. 2026).
3. Estimate per-vendor selector decay rate (how often selectors
   change vs. how often we'd need to update).
4. Build a **selector-drift detector** that runs against R2
   captures monthly via GitHub Actions and alerts when a previously-
   working selector stops matching.

### Deliverable

- Empirical decay rate per vendor
- A monitoring job that pages us when a vendor changes their DOM

### Open question this answers

R12. Critical for long-term reliability.

---

## 11. Phase R9 — Legal / ToS review (target: 2 days)

### Method

1. Read each vendor's Terms of Service:
   - Greenhouse, Lever, Workday, iCIMS, Ashby, Taleo, SuccessFactors
2. Identify automation clauses. (Most prohibit "scraping" but allow
   user-driven autofill.)
3. Identify whether our usage pattern (user-controlled CDP-driven
   fill, no submission without user click) falls within or outside
   their ToS.
4. Document per-vendor risk + mitigation.

### Constraint

- We are a free, user-controlled tool. The user types the URL.
  The user clicks Submit. We do not aggregate applicant data.
- That posture probably keeps us within most ToS — but "probably"
  isn't good enough. We need a written analysis.

### Deliverable

`docs/research/LEGAL_POSTURE.md` — per-vendor ToS summary +
position statement. **Reviewed by an actual lawyer** before public
launch (this is real legal exposure, not a doc to skip).

### Open question this answers

R9.

---

## 12. Phase R10 — Real recruiter / hiring manager interviews
        (target: 1 week, parallel to engineering)

### Method

1. Recruit 5-10 hiring managers / recruiters via:
   - r/recruiting and r/recruiters subreddits (post requesting interviews)
   - LinkedIn outreach
   - existing network
2. 30-minute structured interviews:
   - "What does an autofill-shaped application look like to you?"
   - "Have you ever rejected someone for filling-pattern signals?"
   - "What signals tell you a candidate used AI vs wrote it themselves?"
   - "What does 'verbatim from my own GitHub' read like to you?"
3. Synthesize patterns. Update `HIRING_MANAGER_ANALYSIS.md` with
   real quotes, not inferred ones.

### Deliverable

`docs/research/HIRING_MANAGER_INTERVIEWS_2026Q2.md` —
de-identified quotes, themes, surprising findings.

### Open question this answers

R11. Replaces inference with primary research.

---

## 13. Phase R11 — Beta cohort instrumentation (target: 2 weeks)

Triggered after R3-R7 close.

### Method

1. Recruit a 20-50 person beta cohort (volunteers, friends-of-author,
   r/cscareerquestions opt-in thread).
2. Ship to them with explicit opt-in telemetry under
   `FeedbackDelivery::SendWhenOnline` pointing at a Cochran-Block-
   controlled webhook.
3. Capture (with consent, only what they explicitly send):
   - Install completion / drop-off
   - Mode-graduation timing (TrainingWheels → Shadow → Chaos)
   - Per-vendor fill success rate
   - Classifier corrections per session
4. Analyze, iterate.

### Constraint

- **Opt-in only.** Default-off telemetry remains the product
  default forever.
- **No applicant data.** Only field shape + classifier keys, never
  the user-typed values.
- Beta participants see exactly what they're sharing.

### Deliverable

`docs/research/BETA_COHORT_RESULTS_<dates>.md` — usage metrics,
quality measures, what broke.

### Open question this answers

R8, R10.

---

## 14. Sequencing — walkthrough

The order isn't arbitrary. Each phase unblocks specific later
phases or eliminates a class of risk that would corrupt later
measurements. Walking through it:

### Step 1 — R1 (market census). Why first?

**Cheapest phase, sets focus.** Until we know that Workday is N% of
real US engineering job applications and Ashby is M%, we're guessing
at vendor priority. R1 takes a day; saves weeks of mis-investment.

**What it unblocks:** R2 (we capture the vendors we found), R5
(if R1 says Workday isn't dominant, we don't deepen it first).

**What kills it:** sample bias. Hacker News + LinkedIn aren't a
random sample of all jobs. We document the bias.

### Step 2 — R9 (legal / ToS review). Why second, parallel to R1?

**Capture is a legal action.** R2 wants to `Page.captureSnapshot`
real public postings. Most ToS forbid scraping. We need to read
each vendor's terms BEFORE we hit their pages, even read-only,
even publicly accessible.

**Why parallel to R1:** legal review is wallclock-bound by
counsel-availability, not by what we know about the market.
Start it early; results land alongside R1 closing.

**What it unblocks:** R2 capture protocol (which vendors? how?
attribution?), R11 beta launch (do we need a usage agreement?).

**What kills it:** finding a hard ToS prohibition we can't work
around. If a vendor blocks user-controlled CDP-driven autofill in
their ToS (none currently do, per surveyed work), we either
exclude that vendor or get explicit permission.

### Step 3 — R2 (real DOM capture). Why third?

**The empirical foundation.** Every later research phase
(R3 accuracy, R5 wizard, R6 anti-bot, R7 demographics, R8 decay)
queries against R2's captures. Without R2, every later phase is
back to inference.

**Gated by R1+R9:** R1 tells us *which* vendors to capture; R9
tells us we can.

**What it unblocks:** literally everything downstream.

**What kills it:** rate-limiting / Cloudflare blocks on capture.
If a vendor blocks us, we use Wayback Machine snapshots as a
secondary source; document confidence drop.

### Step 4 — R7 (demographics audit). Why immediately after R2?

**Trust-critical regression.** Demographics fields are the one
class of error that turns into a lawsuit. If our classifier
ever auto-fills "race" or "veteran_status," we're done.
R7 establishes the regression test before any other phase
touches the classifier — so any future change must keep this
property.

**Why before R3:** R3 might cause us to retrain (R4); we want
the demographics-skip property pinned before retraining could
accidentally violate it.

**What it unblocks:** R4 (the trained classifier inherits the
guardrail).

**What kills it:** finding the classifier *does* mis-classify a
demographics field. That's the bug R7 catches early.

### Step 5 — R3 (classifier accuracy benchmark). Why fifth?

**Decision gate for R4.** Run the existing keyword classifier
against R2 captures, measure per-vendor accuracy. Two outcomes:

- **≥98% on canonical fields:** we ship the keyword classifier;
  R4 deferred forever (or until accuracy drifts).
- **<98%:** R4 is mandatory.

**Why before R5/R6:** if we're going to retrain (R4), the trained
classifier needs to be in place before we deepen the Workday
fixture (R5) and run anti-bot tests (R6) — those phases query
the classifier during their own measurements.

**What it unblocks:** R4 (conditional), R5+R6 (always).

### Step 6 — R4 (train real classifier). Why sixth, conditional?

**Triggered by R3.** This is the heavy phase — feature engineering
+ training pipeline + online updater. ~4 days. We only do it if
R3 demands it.

**Why not before R3:** retraining without measurement is
cargo-culting. The classifier *might* already be good enough.
Measure first.

**What it unblocks:** R5 + R11 (beta needs a model that won't
embarrass us).

**What kills it:** model size > 500 KB or inference >100 ms —
we'd need to drop features. Document and try again.

### Step 7 — R5 (Workday multi-page wizard). Why seventh?

**Most-common-vendor depth.** R1 likely confirms Workday is the
single biggest vendor. Today our fixture renders 1 of 4 wizard
pages. R5 closes that gap so the e2e covers a real Workday flow,
not just contact-info.

**Why after R3/R4:** the trained classifier needs to handle the
wizard's later pages (experience / voluntary disclosures / self-id)
which have field shapes the contact-info page doesn't cover. Train
first, deepen second.

**What it unblocks:** R7 demographics audit can now run against the
voluntary-disclosures and self-identification pages, not just
contact-info.

### Step 8 — R6 (anti-bot fingerprint research). Why eighth?

**Survive real production.** Once we have a trained classifier
(R4) and a multi-page Workday flow (R5), we're ready to test
against real public postings (read-only). R6 is where we discover
which automation signals these vendors flag and which we mitigate.

**Why after R5:** R5 gives us a multi-step flow to test
fingerprint persistence across navigation — single-page testing
misses nav-time signals.

**Why parallel-able with R10:** R6 is engineering; R10 is human
research. They share no resources.

**What it unblocks:** safer beta (R11).

**What kills it:** finding fingerprints we can't ethically mitigate
(e.g., a vendor that requires legitimately-human canvas hashing).
We document the limit and accept the per-vendor failure mode.

### Step 9 — R10 (recruiter interviews). Why parallel from Step 7+?

**Wallclock-bound, content-independent.** Scheduling 5-10 real
recruiters takes 1-2 weeks regardless of where we are in
engineering. Start it as soon as R5 begins so it lands by R11.

**Why not earlier:** the questions we ask are sharper if we have
real captures (R2) to show recruiters — "look at this Workday
form, here's how our tool would fill it, what jumps out as
AI-flavored?"

**What it unblocks:** real quotes in `HIRING_MANAGER_ANALYSIS.md`.
R10 is the only phase that produces *primary research* — every
other phase is technical measurement.

### Step 10 — R8 (selector decay monitoring). Why tenth?

**Maintenance infrastructure.** Once R2 is stable, R8 stands up
the GitHub Action that re-captures the same postings monthly and
diffs against baselines. Selectors decay slowly; we need to know
when they decay so we can update fixtures before users notice.

**Why so late:** R8 is *ongoing* infrastructure, not a one-shot.
We need stable baselines (R2 stable) before we can monitor decay.

**What it unblocks:** post-1.0 reliability.

### Step 11 — R11 (beta cohort). Why last?

**The honest gate.** R3 says the classifier works. R5 says the
fixture covers Workday. R7 says we won't auto-fill demographics.
R6 says we survive anti-bot. R9 says we're not committing torts.
R10 says recruiters won't reject our outputs.

R11 is where 20-50 real applicants run the tool against real
postings (read-only — no actual submissions auto-clicked) and we
measure: install funnel completion, mode-graduation timing,
per-vendor success rate, classifier corrections per session.

**What it unblocks:** 1.0 release decision. Beta numbers either
support the §1 targets or they don't.

**What kills it:** install funnel collapses (Native Messaging
manifest is too hard for non-technical users) or per-vendor
success <80% (classifier or fixtures are wrong somewhere R3
didn't catch).

---

### Total wallclock estimate

- R1: 1 day
- R9: 2 days (calendar; lawyer-bound)
- R2: 3 days (after R1 closes)
- R7: 1 day (after R2 closes)
- R3: 2 days (after R2 closes)
- R4: 4 days (gated on R3 < 98%)
- R5: 2 days
- R6: 3 days
- R10: 5-10 days (calendar; recruiter-bound; runs parallel)
- R8: 1 day to scaffold, ongoing
- R11: 14 days (calendar; beta cohort)

Critical path: R1 → R2 → R3 → R4 → R5 → R6 → R11 = ~30 days.
With R9 and R10 running parallel: ~30 days total.

**No shipping under that bar.**

```
Week 1: R1 (1d) → R9 begins (legal, async) → R2 begins (3d) → R7 (1d)
Week 2: R3 (2d) → R4 if needed (4d) → R10 begins (recruiter outreach)
Week 3: R5 (2d) → R6 (3d) → R8 stands up
Week 4: R11 beta launch → instrument → analyze
```

R8 (decay monitoring) starts running on day 1 of week 3 and never
stops.

---

## 15. What gets dropped

A few things in current docs are speculation that should be
demoted until evidence supports them:

- **Android scaffold** — kept as scaffold but should not be promoted
  in marketing until R3+R5 close on desktop. Mobile is harder; we
  earn the right to go there after the desktop product is real.
- **GitHub-driven free-form composer** — still in `PLAN_PROFILE_AND_GITHUB.md`
  but blocked on R10 (real recruiter feedback) and R3 (a real
  classifier to route freetext fields).
- **Custom regex hooks (`custom_patterns.toml`)** — feature is fine
  but shipping it ahead of R3 is putting power-user knobs on a
  product whose default mode isn't proven yet. Defer.

---

## 16. Honesty bar going forward

Every PR claiming a vendor is "supported" must reference at least
one R2 capture + R3 measurement. No more "pattern-inferred"
fixtures shipped silently. The vendor list in `README.md` shows
**confidence levels**, not just names:

| Vendor       | Confidence | Backed by                                            |
|--------------|-----------|------------------------------------------------------|
| Greenhouse   | A         | R2 captures (n) + R3 accuracy (X%) + Hui's selectors |
| Lever        | A         | R2 captures (n) + R3 accuracy (X%) + Hui's selectors |
| Workday      | B         | R2 captures (n) + R3 accuracy (X%) — wizard partial  |
| iCIMS        | C → A     | (currently inferred; R2 will promote)                |
| Ashby        | C → A     | (currently inferred; R2 will promote)                |
| Taleo        | —         | not built; R1 may say it doesn't matter              |

If a vendor stays at `C` after R2 attempts to capture them, we
remove them from the supported list rather than ship guesses.

---

## 17. Decision: ship discipline

Until R3 closes with ≥98% on canonical fields, the README claims
"early experimental" — not "alternative to Simplify." We don't
position this as a Simplify replacement until measured accuracy
beats their freemium tier. Anything else is dishonest marketing.

This document is the contract. Every commit either advances a
phase, closes a research-debt item, or fixes a regression flagged
by R8 monitoring. No vibes-driven additions.
