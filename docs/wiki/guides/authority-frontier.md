---
title: Authority Frontier Program
slug: authority-frontier
topic: trellis-adoption
summary: "The authority frontier classifies every surface as one of five modes: **imperative â shadow â advisory â authoritative â projection-owned**"
tags:
  - capture
volatility: warm
confidence: medium
created: 2026-07-04
updated: 2026-07-04
verified: 2026-07-04
compiled-from: conversation
sources:
  - session:065295ad-311d-4965-a3c1-6f749135f2b8
---

# Authority Frontier Program

## Authority Frontier

The authority frontier classifies every surface as one of five modes: **imperative → shadow → advisory → authoritative → projection-owned**. The classification is registered in a code-owned `SurfaceRegistration` struct and ratcheted in CI, so the frontier's position is an explicit, machine-checked property of the codebase rather than a convention.

Trellis leads a surface when every state-changing operation first becomes a canonical fact or command intent, Trellis derives the desired projection/effect intent, and the host may only execute that returned intent.

The one-output-payload-type-per-graph constraint (#121) combined with the charter's keep-bulk-payloads-out rule means Trellis can lead only small, single-typed DB projections — not bulk table state. Projection-owned is therefore a bounded summit, not the default destination for every surface.

The journal-as-first-writer pattern is applied only on high-value promotion-target seams — turn lifecycle, cursor, and the outbox loop — not on every incidental DB write. Forcing every store mutation through a fact is charter-violating ceremony with no leadership payoff.

<!-- citations: [^06529-701fd] [^06529-8c308] [^06529-4e045] [^06529-8bb1a] [^06529-f6623] -->
## Safety Invariant

On any surface at authoritative or above, no host effect executes without a Trellis plan or frame having been produced first. This invariant is enforced structurally by the CI bypass ratchet, not by convention.

<!-- citations: [^06529-05b95] [^06529-0e232] -->
## CI Bypass Ratchet

A CI bypass ratchet enforces that no new direct-effect path bypasses a declared reconciler seam. It uses grep/AST checks against an allowlist whose entries carry surface, mode, and reason fields. Tests such as `no_direct_status_publish_outside_status_seam` and `no_direct_subscribe_unsubscribe_outside_subscription_executor` fail the build when a new bypass appears. The ratchet ensures the frontier only advances forward — regressions are caught, never silently merged.

<!-- citations: [^06529-f211d] [^06529-4b252] -->
## Probe Seams

Probe seams renders a coverage map of what Trellis owns versus what the host still does imperatively. The map explicitly includes host-side shadow state that lives outside the graph and is invisible to the oracle: `StatusReconciler.last`, `HookContextReconciler.last_view`, and session maps. Making these hidden state stores visible is the first step toward deciding whether each should be pulled into the graph or left as a bounded imperative holdout. <!-- [^06529-853d9] -->

## Promotion Gates

Promotion of a surface's authority mode is gated by all of the following being green: oracle green, replay asserts pass, ACID confirmed on seeded cases, bypass ratchet green, and host-seam-coverage at 100% for that surface. No surface advances until every gate is satisfied. <!-- [^06529-596a6] -->

## Diagnosis Metric

The diagnosis metric target is: an off-the-shelf model, given only the serialized commit/trace stream around a seeded lifecycle bug, identifies root cause with ≥90% accuracy versus its baseline from conventional logs. This is the falsifiable proof that the authority frontier's observability payoff is real. <!-- [^06529-ed000] -->

## Surface-by-Surface Decisions

The session-row projection (`working` / `turn_started_at`) is the one place to exercise projection-owned mode. It is small, single-typed, and drift is a real bug class — exactly the profile where leading from the graph pays off. <!-- [^06529-8c15b] -->

Session_start is deliberately held at shadow/advisory, not promoted to full authoritative. Its interleaved DB/relay/signer/spawn/tmux effects have high blast radius, low reproduce-value, and are charter-hostile — the cost of leading it from the graph exceeds the diagnostic and safety payoff. <!-- [^06529-a9970] -->

## Upstream Asks to Trellis

Seven upstream asks are submitted to Trellis: (1) `tx.preview()`, (2) serialize audit explanations into the trace, (3) audit retention / historical why, (4) first-class node-label ergonomics, (5) projection-frame pattern plus #121 documentation, (6) confirm #114 status in docs, and (7) a host-conformance ledger pattern for trellis-testing. These are the capabilities needed to advance the frontier that the host cannot build alone. <!-- [^06529-1e1bc] -->

## Program Tracking

Epic #228 in pablof7z/tenex-edge is the authority-frontier program. It contains the North Star, two falsifiable targets, the frontier classification table, the probe conformance surface, the gated roadmap, and seven upstream asks, building on #202. The epic is decomposed into 12 slices (#229–#240) spanning four phases: Phase 0 (labels + all-commit ledger, probe stats + oracle, `frontier.rs` + seams + CI ratchet), Phase 1 (capsules, tx.preview→probe simulate, promote `hook_context`), Phase 2 (diff + ACID + seeded-bug corpus), and Phase 3 (turn_lifecycle → cursor → outbox → session_start capped at shadow), followed by the case-study writeup. <!-- [^06529-2e088] -->

## Case Study

The case-study artifact is titled 'tenex-edge: moving the authority frontier under a live agent fabric.' Its spine is the probe seams table time-lapse — Phase 0 baseline through each promotion — backed by a diagnosis-metric experiment, a safety-invariant demonstration, and the leak-already-fixed beachhead. <!-- [^06529-9ba36] -->
