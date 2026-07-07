---
type: episode-card
date: 2026-07-03
session: 065295ad-311d-4965-a3c1-6f749135f2b8
transcript: /Users/pablofernandez/.claude/projects/-Users-pablofernandez-src-trellis/065295ad-311d-4965-a3c1-6f749135f2b8.jsonl
salience: root-cause
status: active
subjects:
  - trellis-fit-assessment
  - subscription-lifecycle
  - status-publishing
  - audit-receipt
  - resource-teardown
supersedes: []
related_claims: []
source_lines:
  - 43-43
  - 132-141
  - 219-219
  - 323-323
  - 429-429
  - 431-491
captured_at: 2026-07-03T07:48:40Z
---

# Episode: Trellis fit assessment: tenex-edge already contains half-wired reconciliation primitives with dead teardown

## Prior State

tenex-edge manages live resources (relay subscriptions, 30315 status events, hook context injection) with effects imperatively interleaved inline with state mutation. A partial hand-rolled audit receipt (turn_start_audit, HookCallLog JSONL) was believed to answer 'why did X happen' questions. Subscription lifecycle was assumed to be handled by a pure state→plan planner in fabric/subscriptions.rs.

## Trigger

Parallel Sonnet agents investigated tenex-edge's codebase across four surfaces: architecture/state model, 30315 status path, hook context injection, and resource lifecycle/leaks. Findings: (1) fabric/subscriptions.rs is already a pure state→Vec<PlannedReq> planner but its close/compact half is dead code (#[allow(dead_code)]) — subscriptions structurally outlive the state that justified them, verbatim the bug class in Trellis's README; (2) status publishing has no dedup before publish and two uncoordinated ~30s timers double-publish; (3) the hand-rolled audit receipt diverges from the actual render (awareness_json considers a different channel set than build_view, cursor filtering differs on full renders); (4) zero refcounting anywhere — build_entity_coverage is the only function that derives shared ownership and it runs solely at startup; (5) the outbox is a working effect-plan+receipt proof-of-concept but scoped to status only.

## Decision

Research conclusion: Trellis is a strong fit for tenex-edge. The codebase already contains the exact primitives Trellis proposes (pure state→plan planner, outbox as durable effect-plan+receipt) but they are half-wired with deliberately dead teardown paths. The specific bug class Trellis targets is present, documented in code comments, and structurally still open on leave/session-end paths. The existing hand-rolled receipt is leaky and confirms the need Trellis would fill.

## Consequences

- Subscription teardown confirmed structurally absent — close_subs and SubscriptionRegistry::compact are dead code; narrow REQs accumulate monotonically for daemon lifetime
- Existing audit receipt (turn_start_audit / HookCallLog) confirmed as approximation that diverges from actual render on non-first-turn snapshots
- Outbox pattern identified as the proof-of-concept to extend to chat, subscriptions, tmux spawns, and membership admission
- Biggest adoption friction identified: extracting effects from mutation paths in rpc_session_start (370-line function with inline DB writes, relay round-trips, task cancels) and handle_incoming demux
- Two uncoordinated heartbeat timers (TENEX_EDGE_HEARTBEAT_MS vs hardcoded HEARTBEAT_SECS) produce redundant/double status publishes with no dedup
- Status outlives dead sessions by up to 90s (NIP-40 TTL) with no active retraction (no kind:5 deletion)
- relay_status DB rows never pruned — unbounded storage growth

## Open Tail

- Fable agent synthesis was planned to produce final conclusions but did not complete within this transcript
- No formal adoption decision was made — session produced research findings only
- Migration path for extracting inline effects from session_start and demux into a plan/commit/effect boundary not yet designed

## Evidence

- transcript lines 43-43
- transcript lines 132-141
- transcript lines 219-219
- transcript lines 323-323
- transcript lines 429-429
- transcript lines 431-491

## Conversation

- Cleaned transcript (verbatim user words, abbreviated agent replies): [`transcripts/2026-07-03-1-trellis-fit-assessment-tenex-edge-already.json`](transcripts/2026-07-03-1-trellis-fit-assessment-tenex-edge-already.json)
- Raw transcript (verbatim user words, full agent replies): [`transcripts/raw/2026-07-03-1-trellis-fit-assessment-tenex-edge-already.json`](transcripts/raw/2026-07-03-1-trellis-fit-assessment-tenex-edge-already.json)
