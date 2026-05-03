# atsisbroken — User-Story Analysis

> Adversarial decomposition. Every persona, every failure mode, every reason a
> user rage-uninstalls. If the scaffold cannot defend against these stories, it
> is not shippable.

## Personas

### P1 — The Volume Applicant ("Jordan, 23, recent grad")
- Submits 50–200 ATS applications per week.
- Has been burned by Simplify.us free-tier limits and by autofill that lies
  ("years of experience" hallucinated as 5 when resume says 1).
- **Win condition:** click "Add to Chrome", paste resume once, watch every
  Workday/Greenhouse/Lever/iCIMS form fill itself with byte-for-byte fidelity.
- **Loss condition:** any field gets data the user did not enter. Trust dies in
  one wrong autofill.

### P2 — The Privacy Hawk ("Mara, 38, security engineer")
- Will not put resume contents in any cloud service. Reads source before installing.
- **Win:** verifies via DevTools that the extension makes zero network calls
  during inference; verifies the .safetensors is bundled, not fetched.
- **Loss:** any outbound request to a non-target domain. One telemetry beacon
  and the extension is uninstalled and tweeted about.

### P3 — The Anti-SaaS User ("Devin, 31, indie dev")
- Refuses subscriptions on principle. Wants forever-free, no accounts.
- **Win:** zero signup, zero login, zero "premium feature" upsell.
- **Loss:** any modal asking for an email. One nag and they're gone.

### P4 — The Career Counselor ("Lena, 47, nonprofit")
- Helps a cohort of clients fill applications. Needs to manage multiple profiles
  on one machine without leaking data between them.
- **Win:** profile switcher in the extension popup; profiles are isolated in
  chrome.storage with explicit selection.
- **Loss:** Client A's resume bleeds into Client B's autofill.

### P5 — The Adversarial ATS ("Workday on a bad day")
- Renders fields late via JS. Uses non-standard `aria-*`. Hides labels in
  `<span>` siblings. Re-renders on focus, wiping autofill.
- **Win:** content script idempotent; re-detects fields after DOM mutations
  (MutationObserver); waits for `input` to be focusable before writing.
- **Loss:** field gets filled then immediately blanked by Workday's re-render.
  User assumes the extension is broken.

### P6 — The Recruiter-Side Auditor ("Hostile reviewer")
- Reviews applications and is suspicious of AI-filled forms. Wants a way to
  verify the human is real.
- **Implication:** atsisbroken must NEVER fabricate. It auto-fills only data
  the user explicitly entered. Free-text generation is opt-in per question and
  attributable to the user's own resume verbatim.

### P7 — The Browser-Diverse User ("Pat on Brave/Edge/Arc")
- Doesn't use stock Chrome.
- **Win:** MV3 is the lingua franca; extension installs on every Chromium fork.
- **Loss:** depends on a Chrome-only API (e.g., `chrome.aiOriginTrial`).

## Adversarial Scenarios (must survive)

| # | Scenario | Mitigation present in scaffold? |
|---|----------|---------------------------------|
| A1 | User pastes a 40-page resume | core/Profile holds raw_resume_text — unbounded String. ✅ accepted, no truncation |
| A2 | Form has a field with no label, only `placeholder="email address"` | predict_field_key matches placeholder. ✅ |
| A3 | Form has `aria-label="What is your email?"` and no `<label>` | predict_field_key matches aria_label. ✅ |
| A4 | Field uses `name="user_phone_mobile"` | "phone"/"mobile" keyword match. ✅ |
| A5 | Adversarial label: `<label>Email of your manager</label>` | predict_field_key returns "email" — wrong target. ❌ Known limitation; the trained model will resolve this via context. Documented as a model-resolves-it bug. |
| A6 | Field requests "salary expectation" — not in Profile schema | predict_field_key returns "" → extension does not autofill. ✅ correct behavior: never invent. |
| A7 | Network call attempted by core | core has no networking deps (only serde + wasm-bindgen). ✅ structurally impossible |
| A8 | Browser extension tries to fetch a model from CDN | Spec: model is bundled in the WASM. No `fetch()` for weights allowed. To enforce: CSP in manifest restricts `connect-src 'none'`. ⚠️ NOT YET WIRED |
| A9 | User reinstalls and loses profile | chrome.storage.local persists across reinstall on same browser profile. ✅ (when ext/ JS is wired) |
| A10 | Two tabs simultaneously autofill | content script must be idempotent. Each invocation reads chrome.storage fresh. ⚠️ design constraint, not yet enforced in code |

## What the current scaffold actually delivers

- ✅ Workspace compiles (native + wasm32-unknown-unknown).
- ✅ predict_field_key handles 9 of 10 keyword classes deterministically.
- ✅ JS↔WASM boundary exists (`predict_field_key_js`, `version_js`).
- ✅ Zero network surface in core.
- ❌ No background.js / content.js / popup.html — extension is non-functional.
- ❌ No real ML model — keyword heuristic only.
- ❌ No CSP hardening in manifest.
- ❌ No profile storage layer.

## P0 next moves (ordered by user-impact-per-LOC)

1. Wire ext/content.js → loads WASM, observes DOM, calls predict_field_key_js.
2. Wire ext/popup.html → resume paste box, profile editor, save to chrome.storage.
3. CSP-harden manifest.json (`connect-src 'none'`, `script-src 'self' 'wasm-unsafe-eval'`).
4. Replace keyword heuristic with custom field-deduction model trained via pixel-forge.
5. Add MutationObserver for re-rendering ATS forms (Workday).
