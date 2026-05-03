<!-- Unlicense — cochranblock.org -->
<!-- Contributors: GotEmCoach, KOVA, Claude Opus 4.7 -->

# Why no one is building this

**Date:** 2026-05-03
**Scope:** Honest answer to "why doesn't this exist already?" The
demand is obvious. The technical pieces are mature. Yet the
specific combination — free, local, no-account, verbatim-source,
runs the user's own model — isn't on anyone's laptop.

---

## The puzzle

Every cs-careers Reddit thread is half complaining about ATS forms.
Simplify.us has 2M+ users on a paid product that hallucinates and
gates features. The technical primitives — Chromium DevTools
Protocol, on-device inference via candle, MV3 extensions, Native
Messaging — have been mature for 2+ years.

So why isn't there an open-source local-first verbatim-source ATS
autofiller already?

Six honest reasons.

---

## 1. The business model is inverted

A SaaS company can't ship this. Their unit economics depend on
freemium → paid conversion (~3% on Simplify-style products). A
free, local, no-account version cannibalizes that funnel without
replacing the revenue.

If a SaaS competitor copied this design, their LTV craters. So
they don't. Their incentives keep them on the cloud-LLM path even
though they know it's worse for users.

A free product can only come from someone who isn't optimizing for
recurring revenue: an indie maintainer, a research lab, or a
self-funded shop. That's a small pool of would-be builders.

## 2. The engineering cost is 10× a SaaS

Doing this right means:
- Cross-compile for 4 desktop triples + 3 mobile ABIs.
- Ship a model that trains on the user's machine in seconds on CPU.
- Detect the user's default browser per OS (no clean Rust crate).
- Optionally install Chromium when missing.
- Native Messaging bridge between extension and binary.
- Multi-tier strategy ladder (CDP → launch → extension → userscript
  → bookmarklet → clipboard → speak).
- TUI for terminal users.
- Audit log + verbatim-source composer.

Compare to the cloud-LLM path: MV3 extension + REST calls + OpenAI
key. A week to MVP, a month to ship.

The cost asymmetry isn't 2×, it's 10×. That's why every commercial
competitor picked the easier path. Not because they didn't think of
local — because local doesn't justify the eng spend on a SaaS
balance sheet.

## 3. The "AI cover letter" wave drowned out the right approach

From 2023 through 2025, every job-search startup pivoted to "GPT
writes your cover letter." That's the *opposite* of what hiring
managers want — hallucinated prose, voice mismatch, generic
openers — but it was easy to demo and easy to monetize per-token.

The actually-correct answer (don't generate, just *quote* the user's
own commits and README sentences) was unfashionable. Founders
chasing the hype got funded; the few who were thinking about
honesty-by-construction didn't pitch it because it sounded boring.

The hype is now reversing — recruiters explicitly flag AI-generated
applications, "AI cover letter detectors" are a product category —
but the field is still oriented around generation rather than
quotation. Someone has to build the alternative.

## 4. The supporting infrastructure most builders need to assemble first

To ship this product alone, a builder needs:
- A local inference engine.
- A model-training pipeline that produces small `.safetensors`.
- A determinism gate for the inevitable non-determinism in inference.
- A multi-platform packaging story.
- Cross-compile CI for at least 4 targets.

Each of those is a multi-month project on its own. A solo dev or
small team would spend their first year building scaffolding before
they could start on the ATS-specific layer.

The only people for whom this is *not* a year of scaffolding are
people who already have that infrastructure for unrelated reasons —
which is rare.

## 5. The technical timing is genuinely recent

On-device tiny classifiers became practical only since ~2024:
- candle (Rust ML framework, 2023)
- burn (Rust ML framework, 2024)
- safetensors as a portable format
- WASM with relaxed-SIMD support
- Reasonably small classifier architectures

Before 2024, "ML on the user's laptop with no cloud GPU" mostly
meant TensorFlow.js (heavy and slow) or "send to OpenAI." The
ergonomic Rust path is genuinely new.

So this is a "now is the time" idea, not a "why didn't they build
it ten years ago" idea. The window opened maybe two years ago. The
hype-cycle distraction in §3 ate the first 18 months. We're early,
not late.

## 6. Privacy-hawk users are loud but small

The single most reliable user for a product like this is the
security/privacy-hawk engineer (P2 in `USER_STORY_ANALYSIS.md`).
They'll audit the source, tweet about it, run `tcpdump` to verify
the no-network claim. They're high-quality users.

They're also a small market. SaaS companies skip them because
they're financially unattractive: they refuse subscriptions, read
every TOS, distrust analytics. Bigger volumes of less-scrupulous
users pay the bills.

A free product doesn't have to choose. P2 is a happy side effect,
not a target segment to monetize.

---

## What it would take

Adding up §1–§6: someone has to be willing to do work that:
- Doesn't pay (§1).
- Costs 10× a normal product (§2).
- Goes against industry hype (§3).
- Requires ML/cross-compile infrastructure most teams don't have (§4).
- Couldn't have been done two years ago (§5).
- Serves a vocal-but-small ideological core (§6).

That intersection is small but not empty. Indie open-source
maintainers, research groups, side-project shops with conviction.

It's surprising no one has shipped this yet. It probably reflects
how thin that intersection actually is — and how recent the
technical timing in §5 became practical.

The window is open now. It might not stay open: Chrome could ship
better built-in autofill; Apple might add a system-level form-filler
on macOS; a well-funded competitor could see the gap and copy the
design. But for now, the slot is empty and the right answer is
known.

So someone should build it. Whether or not it's us.
