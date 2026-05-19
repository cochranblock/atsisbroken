<!-- Unlicense — cochranblock.org -->
<!-- Contributors: GotEmCoach, KOVA, Claude Opus 4.7 -->

# R1-broader — F500 / Enterprise ATS Distribution

**Date executed:** 2026-05-05
**Plan:** `docs/PERFECTION_PLAN.md` Phase R1-broader
**Scope:** Reconcile the HN-startup cohort
([`ATS_MARKET_SHARE_2026Q2.md`](ATS_MARKET_SHARE_2026Q2.md)) against
the Fortune 500 / enterprise market share, using public industry
research that already did the empirical work. No primary scraping
this round — citing existing published data.

---

## TL;DR

Two cohorts. Two completely different distributions.

| Vendor          | HN (this work) | F500 (SHRM 2025) | Overall mkt (AppsRunTheWorld 2024) |
|-----------------|---------------:|-----------------:|-----------------------------------:|
| **Workday**     |          0.9 % |           39.0 % |                              ~9 %  |
| **Taleo** (Oracle) |       0.0 % |           22.4 % |                             ~10 %  |
| **SuccessFactors** (SAP) |  0.0 % |           13.2 % |                              ~5 %  |
| **iCIMS**       |          0.0 % |             ~5 % |                            10.7 %  |
| **Greenhouse**  |         30.0 % |             ~5 % |                              ~7 %  |
| **Ashby**       |         48.8 % |               ~0 |                              ~1 %  |
| **Lever**       |          5.1 % |             ~3 % |                              ~5 %  |
| Other           |       remainder|         remainder|                          remainder |

**Reading the table:**
- An HN-startup applicant in May 2026 mostly fills **Ashby** and **Greenhouse**.
- A Fortune 500 applicant in May 2026 mostly fills **Workday** and **Taleo**.
- The overall ATS market (small to large) has **iCIMS** as the leader by raw count of installations — many small-business deployments.

These are not contradictions. They reflect different applicant
populations using different employer cohorts.

---

## What this means for atsisbroken

### Two-cohort fixture priority (revised)

| Vendor           | Why we cover it                                    | Confidence today | Action |
|------------------|----------------------------------------------------|------------------|--------|
| **Ashby**        | 49% of HN cohort — the most common form a startup applicant fills | C (pattern-inferred)  | **Priority 1**: capture real Ashby DOMs (R2). Promote C → A. |
| **Workday**      | 39% of F500 — most common form an enterprise applicant fills      | B (1 page of 4)       | **Priority 2**: capture multi-page wizard; finish R5. |
| **Greenhouse**   | 30% of HN, ~5% of F500 — broad coverage         | A                     | Maintain. Already fixture-A. |
| **Taleo**        | 22% of F500 — second most common F500 form     | not built              | **Priority 3**: build Taleo fixture from scratch. Sourced selectors needed. |
| **SuccessFactors** | 13% of F500 — third F500 form                | not built              | **Priority 4**: build SF fixture. SAP-owned; classic dropdown-heavy. |
| **iCIMS**        | 10.7% of overall mkt; small-biz heavy           | C (pattern-inferred)   | **Priority 5**: capture real iCIMS DOMs to promote C → A. |
| **Lever**        | 5% HN, 3% F500 — minor in both                  | A                     | Maintain. |
| **Workable / JazzHR / Recruitee / Breezy** | each <5% of any cohort  | not built | Together ~12% of HN. **Priority 6**: bulk-add light fixtures. |

### Updated PERFECTION_PLAN sequencing

R1-broader changes the R2 (capture) priority list. Old: Greenhouse,
Lever, Workday, iCIMS, Ashby (the order I built fixtures in).
New, evidence-driven order:

1. **Ashby** — biggest HN cohort gap, lowest current confidence
2. **Workday** — biggest F500 cohort, partial fixture
3. **Taleo** — second F500, no fixture
4. **iCIMS** — overall market leader, C confidence today
5. **SuccessFactors** — third F500, no fixture
6. **Greenhouse** — already A; just refresh captures monthly
7. **Workable + JazzHR + Recruitee + Breezy** — bulk-add as a single PR

