<!-- Unlicense — cochranblock.org -->
<!-- Contributors: GotEmCoach, KOVA, Claude Opus 4.7 -->

# atsisbroken — ATS Fixture Sources

**Date:** 2026-05-05
**Scope:** Where the field selectors / DOM patterns in
`kova::exopack::ats_fixtures` come from. Every vendor renderer
mirrors selectors observed in real production extensions and
articles, attributed below.

The fixtures are still **mocks** — none of this is harvested live
from a vendor's site, all selectors are encoded in Rust strings,
none of it requires network. But the *patterns* (field name shapes,
attribute conventions, multi-page wrappers) trace to public sources
that have done the empirical work of reverse-engineering the real
forms.

---

## Greenhouse

**Primary source:** [`jeffistyping/workpls`](https://github.com/jeffistyping/workpls)
by **Jeffrey Hui**. `providers/greenhouse.js` matches against the
following on production Greenhouse application forms:

| Selector observed in the wild       | What it represents |
|-------------------------------------|--------------------|
| `document.forms['application_form']` | The outer form ID Greenhouse renders |
| `job_application[first_name]`       | First name input |
| `job_application[last_name]`        | Last name input |
| `email`                             | Bare email input (not nested) |
| `phone`                             | Bare phone input |
| `urls[LinkedIn]`                    | LinkedIn URL block |
| `urls[Github]`, `urls[Github ]`     | GitHub URL — quirk: trailing-space variant exists when companies add it via the Greenhouse admin UI |
| `urls[Twitter]`                     | Twitter URL block |
| `job_application[answers_attributes][N][text_value]` | Numerically-indexed free-text answers; N depends on how many custom questions the company added |
| `job_application[gender]`, `[hispanic_ethnicity]`, `[race]`, `[veteran_status]`, `[disability_status]` | EEO / demographics |

Hui's quote (verbatim from the source):
> "This is a url field added by the company who listed the posting,
>  so there's sometimes some whitespace/capitalization differences.
>  It's common enough in enough engineering applications that I felt
>  like it was necessary to include."

**Secondary source:** [`josephajibodu/greenhouse-autofill-chrome-extension`](https://github.com/josephajibodu/greenhouse-autofill-chrome-extension)
by **Joseph Ajibodu**. Confirms the form-name + nested-bracket pattern.

---

## Lever

**Primary source:** [`jeffistyping/workpls`](https://github.com/jeffistyping/workpls)
by **Jeffrey Hui**. `providers/lever.js`:

| Selector observed in the wild | What it represents |
|-------------------------------|--------------------|
| `document.forms[0]`           | First form on the page (no form id) |
| `name`                        | **Single** combined name field — Lever doesn't split first/last |
| `email`, `phone`              | Bare inputs |
| `urls[LinkedIn]`              | LinkedIn URL |
| `urls[Github]`, `urls[Github ]`, `urls[GitHub]` | GitHub URL — three variants Hui handles in the wild (capitalization + trailing-space) |
| `urls[Twitter]`               | Twitter URL |
| `cards[<id>][field<n>]`       | Custom per-posting questions added by the hiring company |
| `comments`                    | Free-text "additional information" textarea |

The single-`name`-field pattern is the most surprising vendor
difference and the one most easily missed by naive autofillers.

---

## Workday

**Primary source:** [`ubangura/Workday-Application-Automator`](https://github.com/ubangura/Workday-Application-Automator)
by **Nathaniel Ubangura**. `apply.js` uses Playwright against real
Workday tenants and the canonical attribute is `data-automation-id`
(used everywhere — fields, page wrappers, navigation buttons).

| Page-level selector observed                              | What it gates |
|-----------------------------------------------------------|---------------|
| `div[data-automation-id="contactInformationPage"]`        | "My Information" page (name + email + phone + address) |
| `div[data-automation-id="myExperiencePage"]`              | Experience / education |
| `div[data-automation-id="voluntaryDisclosuresPage"]`      | EEO / demographics |
| `div[data-automation-id="selfIdentificationPage"]`        | Disability / veteran / gender disclosures |

| Field selector observed                                   | What it represents |
|-----------------------------------------------------------|--------------------|
| `input[data-automation-id="legalNameSection_firstName"]`  | First name |
| `input[data-automation-id="legalNameSection_lastName"]`   | Last name |
| `input[data-automation-id="email"]`                       | Email (different from `primaryEmail` aria-label!) |
| `input[data-automation-id="password"]`                    | Account-creation password |
| `input[data-automation-id="phone-number"]`                | Phone number |
| `button[data-automation-id="phone-device-type"]`          | **`<button>`, not `<select>`** — Workday's "selects" are buttons that open a custom dropdown |
| `input[data-automation-id="addressSection_addressLine1"]` | Street |
| `input[data-automation-id="addressSection_city"]`         | City |
| `input[data-automation-id="addressSection_postalCode"]`   | Zip |
| `button[data-automation-id="addressSection_countryRegion"]` | Country picker (button-as-dropdown) |
| `button[data-automation-id="utilityButtonSignIn"]`        | Sign-in entry |
| `button[data-automation-id="signInSubmitButton"]`         | Submit credentials |
| `a[data-automation-id="applyManually"]`                   | "Apply manually" gate (vs. apply with LinkedIn/Indeed) |
| `button[data-automation-id="bottom-navigation-next-button"]` | Multi-page wizard advance |

Ubangura's automator demonstrates that **every interactive element
in a Workday tenant carries a stable `data-automation-id`** —
including buttons that look like selects. Our fixture mirrors this
convention faithfully, including the button-as-dropdown for
phone-device-type and country-region.

---

## iCIMS

**Source for behavioral patterns:** ["Taleo & iCIMS Made You Waste 45
Minutes. On Purpose"](https://aiapplyd.com/blog/taleo-icims-worst-application-systems-2026)
by **Ava Bagherzadeh** (April 6, 2026, AI Applyd Blog).

Behavioral observations (no specific selectors documented):
- Account creation **before** viewing the full job description
- Profile sync issues across companies using the same iCIMS deploy
- "Tiny text inputs" + "dropdown menus that load like dial-up"
- Resume parsing that "claims to populate fields" but leaves half empty

Quote, verbatim:
> "Upload your resume. The system says 'We'll parse your information.'
>  You click next. Half the fields are empty."

Our fixture uses iCIMS's documented `<fieldset>` + `<legend>`
grouping convention (visible in their public design docs and
inspectable on any iCIMS-hosted job page) and a stable `iCIMSField_<n>`
naming scheme. The numeric IDs commonly seen in production
(`iCIMSField_1234`) are easy to mimic but add no test value over
semantic IDs.

**Gap to acknowledge:** We don't have a public open-source autofill
extension to triangulate iCIMS selectors against. Patterns here are
inferred from the article + observed iCIMS markup. Higher confidence
patterns would come from one of the commercial extensions
(EarnBetter, Simplify Copilot) — they're closed-source so we can't
cite them line-by-line.

---

## Ashby

**Source for behavioral patterns:** Same Bagherzadeh article and
public Ashby job pages (e.g., careers pages built on Ashby).

Behavioral observations:
- Modern React-shaped components
- `role="combobox"` on category fields where other vendors use
  `<select>`
- Custom dropdown lists rendered as `<ul role="listbox">`
- Stable semantic field names (`firstName`, `lastName`, `email`,
  `phoneNumber`, `linkedinUrl`, `githubUrl`)
- "Section ID" wrappers per-category (e.g., `_field` class names
  in our fixture mirror their `_field` / `_questions` structure)

**Gap to acknowledge:** Same as iCIMS — no open-source autofill
extension to cite. Ashby's React rendering and custom combobox
behavior are visible in any Ashby careers page's DOM but we
haven't found a public extension that documents the specific
internal class names. Best-effort inference.

---

## What we are NOT claiming

- These fixtures are not faithful reproductions of the full
  multi-step ATS flows. Workday in production has ~5 pages per
  application, custom hydration timing, server-side validation,
  and tenant-specific custom fields. Our fixture is one page.
- The fixtures don't exercise anti-bot fingerprinting (mouse
  trails, timing checks, canvas fingerprints).
- Demographic / EEO fields are present in some renderers
  (Greenhouse) but we don't autofill them under any condition —
  see `PLAN_PROFILE_AND_GITHUB.md §1.5`.

## What we ARE claiming

- The **field name patterns** mirror what production extensions
  match against in production tenants.
- The **selector strategies** (`data-automation-id` for Workday,
  bracket-bracket for Greenhouse, single-`name` for Lever) are
  faithful to the vendors' actual conventions.
- The fixtures are **good enough to catch classifier regressions
  against vendor-specific naming patterns** — proven by the live
  e2e test catching the substring-`tel`-inside-`websiteLinkedIn`
  bug that no synthetic test would have surfaced.

## Forward work

- Capture real public Greenhouse / Lever / Workday job postings via
  `Page.captureSnapshot` MHTML and store sanitized snapshots in a
  separate `fixtures/captured/` tree. Higher fidelity at the cost
  of being tied to specific tenants. Worth doing once the live
  fill loop ships.
- Add **multi-page** Workday flow (`contactInformationPage` →
  `myExperiencePage` → `selfIdentificationPage`) with the next-button
  navigation, so the full wizard can be exercised end-to-end.
- iCIMS / Ashby fixtures need triangulation against a live tenant
  before we trust their selectors.

---

## Sources cited (verbatim)

- [Jeffrey Hui — `jeffistyping/workpls`](https://github.com/jeffistyping/workpls) — Greenhouse + Lever autofill extension. License visible in repo.
- [Joseph Ajibodu — `josephajibodu/greenhouse-autofill-chrome-extension`](https://github.com/josephajibodu/greenhouse-autofill-chrome-extension) — Greenhouse-specific extension.
- [Nathaniel Ubangura — `ubangura/Workday-Application-Automator`](https://github.com/ubangura/Workday-Application-Automator) — Playwright-based Workday automator.
- [Ava Bagherzadeh — "Taleo & iCIMS Made You Waste 45 Minutes. On Purpose"](https://aiapplyd.com/blog/taleo-icims-worst-application-systems-2026) — AI Applyd Blog, April 6 2026.
