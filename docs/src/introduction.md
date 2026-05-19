# atsisbroken

ATS scraper and job application automation. Breaks applicant tracking systems so candidates can actually apply.

## What It Is

atsisbroken is a Rust CLI + browser automation tool that identifies why job applications fail in ATS systems and routes around the breakage. It scrapes ATS-hosted job postings, normalizes the data, and surfaces actionable intelligence about which systems are hostile to applicants.

Supported ATS targets: Workday, Greenhouse, Lever, Ashby, iCIMS.

## CLI Surface

```
atsisbroken init       # set up local profile
atsisbroken status     # current queue state
atsisbroken run        # execute application pipeline
atsisbroken sync       # pull latest job data
atsisbroken graduate   # mark application complete
atsisbroken speak      # generate cover text
```

## Architecture

Browser automation via CDP (Chrome DevTools Protocol). No Selenium, no Playwright — direct WebSocket to the browser. Single binary, no runtime dependencies beyond a Chrome/Chromium install.
<!-- COCHRANBLOCK-BRAND-FOOTER:START -->

---

<sub>&#9656; **THE COCHRAN BLOCK, LLC** &#183; CAGE `1CQ66` &#183; UEI `W7X3HAQL9CF9` &#183; UNLICENSE &#183; [cochranblock.org](https://cochranblock.org)</sub>
<!-- COCHRANBLOCK-BRAND-FOOTER:END -->