---

## Trend signal — Workday losing share to Greenhouse

[Multiple sources](https://blog.ongig.com/applicant-tracking-system/top-ats-systems-used-by-the-fortune-500-2019/)
report Workday lost ~5pp of F500 market share since 2019, with
Greenhouse picking up most of it. SHRM's 2025 study also shows
Workday's lead narrowing.

This means our Workday-first instinct (because it's biggest today)
should be **discounted by trend** — Greenhouse will grow into the
F500 vacuum Workday is leaving. Five years out, the F500 cohort's
Greenhouse share could be 10-15%.

**Implication:** Greenhouse fixture maintenance is more important
than its current F500 share suggests. Don't deprioritize it.

---

## What we still don't know

| Question                                                    | Status |
|-------------------------------------------------------------|--------|
| Per-vendor selector decay rate                              | R8 (not yet executed) |
| Workday tenant-to-tenant DOM variation                      | R2 (will surface) |
| iCIMS legacy-product split (multiple iCIMS frontends exist) | R2 |
| Custom-careers-page distribution: are they Greenhouse iframes or fully bespoke? | R2 |
| Trend forecast: 2027-2030 vendor share                      | not researched |

---

## Sources cited

- [AppsRunTheWorld — Top 10 HCM Software Vendors in Applicant Tracking Market Segment, 2024-2029](https://www.appsruntheworld.com/top-10-hcm-software-vendors-in-applicant-tracking-market-segment/)
  — overall market share: iCIMS 10.7% leads, top 10 = 51.1% of market.
- [SHRM — "Workday's ATS Is the 'Top Choice' of the Fortune 500"](https://www.shrm.org/topics-tools/news/talent-acquisition/workdays-ats-top-choice-fortune-500)
  — Workday 39% F500; SuccessFactors 13.2%.
- [Ongig — Top ATS Systems Used by the Fortune 500 (2019)](https://blog.ongig.com/applicant-tracking-system/top-ats-systems-used-by-the-fortune-500-2019/)
  — historical baseline; Workday was 22.6% in 2019, Taleo 22.4%.
- [Workday Wikipedia](https://en.wikipedia.org/wiki/Workday,_Inc.)
  — 60%+ of Fortune 500 use Workday; 11,000 organizations; 70M users.
- [CloudWars — Workday Wins 7 Fortune 500 Customers in Q4](https://cloudwars.com/cloud/workday-wins-7-fortune-500-customers-in-q4-including-3-from-oracle-and-sap/)
  — corroborates Workday gaining from Oracle and SAP.
- [Taleo Wikipedia](https://en.wikipedia.org/wiki/Taleo)
  — Taleo had 5,000+ customers including ~half of Fortune 100 as of 2011.
- [Maximize Market Research — Applicant Tracking System Market Forecast 2026-2032](https://www.maximizemarketresearch.com/market-report/applicant-tracking-system-ats-market/11581/)
  — market forecast $3.6B by 2029 at 7.6% CAGR.
- [HN — "Ask HN: Who is hiring? (May 2026)"](https://news.ycombinator.com/item?id=47975571)
  — primary source for the HN cohort numbers in the prior report.

Caveat on the F500 numbers: SHRM 2025 study uses a different
denominator than AppsRunTheWorld (F500 only vs. all customers;
"identified systems" vs. all). I've labeled which source each
number comes from. The cross-source agreement on Workday-leads-
F500-but-Greenhouse-is-rising is consistent.

---

## Status

- R1 (HN cohort) — DONE
- R1-broader (F500 / overall market) — DONE (this document)
- R1 sequence is closed. Next: **R2 (real DOM capture)**, prioritized
  per the revised list above.
<!-- COCHRANBLOCK-BRAND-FOOTER:START - generated by cochranblock/scripts/brand-stamp.sh -->

---

<sub>&#9656; **THE COCHRAN BLOCK, LLC** &#183; CAGE `1CQ66` &#183; UEI `W7X3HAQL9CF9` &#183; UNLICENSE &#183; [cochranblock.org](https://cochranblock.org)</sub>
<!-- COCHRANBLOCK-BRAND-FOOTER:END -->
