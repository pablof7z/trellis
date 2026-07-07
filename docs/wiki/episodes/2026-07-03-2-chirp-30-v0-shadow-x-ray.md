---
type: episode-card
date: 2026-07-03
session: c7805f5d-42c5-44b6-8eaa-ecd2453ed822
transcript: /Users/pablofernandez/.claude/projects/-Users-pablofernandez-src-trellis/c7805f5d-42c5-44b6-8eaa-ecd2453ed822.jsonl
salience: reversal
status: active
subjects:
  - chirp-30-v0-shadow-xray
  - nmp-2808-authority-promotion
  - shadow-mode-obsolete
supersedes: []
related_claims: []
source_lines:
  - 123-173
  - 209-212
captured_at: 2026-07-03T07:43:37Z
---

# Episode: Chirp #30 v0 Shadow X-Ray obsoleted — skip straight to v1 authority receipts

## Prior State

Chirp #30 staged a v0 'Shadow X-Ray' phase to visualize NMP's discarded shadow ResourcePlan alongside legacy behavior, as a first step before the real v1 authority-based overlay.

## Trigger

NMP #2808 (shadow→authority promotion of Trellis feed-session reconciliation) merged today via PR #2832. There is no longer a shadow plan to visualize — Trellis reconciliation is now authoritative.

## Decision

Drop v0 'Shadow X-Ray' entirely. Go directly to v1 authority receipts — real applied commands with causal chains, which was already gated on #2808 (now closed).

## Consequences

- Removes a planned implementation phase from chirp #30's roadmap
- Simplifies the path to the X-Ray pane — one fewer phase to build
- v1's only remaining dependency is the ADR-0075 amendment and the nmp-devtools stream (Phase A/B of the epic)

## Open Tail

- chirp #30 issue body may need updating to mark v0 as obsolete/historical

## Evidence

- transcript lines 123-173
- transcript lines 209-212

## Conversation

- Cleaned transcript (verbatim user words, abbreviated agent replies): [`transcripts/2026-07-03-2-chirp-30-v0-shadow-x-ray.json`](transcripts/2026-07-03-2-chirp-30-v0-shadow-x-ray.json)
- Raw transcript (verbatim user words, full agent replies): [`transcripts/raw/2026-07-03-2-chirp-30-v0-shadow-x-ray.json`](transcripts/raw/2026-07-03-2-chirp-30-v0-shadow-x-ray.json)
