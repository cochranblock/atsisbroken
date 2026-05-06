<!-- Unlicense — cochranblock.org -->
<!-- Contributors: GotEmCoach, KOVA, Claude Opus 4.7 -->

# R2 — Real DOM Capture Progress

**Date executed:** 2026-05-05 (rolling)
**Plan:** `docs/PERFECTION_PLAN.md` Phase R2
**Priority order:** Per [R1-broader](ATS_MARKET_SHARE_F500_2026.md):
**Ashby → Workday → Taleo → iCIMS → SuccessFactors → Greenhouse → Lever → tail**.

---

## Vendor: Ashby — first capture done

**Captured:** 2026-05-05 from
[`https://jobs.ashbyhq.com/lago/638af98e-061c-4f8b-adf0-ec23311ea2a4/application`](https://jobs.ashbyhq.com/lago/638af98e-061c-4f8b-adf0-ec23311ea2a4/application).
Lago is a public Y-Combinator-backed open-source company. Public
posting; no auth required.

**Stored at:** `tests/fixtures/captured/ashby/lago.html` (dump-dom output).

### What's NEW vs my pattern-inferred fixture

| Property                                | Pattern-inferred fixture (before)             | Live capture (now)                                                |
|----------------------------------------|------------------------------------------------|-------------------------------------------------------------------|
| Wrapper class                          | `_field`                                       | **`ashby-application-form-field-entry`** (stable suffix)          |
| Form container class                   | `ashby-application-form`                       | **`ashby-application-form-container`**                            |
| Section grouping                       | (none)                                         | **`ashby-application-form-section-container`**                    |
| Label class                            | (default)                                      | **`ashby-application-form-question-title`**                       |
| Submit button class                    | (default)                                      | **`ashby-application-form-submit-button`**                        |
| Resume upload                          | not modeled                                    | **`ashby-application-form-autofill-pane`** — top-level dropzone   |
| Name fields                            | first + last (separate, like Greenhouse)       | **single `Name` field** (like Lever)                              |
| Form labels                            | "First Name" / "Last Name" / etc.              | "Name", "Email", "Resume", "LinkedIn Link", "GitHub Link", "Where are you based?" |
| `aria-label`                           | every field                                    | **none** — Ashby relies on `<label for>` semantic HTML            |
| Placeholders                           | vendor-specific patterns                       | generic — "Type here…", "hello@example.com…", "https://example.com…" |
| Address                                | structured (street/city/zip)                   | **single "Where are you based?" text field**                       |
| `role="combobox"`                      | speculated                                     | **not seen** in this tenant; Ashby uses real `<select>` here       |

### What that means for the fixture

`kova::exopack::ats_fixtures::render_ashby` rewritten this commit:
- Uses the **stable `ashby-application-form-*` class names**, not
  the React-hashed `_xd2v0_1` prefixes (those drift per release).
- Single `Name` field (drops the `last` slot for Ashby in
  `expected_keys`).
- Label-based, no aria-label.
- Generic placeholders matching production text.

Confidence promoted: **C (pattern-inferred) → B (one tenant verified)**.
A confidence requires 3+ tenants. PermitFlow + Charge Robotics
captures attempted but rendered minimal HTML on the first attempt
(likely React hydration timing); deferred for re-capture.

### Where this lands the e2e

Live e2e test (`tests/ats_e2e.rs`) still passes against the
updated fixture: 5/5 vendors classify + fill + screenshot
correctly. The Ashby render now reflects real selectors observed
in production.

---

## Vendor: Greenhouse — refresh capture (skipping for now)

Already A-confidence (Hui + Ajibodu source-cited). R2 backlog
puts it lower — refresh from a live posting once the higher-priority
vendors close.

---

## Pending captures (priority order, R2 backlog)

1. **Ashby** — 2 more tenants (PermitFlow, Charge Robotics) —
   re-capture with longer wait OR via chromiumoxide's
   `Page::wait_for_navigation` instead of `--virtual-time-budget`.
2. **Workday** — needs a public Workday tenant URL. From R1's
   sample, only 2 Workday URLs — both need login. **Action:** identify
   public Workday-hosted F500 careers pages from the Workday W-list
   that don't require login.
3. **Taleo** — pull from Oracle's Taleo customer set. Many F500
   companies use Taleo and their job postings are public.
4. **iCIMS** — same approach.
5. **SuccessFactors** — same approach.

---

## Methodology notes

- Used `chromium --headless --dump-dom --virtual-time-budget=15000`
  for first attempts. Works for stable React forms (Ashby Lago) but
  loses minimal-HTML pages where the form mount happens after the
  budget elapses.
- For unreliable captures, will switch to `chromiumoxide` with
  `Page::wait_for_navigation` + 2s settle + `Runtime.evaluate` to
  get `document.documentElement.outerHTML`. More code, more
  reliable.
- All captures are **read-only** — no submission, no account
  creation, no form POST. Page rendering only.
- Captures stored under `tests/fixtures/captured/<vendor>/`.

---

## Status

R2 — IN PROGRESS:
- Ashby: 1/3 tenants captured. Fixture updated. Confidence C → B.
- Workday: 0 captures yet.
- Taleo / iCIMS / SuccessFactors / Greenhouse refresh: 0 captures yet.

Next session: 2 more Ashby tenants (chromiumoxide-based capture) +
1 Workday public posting + 1 Taleo public posting.
<!-- COCHRANBLOCK-BRAND-FOOTER:START - generated by cochranblock/scripts/brand-stamp.sh -->

---

<sub>&#9656; **THE COCHRAN BLOCK, LLC** &#183; CAGE `1CQ66` &#183; UEI `W7X3HAQL9CF9` &#183; UNLICENSE &#183; [cochranblock.org](https://cochranblock.org)</sub>
<!-- COCHRANBLOCK-BRAND-FOOTER:END -->
