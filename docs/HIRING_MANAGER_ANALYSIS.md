<!-- Unlicense — cochranblock.org -->
<!-- Contributors: GotEmCoach, KOVA, Claude Opus 4.7 -->

# atsisbroken — Hiring Manager Analysis

**Date:** 2026-05-03
**Scope:** What hiring managers actually evaluate when reading the
applications atsisbroken produces — and how the product's outputs
land on the *receiving* side of the form. Companion to
`USER_STORY_ANALYSIS.md` (applicant-side personas P1–P7).
**References:** `PLAN_PROFILE_AND_GITHUB.md` (the verbatim-source
composer), `USER_STORY_ANALYSIS.md` P6 (Hostile Reviewer).

---

## Premise

Every application atsisbroken fills passes through a hiring manager
or recruiter. If our outputs read as **slop, generic, or AI-flavored**,
the candidate is rejected on the first scan — regardless of what the
applicant did or didn't do themselves. The product's value collapses
if it produces text that signals *automated*.

The verbatim-source composer (`PLAN_PROFILE_AND_GITHUB.md` §4) is
designed against a hiring manager rubric. This doc spells out that
rubric and verifies the composer satisfies it persona-by-persona.

---

## Hiring manager personas

### HM1 — Big-Tech Engineering Manager
*"Sarah, EM at a FAANG, fills two engineering reqs per quarter."*
- **First scan:** 30 seconds per resume. Looks for company names,
  YoE alignment, language matches.
- **Trusts:** specific project metrics, named open-source repos
  the candidate maintains, quotes that sound like an engineer wrote
  them ("rewrote the pricing pipeline to cut p99 by 40%").
- **Flags:** generic phrasing, missing GitHub link for the only
  project mentioned, unverifiable claims ("led team of 15").
- **atsisbroken outcome:** ✓ — composer cites repo by name, quotes
  the candidate's own commit messages. Sarah can click through and
  verify. The README-excerpt sentences read like the candidate
  wrote them because **the candidate did write them.**

### HM2 — Startup Founder
*"Devraj, founder, hiring engineer #4."*
- **First scan:** reads the entire application in 4 minutes; cares
  more about narrative coherence than keyword match.
- **Trusts:** voice consistency between cover letter and free-form
  answers, evidence of agency ("I noticed X was broken, so I…").
- **Flags:** GPT-flavored prose, "I'm passionate about", over-formal
  phrasing.
- **atsisbroken outcome:** ✓ — voice is preserved because every
  generated sentence is verbatim from the candidate's own sources.
  No paraphrasing layer to flatten voice. Cover letter stays out of
  scope (Section 10 of the plan); the candidate writes it themselves.

### HM3 — Big-Co Internal Recruiter
*"Marsha, recruiter at a Workday-using F500, screens 200 apps/week."*
- **First scan:** ATS-side keyword matching first, then human pass
  on top 30%.
- **Trusts:** correct field-by-field fills (no "Email" field with
  a phone number in it), structured data over free text.
- **Flags:** every field auto-filled with the same boilerplate,
  free-text answer reused word-for-word across multiple companies.
- **atsisbroken outcome:** ✓ — model emits `unknown` for fields
  it can't confidently classify, leaving them blank rather than
  jamming irrelevant data. Cached answers are explicitly
  per-question, not per-company, so reused answers are appropriate
  to the question (not the firm).

### HM4 — Specialized Tech Recruiter
*"Akiko, third-party tech recruiter placing senior IC roles."*
- **First scan:** reads the GitHub link. Often reads it before the
  resume.
- **Trusts:** activity recency, repo descriptions that match the
  candidate's stated stack.
- **Flags:** GitHub mostly forks, no original work, README written
  in a different voice from the application's free-form answers.
- **atsisbroken outcome:** ✓ — composer skips `is_fork: true`
  repos for project answers (`PLAN_PROFILE_AND_GITHUB.md` §4.2,
  candidate selection rule). Voice match is **structural**: if the
  README says "scaled to 50k users" and the application's free-text
  quotes that sentence, voice is identical because it's the same
  bytes.

### HM5 — Hiring Committee Reviewer (post-screen)
*"Jason, sees applications post-recruiter-screen, on a panel."*
- **First scan:** reads to confirm the recruiter's positive signal.
  Looks for a reason to *de-escalate*.
- **Trusts:** application coherence, plausible career progression.
- **Flags:** anything that smells off — wrong company name in the
  cover letter, a project description that doesn't match the
  GitHub repo it links to.
- **atsisbroken outcome:** ✓ — composer cites the repo URL inline.
  The project description IS the README excerpt. Coherence is
  enforced by sourcing.

