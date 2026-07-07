---
type: episode-card
date: 2026-07-03
session: c7805f5d-42c5-44b6-8eaa-ecd2453ed822
transcript: /Users/pablofernandez/.claude/projects/-Users-pablofernandez-src-trellis/c7805f5d-42c5-44b6-8eaa-ecd2453ed822.jsonl
salience: product
status: active
subjects:
  - chirp-30-v2-time-travel
  - trellis-flight-recorder
  - trace-replay-capsule
  - nmp-2809
supersedes: []
related_claims: []
source_lines:
  - 16-19
  - 123-173
  - 167-168
  - 207-222
captured_at: 2026-07-03T07:43:37Z
---

# Episode: Time travel promoted from v2 stretch goal to committed Phase E scope

## Prior State

Time travel (record + scrub a session trace) was listed as v2 in chirp #30, gated on trellis #114 and NMP #2809, and treated as aspirational. The user explicitly asked whether to aim for it.

## Trigger

User directive to include time travel in the plan. Survey finding that trellis #114 (serializable scripts/traces) is closed via PR #141, and Trellis Flight Recorder workstation (#160, merged today) already ships a full trace-replay UI with provenance, transaction timeline, receipt inspector, and replay tabs.

## Decision

Time travel is committed Phase E scope in epic #2858, not a stretch goal. Constraints: (1) replay-from-capsule-alone so a redacted trace attached to an issue is debuggable without the device or live relays; (2) redaction policy with local/debug vs shareable modes; (3) ring-buffer retention with explicit export; (4) correlation persisted into the capsule so scrubbing reconstructs full cause chains (refcount 2→1 vs 1→0→CLOSE); (5) reuse Trellis Flight Recorder for desktop-side inspection rather than building a second scrubber.

## Consequences

- NMP still needs retention/redaction/export policy and tooling around the trace export side of #2809 — this is the remaining NMP work for Phase E
- Trellis Flight Recorder (#160) becomes the reference implementation and reuse target for the time-travel UI, connecting the NMP capsule format to an existing trace workstation
- Capsule format must persist correlation data, not just event logs, to support causal-chain scrubbing
- Agent-first prover (Phase C) lands before the pane (Phase D) and before time travel (Phase E), so agents can iterate on diagnostics from receipts alone before any UI exists

## Open Tail

- NMP capsule format not yet defined; must align with Trellis Flight Recorder's expected input
- Redaction policy details (what is safe to export vs local-only) not yet specified

## Evidence

- transcript lines 16-19
- transcript lines 123-173
- transcript lines 167-168
- transcript lines 207-222

## Conversation

- Cleaned transcript (verbatim user words, abbreviated agent replies): [`transcripts/2026-07-03-3-time-travel-promoted-from-v2-stretch.json`](transcripts/2026-07-03-3-time-travel-promoted-from-v2-stretch.json)
- Raw transcript (verbatim user words, full agent replies): [`transcripts/raw/2026-07-03-3-time-travel-promoted-from-v2-stretch.json`](transcripts/raw/2026-07-03-3-time-travel-promoted-from-v2-stretch.json)
