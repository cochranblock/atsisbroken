# atsisbroken — User-Story Analysis

> Adversarial decomposition. Every persona, every failure mode, every reason a
> user rage-uninstalls. If the scaffold cannot defend against these stories, it
> is not shippable.

**Architecture in scope.** Single Rust binary that drives Chromium via the
DevTools Protocol (`chromiumoxide`). The classifier is **trained locally**
from the user's resume + a small seed corpus shipped with the binary. No
baked third-party weights. Three autonomy modes:

- **`TrainingWheels`** — yes/no per fill, supervised online learning.
- **`Shadow`** — user fills manually, binary watches and learns from each
  keystroke (`Observation` events). Once classifier confidence on a given
  key crosses `ConfidenceThreshold` (default 0.85), that field type
  auto-fills on the next sighting. The model graduates one field at a
  time — email might go autonomous after 12 manual fills while
  free-text-essay stays user-driven for months.
- **`Chaos`** — fully autonomous on every classified field.

## Personas

### P1 — The Volume Applicant ("Jordan, 23, recent grad")
- Submits 50–200 ATS applications per week.
- Has been burned by Simplify.us free-tier limits and by autofill that lies.
- **Win condition:** download one binary, paste resume once, watch every
  Workday/Greenhouse/Lever/iCIMS form fill itself with byte-for-byte fidelity
  to the resume the user pasted.
- **Loss condition:** any field gets data the user did not enter.

### P2 — The Privacy Hawk ("Mara, 38, security engineer")
- Will not put resume contents in any cloud service. Audits with `tcpdump`.
- **Win:** binary makes zero network calls outside the local CDP socket and
  the in-page `Network.*` traffic the browser was already going to make.
  Verifies the model is bundled in the binary, not fetched.
- **Loss:** any outbound request to a non-target domain. One telemetry beacon
  and the binary is uninstalled and tweeted about.

### P3 — The Anti-SaaS User ("Devin, 31, indie dev")
- Refuses subscriptions on principle. Wants forever-free, no accounts.
- **Win:** zero signup, zero login, zero "premium feature" upsell.
  Profile is a local TOML file the user owns.
- **Loss:** any modal asking for an email.

### P4 — The Career Counselor ("Lena, 47, nonprofit")
- Helps a cohort of clients fill applications.
- **Win:** profile lives at `--profile path.toml`; switch profiles per client
  with a CLI flag.
- **Loss:** Client A's resume bleeds into Client B's autofill.

### P5 — The Adversarial ATS ("Workday on a bad day")
- Renders fields late via JS. Hides labels in `<span>` siblings. Re-renders
  on focus.
- **Win:** CDP `Page.lifecycleEvent` + `Runtime.evaluate` snapshot the form
  AFTER hydration completes; field detection runs against the post-hydration
  DOM. CDP also watches for navigation/mutation and re-detects.
- **Loss:** field gets filled then immediately blanked by Workday's
  re-render. User assumes the binary is broken.

### P6 — The Recruiter-Side Auditor ("Hostile reviewer")
- Reviews applications and is suspicious of AI-filled forms.
- **Implication:** atsisbroken must NEVER fabricate. It autofills only data
  the user explicitly entered. Free-text generation is opt-in per question
  and attributable to the user's own resume verbatim.

### P7 — The Cross-Platform User
- Runs macOS (Intel and Apple Silicon), Linux, or Windows.
- **Win:** GitHub Releases ships pre-built `diamond-edge` binaries for
  `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-unknown-linux-gnu`,
  `x86_64-pc-windows-msvc`. One download, one chmod, run.
- **Loss:** "no Linux build" or "ARM users on their own". Each missing
  triple kills a slice of the user base.

### P8 — The Browser-Diverse User
- Doesn't run stock Chrome. Wants Brave / Edge / Arc / Chromium / Thorium.
- **Win:** CDP is the universal protocol; `--browser-path` or
  `CHROME_PATH` env var picks the binary. Any Chromium-family browser
  launched with `--remote-debugging-port` works.
- **Loss:** hard-coded path to `/Applications/Google Chrome.app`.

## Adversarial Scenarios (must survive)

| # | Scenario | Mitigation in scaffold? |
|---|----------|-------------------------|
| A1 | User pastes a 40-page resume | `Profile::raw_resume_text: String` — unbounded. ✅ |
| A2 | Form has only `placeholder="email address"` | FieldDescriptor carries placeholder + aria + label + name + id. ✅ |
| A3 | Form has only `aria-label`, no `<label>` | Same — aria_label is a first-class field. ✅ |
| A4 | Field uses `name="user_phone_mobile"` | Inference receives `name` as part of the descriptor. ✅ |
| A5 | Adversarial label: `<label>Email of your manager</label>` | Pure-keyword classifier would mis-fire; the trained model is the resolver. ⚠️ depends on model quality |
| A6 | Field requests "salary expectation" — not in Profile schema | Spec: model emits `unknown` → binary skips field. Never invent. ⚠️ enforced once `kova-engine::Engine::from_safetensors` is wired |
| A7 | Network call attempted by atsisbroken | Crate dependency tree reviewed in `PROOF_OF_ARTIFACTS.md`. No HTTP client outside `chromiumoxide`'s local CDP socket. ✅ structurally |
| A8 | Binary tries to download a model from CDN | Model is `include_bytes!`-baked at compile time. Cannot be fetched at runtime. ✅ structurally guaranteed |
| A9 | User reinstalls and loses profile | Profile is a TOML file the user owns. Survives reinstall trivially. ✅ |
| A10 | Browser crashes mid-fill | CDP session detect + restart. ⚠️ not yet wired |
| A11 | macOS Gatekeeper blocks the binary | GitHub Release ships notarized binaries. ⚠️ release pipeline TBD |

## What the current scaffold actually delivers

- ✅ Single-package layout. `cargo build` produces one binary.
- ✅ Custom `.safetensors` slot reserved at the workspace root, baked via
  `include_bytes!`. Currently a clearly-labeled placeholder.
- ✅ Four bake variants gated by feature: `bake-tiny` (default),
  `bake-cinder`, `bake-cinder-f16`, `bake-quench`. Compile error if none
  selected.
- ✅ Determinism contract: `baked_model_fingerprint()` (FNV-1a) returns the
  same value across runs and machines. Tested.
- ✅ Profile / Experience / Education / FieldDescriptor schemas with
  serde round-trip tests.
- ✅ Diamond profiles wired (speed-Diamond + size-Diamond).
- ❌ No CDP loop yet — `src/main.rs` is a stub.
- ❌ No real model — placeholder bytes.
- ❌ No release pipeline.

## P0 next moves (ordered by user-impact-per-LOC)

1. CDP loop in `src/main.rs`: launch Chromium, attach to a tab, snapshot
   the DOM, extract `FieldDescriptor`s, emit JSON for the inference call.
2. Wire `kova_engine::inference::Engine::from_safetensors(FIELD_MODEL)`
   and run it against the snapshotted descriptors.
3. Profile loader: `clap` flag → TOML → `Profile`. Round-trip with the
   existing serde tests.
4. Train the real `atsisbroken-tiny.safetensors` in `pixel-forge`.
5. Cross-compile pipeline: GitHub Actions builds the four target triples
   under `--profile=diamond-edge`, attaches to a Release.