### HM6 — Creative Agency Talent Manager
*"Priya, senior recruiter at a design agency, hiring a PM."*
- **First scan:** portfolio link, then writing samples.
- **Trusts:** specificity, narrative voice, "show don't tell."
- **Flags:** generic management prose, no portfolio link, "responsible
  for X" without outcomes.
- **atsisbroken outcome:** ⚠ — current Profile schema has portfolio
  + behance + dribbble fields but the answer composer doesn't yet
  pull from non-GitHub creative sources. Phase L.5 custom extractors
  let Priya's applicant write a regex over `profile:experience_bullets`
  that pulls outcomes-flavored sentences. Today: ✓ for IC engineering;
  ⚠ for creative roles until non-GitHub extractors land.

### HM7 — Government Hiring Officer
*"Kelvin, USAJOBS hiring officer."*
- **First scan:** veteran preference status, security clearance,
  exact wording on KSA narratives.
- **Trusts:** verbatim alignment with stated requirements; flags
  *embellishment* hard.
- **Flags:** any claim that exceeds documented credentials.
- **atsisbroken outcome:** ✓ — federal hiring is the strongest
  fit for verbatim-source composition. Kelvin can audit every
  free-form claim back to a source. The audit log
  (`freeform_audit.jsonl`) is exactly the receipt-trail federal
  reviewers expect.

---

## What HMs evaluate, mapped to atsisbroken outputs

| HM evaluation criterion              | atsisbroken output that satisfies it                              |
|--------------------------------------|-------------------------------------------------------------------|
| **Specificity**                      | Composer cites repo by name, quotes verbatim sentences            |
| **Verifiability**                    | URLs present in every cited source; audit log per generated answer |
| **Voice consistency**                | No paraphrasing layer; voice is the candidate's by construction   |
| **Narrative coherence**              | Same sources reused across answers when relevant — same person, same projects, same words |
| **No hallucinated metrics**          | LLM-tell deny-list catches confabulation; numbers must come from a source |
| **Field correctness**                | Classifier emits `unknown` rather than mis-fill                   |
| **Per-company adaptation**           | Question classifier routes to the right slot per question; user can override per-company via `custom_patterns.toml` |
| **Skip-rather-than-guess**           | `unknown` route is structural, not optional                       |

---

## AI-detection heuristics (and atsisbroken's defense)

Hiring managers (and the new wave of "AI cover-letter detectors")
key off a small set of patterns. atsisbroken's verbatim-only
discipline neutralizes most of them.

| HM heuristic for "AI-generated"            | atsisbroken defense                                                   | Verdict |
|--------------------------------------------|-----------------------------------------------------------------------|---------|
| "I am passionate about X"                  | Deny-list in `composed_answer_no_llm_tells` test                      | ✓ |
| "leveraged synergies to drive impact"      | Deny-list catches `leveraged`, `synergies`, `drive impact`            | ✓ |
| Suspiciously round metrics ("scaled to 1M users") | Numbers only emerge from a source; if README says it, it's true | ✓ if README/commit truthful |
| Generic project description                 | Composer pulls actual README excerpt; project IS specific             | ✓ |
| Voice mismatch resume↔cover letter          | atsisbroken doesn't write cover letters (out of scope)                | n/a |
| All applications worded identically         | Question classifier routes per question; same answer to same question is *correct* (consistent) | ✓ |
| Per-company personalization missing         | User adds `custom_patterns.toml` routes for company-name-aware slots  | ✓ if user opts in |
| Em-dashes everywhere                        | Composer uses `—` only inside quoted source content                   | ✓ |
| Three-bullet listicles                      | Composer doesn't generate listicles; it quotes prose                  | ✓ |
| "As an experienced X" opener                | Deny-list catches "as an experienced" / "as a seasoned"               | ✓ |
| Word-for-word match to a known LLM template | atsisbroken's emitted text appears in the candidate's GitHub or resume; not an LLM template | ✓ |

The structural property: **if you grep atsisbroken's output against
the candidate's GitHub + Profile, every word matches.** A hiring
manager checking "did the AI write this?" finds the answer is
"the candidate did, atsisbroken just chose which of their words to
quote."

---

## Translation table — applicant action → HM signal

| Applicant runs                                       | HM sees                                                       | HM signal |
|------------------------------------------------------|---------------------------------------------------------------|-----------|
| `atsisbroken init` from real resume                   | Application fields match resume word-for-word                 | "candidate is internally consistent" |
| `atsisbroken connect-github`                          | GitHub link works; described projects match the repos         | "this is real" |
| `atsisbroken go <ATS URL>` in TrainingWheels          | Per-field accuracy near 100%; nothing in wrong slots          | "candidate filled this carefully" |
| Free-form answer composed from README                 | Project description matches the linked repo's README          | "I can verify in 30 seconds" |
| Custom regex extractor pulls scaling claims           | Numerical claims trace to commit messages or README           | "metrics are sourced" |
| Application history ledger filled                    | If asked, candidate can produce a list of every application   | "organized job hunt" |
| Audit log present                                    | If accused of AI-generation, candidate produces receipts      | "honest above and beyond" |
| `unknown` fields skipped                              | Some fields blank; nothing inappropriate                      | "candidate had a strategy" (not "candidate was lazy") |

