# atsisbroken — Training Data: Responsibly-Sourced Options

What atsisbroken needs to train its field classifier:

> **(FieldDescriptor → key)** pairs — i.e., form-field DOM metadata
> (label, placeholder, aria_label, name, id, kind) paired with the
> Profile slot it should map to (`email`, `phone`, `freetext`, etc.).

We do **not** need: real applicant content (resumes, cover letters,
submitted answers). The classifier trains on the *shape of the
question*, not the answer.

## Risk tiers

| Tier | Description | Use? |
|---|---|---|
| 0 | Synthetic, authored by Cochran Block, public-domain | **Yes — primary** |
| 1 | US federal / public-domain government data | **Yes** |
| 2 | CC0 / Public Domain Dedication third-party datasets | **Yes** |
| 3 | CC-BY / CC-BY-SA datasets | **Yes** with attribution |
| 4 | Crawled public pages from sites whose ToS + robots.txt permit it | **Maybe** — case-by-case |
| 5 | Crawled pages with restrictive ToS | **No** |
| 6 | Anything containing applicant PII | **Never** |

## Concrete sources

### Tier 0 — Synthetic (recommended primary)

`assets/seed-corpus.jsonl` is the canonical example. Authored entirely
by Cochran Block. Unlicense (public domain).

**Strategy.** Hand-author a few hundred core patterns (covered already),
then templatically expand: each label has known synonyms ("Email",
"E-mail", "E-mail address", "Contact email"…) crossed with each
`name`/`id`/`aria_label` shape we've observed. A 200-row hand corpus
expands to ~5,000 synthetic pairs deterministically. Reproducibility
contract: a public seed + a public expander script ⇒ anyone can
verify the corpus.

**Why this is enough.** The classifier is a vocabulary mapper, not a
language model. Most ATS forms reuse the same ~50 patterns.

### Tier 1 — US federal / public-domain government

- **USAJOBS** — federal government job application schema. Forms and
  field metadata are **public domain** (works of the US government,
  17 USC §105). https://www.usajobs.gov/. Pull form HTML from public
  application templates; extract FieldDescriptors.
- **GSA forms catalog** — standardized federal forms (SF-86 etc.).
  Public domain. Contains canonical field names that ATS systems
  often mirror.
- **Schema.org** — `JobPosting` and `EmployeeRole` schemas are
  CC-BY-SA. Useful for `kind` taxonomy alignment (e.g., `email`
  vs `tel` vs `url` field kinds). https://schema.org/JobPosting.

### Tier 2 — CC0 datasets

- **Hugging Face Datasets, filtered to CC0** — search for "form"
  datasets with `cc0-1.0` license. Quality varies; vet each row.
- **OpenForms / Common Voice metadata** — tangentially useful; their
  form definitions sometimes contain canonical field labels.

### Tier 3 — CC-BY (with attribution)

- **Common Crawl** — petabyte-scale web archive. CC-BY-style terms.
  Contains snapshots of public job board pages (the *listing* pages,
  not the gated *application* pages). Filter to pages with `<form>`
  containing `<input type="email">` etc.; extract FieldDescriptors.
  Attribution requirement: cite Common Crawl in `LICENSE-PROVENANCE.md`.

### Tier 4 — Crawl with care

If we crawl ATS vendors directly:

1. Honor `robots.txt` always.
2. Hit only the **public job listing pages**, never the post-login
   application pages.
3. Rate-limit (1 req / 5 sec / domain).
4. User-Agent identifies the project + a contact email.
5. Discard any HTML byte not explicitly part of `<input>`,
   `<textarea>`, `<select>`, `<label>`, or their immediate parents.
   We never store applicant content.

Even after all of that, this tier requires per-vendor legal review
(Workday, Greenhouse, Lever, iCIMS, Taleo each have different ToS).
**Default: don't.**

### Tier 5/6 — Hard no

- LinkedIn / Indeed scraping for applicant data — no.
- Past applicant resumes / cover letters / real submissions — no.
- Datasets that bundle scraped resumes from public clouds — no, even
  if they claim "anonymized". We don't need that data.

## User-contributed feedback (opt-in)

The `FeedbackQueue` (already wired) is the highest-signal training
source we'll have once shipped. Users in `TrainingWheels` mode produce
labelled `(FieldDescriptor → expected_key)` pairs by saying yes/no.

**Privacy contract for user-contributed training data:**

1. **Local by default.** `FeedbackDelivery::LocalOnly`. Nothing leaves
   the device unless the user explicitly opts in.
2. **Opt-in destination.** `SendWhenOnline { destination }` requires
   the user to set a destination — `mailto:` or `https://`.
3. **Strip values.** When `sync` runs, only the
   `FieldDescriptor` (label/placeholder/aria/name/id/kind) and the
   predicted/actual *key* leave the device. **Never** the value the
   user typed into the field.
4. **Hash everything else.** Even within the FieldDescriptor, hash any
   `id` or `name` that looks like a user-specific token (`user_12345_email`).
5. **Public dataset, public PR.** If a contribution is accepted into
   the public seed corpus, it lands as a PR against
   `assets/seed-corpus.jsonl`. The user can audit before merge.

## Recommended bootstrap path

1. **Now (Tier 0):** expand `assets/seed-corpus.jsonl` from ~25 rows
   to 200 by hand. ~3 hours of work. Public domain. Sufficient to
   start training the on-device classifier and see real accuracy
   numbers on a few real ATS forms.
2. **Phase 2 (Tier 1):** scrape USAJOBS public application templates
   (federal, public domain). Produce another 300–500 rows.
3. **Phase 3 (user-contributed):** ship `TrainingWheels` mode; let
   real users contribute via opt-in `sync`. This is the long-term
   compounding advantage.
4. **Skip Tier 4 entirely** unless there's a compelling reason.
   Tier 0 + 1 + user feedback is enough for a credible product.

## What NOT to do

- Do **not** scrape applicant content from public clouds.
- Do **not** train on third-party datasets that include real resumes.
- Do **not** export user feedback that contains the user's own typed
  values, even if "anonymized".
- Do **not** ship a "telemetry by default" mode. The product's whole
  thesis is that the user owns their data.
