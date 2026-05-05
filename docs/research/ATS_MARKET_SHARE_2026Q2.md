<!-- Unlicense — cochranblock.org -->
<!-- Contributors: GotEmCoach, KOVA, Claude Opus 4.7 -->

# R1 — ATS Market Share (HN Sample, 2026 Q2)

**Date executed:** 2026-05-05
**Plan:** `docs/PERFECTION_PLAN.md` Phase R1
**Method:** scrape `Ask HN: Who is hiring? (May 2026)` (HN item
[47975571](https://news.ycombinator.com/item?id=47975571)),
extract every URL from the first 300 top-level comments, classify
by ATS host pattern.

---

## TL;DR

| Vendor          | Count | Share of classified | Confidence in our fixture |
|-----------------|------:|--------------------:|---------------------------|
| **Ashby**       |   106 |               48.8% | C (pattern-inferred) — **biggest gap** |
| **Greenhouse**  |    65 |               30.0% | A (production-source-cited) |
| **Lever**       |    11 |                5.1% | A (production-source-cited) |
| Workable        |     9 |                4.1% | not built |
| JazzHR          |     6 |                2.8% | not built |
| Recruitee       |     6 |                2.8% | not built |
| Notion forms    |     3 |                1.4% | n/a (not an ATS) |
| Breezy          |     3 |                1.4% | not built |
| SmartRecruiters |     2 |                0.9% | not built |
| Personio        |     2 |                0.9% | not built |
| **Workday**     |     2 |                0.9% | B (production-source-cited, partial wizard) |
| BambooHR        |     1 |                0.5% | not built |
| Rippling        |     1 |                0.5% | not built |
| **iCIMS**       |     0 |                0.0% | C (pattern-inferred) — **zero in this sample** |
| Taleo           |     0 |                0.0% | not built |
| SuccessFactors  |     0 |                0.0% | not built |

**ATS-vendor-matched:** 217 URLs  
**Custom careers pages** (hosted in-house, no recognizable ATS host): 186 URLs  
**Non-job-related URLs** (employer's blog post, company website, etc.): 280 URLs  
**Total URLs scraped:** 683 from 300 comments.

---

## Key findings

### 1. The vendor I'm least confident in is the most common one

Ashby — **48.8%** of the ATS-classified URLs — is the vendor my
fixture rated "Low / pattern-inferred" in
[`ATS_FIXTURE_SOURCES.md`](../ATS_FIXTURE_SOURCES.md). Neither of
my open-source extension citations covers Ashby. The fixture's
selectors (`firstName`, `phoneNumber`, `linkedinUrl`) are inferred
from public Ashby careers pages — we have not triangulated against
a working autofill extension that handles Ashby in production.

**Action:** Phase R2 must capture 5+ real Ashby postings before
anything else. Promote Ashby fixture from C → A *before* the next
ship.

### 2. The HN sample is heavily startup-biased

Workday is **0.9%** in this sample (2 URLs out of 217 classified).
That's not because Workday is unimportant — it's because F500
companies don't post on HN's "Who is hiring?" thread. Workday
dominates enterprise.

**Action:** the HN sample is **not** a substitute for a broader
sample. Plan: re-run R1 against:
- Indeed top-search results for "software engineer" (general market)
- LinkedIn jobs (broader still)
- USAJOBS (federal hiring; Workday-equivalent stack)

This first sample tells us what a typical-startup-applicant
encounters; the broader sample tells us what a Fortune-500
applicant encounters. **Both** matter for atsisbroken.

### 3. iCIMS / Taleo / SuccessFactors are zero in this sample

Same reason — these are F500 / legacy enterprise ATS. The HN
sample can't tell us their share. Plan: dedicated capture from
F500 careers pages (which often link to iCIMS/Taleo/SF).

### 4. ~46% of "job-related" URLs in this sample are custom careers pages

Hosted in-house (`https://posthog.com/careers`,
`https://railway.com/careers`, etc.). These pages might be
backed by Greenhouse/Lever/Ashby under the hood (an iframe or
form post), or they might be entirely custom HTML. **R2 needs
to capture these too** — atsisbroken will hit them whether or
not we plan to.

---

## Bias / caveats

| # | Bias                                                | Direction       |
|---|-----------------------------------------------------|-----------------|
| B1 | HN audience: tech / startup-skewed                 | Inflates Ashby, Greenhouse, Lever; deflates Workday, iCIMS, Taleo, SF |
| B2 | Engineering-roles-only sample                      | Same direction as B1 |
| B3 | Self-selected HN posters (small companies + alumni) | Probably inflates Ashby further |
| B4 | One-month snapshot (May 2026)                      | Vendor share might shift seasonally; won't be visible from one thread |
| B5 | URL-classification fragility                       | Vendors with non-obvious hosts (custom domains via CNAME) get counted as "custom careers" |

The findings stand for this sample. They do **not** generalize to
the full US engineering job market without broader sampling.

---

## Decisions this triggers

| Decision                                                      | Trigger                |
|---------------------------------------------------------------|------------------------|
| Promote Ashby fixture from C confidence to A                   | 49% of HN sample = priority 1 |
| Run R2 against ≥5 public Ashby postings BEFORE Workday wizard  | Same                   |
| Add Workable / JazzHR / Recruitee / Breezy fixture renderers   | Each has >2% share; combined ~12% |
| Re-run R1 with broader sources (LinkedIn / Indeed / USAJOBS)   | HN-bias caveat         |
| Re-prioritize PLAN_BROWSER_AUTOMATION Phase E (Workday wizard) | Workday only 0.9% here; not the dominant case for HN cohort. F500 broader sample needed. |
| Update README's "supported vendors" list to show evidence ranks | Honesty bar §16 of PERFECTION_PLAN |

---

## Reproduction

```bash
# Pull the May 2026 HN Who-is-hiring thread + extract URLs
python3 - << 'EOF'
import urllib.request, json, re, html, sys
from collections import Counter
thread_id = 47975571
with urllib.request.urlopen(f"https://hacker-news.firebaseio.com/v0/item/{thread_id}.json") as r:
    thread = json.load(r)
kids = thread.get('kids', [])
all_urls = []
for cid in kids[:300]:
    with urllib.request.urlopen(f"https://hacker-news.firebaseio.com/v0/item/{cid}.json", timeout=5) as r:
        c = json.load(r) or {}
    text = html.unescape(c.get('text', '') or '')
    hrefs = re.findall(r'href="([^"]+)"', text)
    bares = re.findall(r'https?://[^\s<>"\)]+', text)
    all_urls += list(set(hrefs + bares))
# (classification regex omitted; see source)
EOF
```

Source-of-truth raw URL list: `/tmp/hn_apply_urls.txt` after running
the reproduction step (regenerated each run because HN data changes).

---

## Sources cited

- HN thread: `Ask HN: Who is hiring? (May 2026)` —
  [item 47975571](https://news.ycombinator.com/item?id=47975571)
- HN Firebase API:
  [hacker-news.firebaseio.com/v0/item/{id}.json](https://github.com/HackerNews/API)

---

## Status

R1 (HN-cohort sample) — DONE.
R1-broader (Indeed / LinkedIn / USAJOBS) — TODO. Without it, we
have HN-applicant priorities, not US-applicant priorities.