---

## Anti-patterns atsisbroken explicitly avoids

These are anti-patterns observed in commercial competitors and
flagged by HMs at scale.

| Anti-pattern                                                   | Status |
|----------------------------------------------------------------|--------|
| Auto-submit (sends without user review)                        | ✗ Constraint C1 in `PLAN_BROWSER_AUTOMATION.md` |
| Auto-fill demographics (race, gender, vet, disability)         | ✗ Profile §1.5 — never auto-fill, even in Chaos mode |
| Generative cover letter from skim of company page              | ✗ Out of scope; user writes cover letters |
| Mass-apply without rate-limiting                                | ✗ `--rate-limit=6/min` default in `cmd_go` |
| Generative paraphrasing of the candidate's resume               | ✗ Composer is verbatim-only |
| Inventing experience the candidate doesn't have                 | ✗ `unknown` route + deny-list |
| Storing applicant data in a third-party cloud                   | ✗ All-local; opt-in feedback only |
| Selling applicant data to recruiters                            | ✗ No commercial pipeline exists |
| Tracking which jobs the user applied to and selling to advertisers | ✗ Application ledger is local-only |

The list above is also the marketing differentiator. Other auto-fill
tools cross several of these lines.

---

## The "explainability" property

For any free-form answer atsisbroken composes, three pieces of
metadata are recorded and surface-able to the user:

1. **Source citation** — which repo / which commit / which README
   section / which Profile field.
2. **Extractor name** — built-in pattern or user-defined custom
   regex from `custom_patterns.toml`.
3. **Matched span** — the exact byte range of the source that was
   quoted.

When a hiring manager (or screening tool) asks "is this AI-generated?",
the candidate can answer:
> "I quoted myself. Here's the audit log; here are the source URLs.
> Every sentence is from my README, my commits, or my resume."

That's a defensible answer in front of any reasonable human
reviewer. Compare to:
> "I asked an LLM to draft it and tweaked from there."

Which is what every other auto-fill tool produces. The latter
forces the candidate into a bad-faith conversation. The former
makes atsisbroken users **more honest, not less**.

---

## Findings → Backlog

| Finding                                                                  | Action                                                                | Where it lands |
|--------------------------------------------------------------------------|-----------------------------------------------------------------------|----------------|
| Creative-agency HM6 ⚠ — composer doesn't pull non-GitHub portfolio data  | Phase L.5 custom extractor over `profile:experience_bullets`           | already tracked in PLAN_PROFILE_AND_GITHUB.md |
| Per-company adaptation requires manual `custom_patterns.toml`           | Future: optional auto-fetch of company About page → custom slot       | Phase 5 (Ecosystem) |
| Audit log not yet user-surfaced                                         | TUI tab "audit" or `atsisbroken audit --question "..."` subcommand     | New BACKLOG item |
| LLM-tell deny-list initial vocabulary needs research                    | Pull from real samples + recruiter blog posts; ship with seed corpus  | Phase K starter list |
| Applications ledger doesn't yet expose to TUI                           | TUI tab "applications" once Profile schema migration lands            | Phase L companion |

---

## Reproduction (for the candidate themselves)

To verify atsisbroken's outputs would land well with a hiring manager:

```sh
# 1. Generate a free-form answer
atsisbroken go https://boards.greenhouse.io/example/jobs/1234

# 2. Inspect the audit log for that session
cat ~/.atsisbroken/freeform_audit.jsonl | tail -20

# 3. For every generated sentence, confirm it appears in:
#    - Your GitHub README, OR
#    - One of your commit messages, OR
#    - A Profile field you wrote yourself
grep -F "<generated sentence>" ~/path/to/your/repos/*/README.md
grep -F "<generated sentence>" ~/.atsisbroken/profile.toml
```

If grep returns nothing, that's a bug. File it. The composer should
make grep return *something* on every emitted sentence.

---

## Sources cited externally

- [Sprout — AI Job Bots Guide (Feb 2026)](https://www.usesprout.com/blog/ai-job-application-bots-complete-guide):
  79% of orgs use AI in their ATS; 64% deploy auto-reject filters
  on content (not means).
- [Greenhouse — How ATS works (2026)](https://huntr.co/blog/how-applicant-tracking-systems-work):
  No bot-scoring auto-rejection; rejection is always human.
