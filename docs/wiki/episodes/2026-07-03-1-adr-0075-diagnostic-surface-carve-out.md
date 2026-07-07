---
type: episode-card
date: 2026-07-03
session: c7805f5d-42c5-44b6-8eaa-ecd2453ed822
transcript: /Users/pablofernandez/.claude/projects/-Users-pablofernandez-src-trellis/c7805f5d-42c5-44b6-8eaa-ecd2453ed822.jsonl
salience: architecture
status: active
subjects:
  - adr-0075-diagnostic-surface
  - nmp-devtools
  - trellis-type-boundary
  - chirp-renders-not-parses
supersedes: []
related_claims: []
source_lines:
  - 3-5
  - 100-102
  - 123-173
  - 175-175
  - 209-217
captured_at: 2026-07-03T07:43:37Z
---

# Episode: ADR-0075 diagnostic surface carve-out — Trellis types allowed in dev-only nmp-devtools, still forbidden at app surface

## Prior State

ADR-0075 forbids Trellis types from reaching any app/native/web surface with no carve-out for diagnostics or tooling. Doctrine-lint and the Trellis public-API leak ratchet (#2642) enforce this single boundary. NMP applies Trellis ResourcePlan internally but only emits test-only traces — no bounded diagnostic records are retained or exported.

## Trigger

NMP #2809 proposes amending ADR-0075 to split 'app surface' from 'diagnostic surface.' Background survey confirmed all four trellis prerequisites (#114 serializable traces, #122 typed keys, #125 bounded audit, #126 scope reclamation) are closed, and NMP #2808 (shadow→authority promotion) merged today via PR #2832 — leaving only the ADR decision itself as the blocker.

## Decision

Amend ADR-0075 to define two surfaces: (1) app surface — unchanged, no Trellis vocabulary crosses UniFFI/web/app APIs; (2) diagnostic surface — a dev-build-only nmp-devtools channel exposing ordered open/replace/refresh/close events with owner, provenance, revision, refcount, and teardown-cascade data. NMP owns the receipt shape so Chirp (and any consumer) renders diagnostics but never sees ResourcePlan, ResourceKey, or Graph. Doctrine-lint and leak ratchet must be updated to enforce the diagnostic-surface exemption mechanically, not case-by-case.

## Consequences

- Nothing blocks NMP #2809 except drafting the ADR amendment text — it is the decision gate the rest of the epic queues behind
- Chirp's X-Ray pane must consume an NMP-defined receipt contract, not parse Trellis types directly
- NMP must convert test-only Trellis traces into bounded retained/exportable diagnostic records (retention + redaction policy needed)
- Existing relay diagnostics (wire subscriptions, relay rows, consumer counts) must be correlated with the reconciliation command stream in the nmp-devtools projection
- Leak ratchet (#2642) needs a corresponding update so the diagnostic-surface exemption is CI-enforced

## Open Tail

- ADR-0075 amendment text not yet drafted — it is the first and gating deliverable
- NMP-owned receipt shape is not yet specified concretely

## Evidence

- transcript lines 3-5
- transcript lines 100-102
- transcript lines 123-173
- transcript lines 175-175
- transcript lines 209-217

## Conversation

- Cleaned transcript (verbatim user words, abbreviated agent replies): [`transcripts/2026-07-03-1-adr-0075-diagnostic-surface-carve-out.json`](transcripts/2026-07-03-1-adr-0075-diagnostic-surface-carve-out.json)
- Raw transcript (verbatim user words, full agent replies): [`transcripts/raw/2026-07-03-1-adr-0075-diagnostic-surface-carve-out.json`](transcripts/raw/2026-07-03-1-adr-0075-diagnostic-surface-carve-out.json)
