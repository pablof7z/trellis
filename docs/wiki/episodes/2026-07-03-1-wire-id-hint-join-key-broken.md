---
type: episode-card
date: 2026-07-03
session: f940bd78-c4e8-413d-82a8-53aa459f690c
transcript: /Users/pablofernandez/.claude/projects/-Users-pablofernandez-src-trellis/f940bd78-c4e8-413d-82a8-53aa459f690c.jsonl
salience: root-cause
status: active
subjects:
  - xray-relay-correlation
  - wire-id-hint
  - canonical-filter-hash
  - feed-session-diagnostics
supersedes: []
related_claims: []
source_lines:
  - 294-336
captured_at: 2026-07-03T10:44:43Z
---

# Episode: wire_id_hint join key broken by pre-compile vs post-compile shape divergence

## Prior State

wire_id_hint, derived from canonical_filter_hash of the feed-session child's raw interest shape, was assumed to be a viable short-term join key for correlating feed-session diagnostic receipts with relay/socket wire subscriptions. The open concern (from codex1) was that DependentInterestChild defaults to InterestId(0), making interest-based correlation unsafe.

## Trigger

joinkey-review subagent performed root-cause analysis comparing the hint's hash input against the real wire subscription id (wire::sub_id_for). Found that the hint hashes the pre-compile interest shape while the wire id hashes the post-compile SubShape, and author-partitioning (compiler/partition/case_a_authors.rs), lattice-merge (compiler/mod.rs:403-437), and coverage-gate since-bump (planner/plan.rs:224-236) all mutate the shape in between — so multi-author follow feeds (essentially all real feeds) silently fail to correlate.

## Decision

The InterestId(0) concern was resolved as a red herring (hint never uses InterestId). The durable fix is to correlate on SubShape.originating_interests (real InterestIds) instead of a shape hash. If merging immediately, the current wire_id_hint must be relabeled as 'best-effort shape fingerprint,' miss outcomes changed from pending/missing_wire_row to unknown('uncorrelated'), and the producer+correlation gated behind an experimental-lossy flag. The shape-hash join key is NOT to be trusted as authoritative.

## Consequences

- relay-seam-design agent was cross-notified: its snapshot producer must source wire_id from the real post-compile wire subscription, not the pre-compile shape hash, or it will inherit the mismatch
- Any XrayWireSubscriptionSnapshot producer that lands will emit post-merge wire_ids that the current hint cannot match for multi-author feeds
- relay_correlation.rs:110-120 will report live, event-receiving wire subs as pending('wire_row_not_seen') or unknown('missing_wire_row') — silent under-correlation defeating #2868's purpose for exactly the feeds it targets
- Long-term fix requires threading registry-assigned InterestId onto feed-session receipts — the same InterestId(0) linkage codex1 originally flagged, now confirmed as the real blocker for a correct join
- Current PR can only ship as an explicitly-flagged, documented-lossy scaffold, not as a trusted correlation mechanism

## Open Tail

- scope-review agent's verdict on whether the lossy-scaffold approach is landable in the current PR or must split into a follow-up issue (agent idle, report not yet received)
- relay-seam-design agent's implementation must reconcile with the finding — still in progress
- No producer of XrayWireSubscriptionSnapshot exists yet; only struct + fn + tests, so the mismatch is currently unvalidated at runtime

## Evidence

- transcript lines 294-336

## Conversation

- Cleaned transcript (verbatim user words, abbreviated agent replies): [`transcripts/2026-07-03-1-wire-id-hint-join-key-broken.json`](transcripts/2026-07-03-1-wire-id-hint-join-key-broken.json)
- Raw transcript (verbatim user words, full agent replies): [`transcripts/raw/2026-07-03-1-wire-id-hint-join-key-broken.json`](transcripts/raw/2026-07-03-1-wire-id-hint-join-key-broken.json)
