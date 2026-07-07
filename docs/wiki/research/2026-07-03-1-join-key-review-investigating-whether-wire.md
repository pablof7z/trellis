---
type: research-record
date: 2026-07-03
session: f940bd78-c4e8-413d-82a8-53aa459f690c
transcript: /Users/pablofernandez/.claude/projects/-Users-pablofernandez-src-trellis/f940bd78-c4e8-413d-82a8-53aa459f690c.jsonl
source_lines: 294-321
agent_attribution: joinkey-review
has_preregistered_criteria: true
has_method: true
has_structured_report: true
characterization: "Join-key review investigating whether wire_id_hint/canonical_filter_hash is the correct join key; verdict: NEEDS FIX due to pre-compile vs post-compile shape divergence from author-partitioning and lattice-merge"
captured_at: 2026-07-03T11:00:29Z
---

Join-key review investigating whether wire_id_hint/canonical_filter_hash is the correct join key; verdict: NEEDS FIX due to pre-compile vs post-compile shape divergence from author-partitioning and lattice-merge

---

<teammate-message teammate_id="joinkey-review" color="blue" summary="Join-key review complete: needs fix">
Review complete.

## Verdict: NEEDS FIX before it's load-bearing. Mergeable only as an explicitly-flagged, best-effort/lossy scaffold — must NOT be trusted as the authoritative join key.

Good news up front: the key *format* is correct by construction, and codex1's `InterestId(0)` worry is a red herring. The real bug is that the hint hashes the *pre-compile* shape while the wire id hashes the *post-compile* shape — and for a real follow feed those never match.

### What codex1 got right (don't change)
- **Format matches on purpose.** Real wire sub id = `wire::sub_id_for` = `format!("sub-{}", shape.canonical_filter_hash)` (crates/nmp-core/src/subs/wire.rs:153-155). `SubShape.canonical_filter_hash` = `canonical_filter_hash(&shape)` (crates/nmp-planner/src/compiler/mod.rs:433). The hint uses the same prefix + same function.
- **`InterestId(0)` is not the problem.** The hint never uses `InterestId`. `InterestId(0)` is only the fallback for the `interest_id` *field* on `WireFrame` (wire.rs:133, subs/handlers.rs:46/123, app.rs:262), never the `sub_id`. codex1 sidestepped `InterestId` *because* feed-session children have no real id — the correct call. This concern resolves IN FAVOR of the current approach.

### The real failure: pre-compile shape ≠ on-wire shape
`diagnostic_interest()` (feed-session/src/trellis_adapter_delta.rs:183-201) hashes `active.child.interest.shape` — the raw child, BEFORE the planner runs. The wire id hashes the compiler's OUTPUT `SubShape`, transformed in between:

1. **Author partitioning (the killer).** Stages 1+2 split a multi-author interest per write-relay: `{authors:{A,B,C},kinds:{1}}` → `{authors:{A,B}}` on relay-1, `{authors:{C}}` on relay-2 (compiler/mod.rs:13,312; partition/case_a_authors.rs:135-312, `entry.authors` is a per-relay subset). Each narrowed shape has a different `canonical_filter_hash`. The hint `sub-<hash({A,B,C})>` matches NEITHER wire sub. Any follow feed whose followed set spans >1 write relay — i.e. essentially all — correlates ~never.
2. **Lattice merge.** Stage 3 unions compatible shapes into one `SubShape` hashed over the merged shape (compiler/mod.rs:403-437; `originating_interests` is a Vec). Any feed-session interest that merges with another → `sub-<hash(merged)>` ≠ `sub-<hash(child)>`.
3. **Coverage-gate `since` bump.** M4 gate mutates shape + calls `recompute_hash()`, changing the wire id post-compile (planner/plan.rs:224-236). (The *watermark* since/until rewrite deliberately does NOT change the hash — subs/handlers.rs:103-105 — so that path is safe.)

### What breaks
On a join miss, relay_correlation.rs:110-120 scores Open/Replace/Refresh as `pending("wire_row_not_seen")` and non-last-owner Close as `unknown("missing_wire_row")`. A live, event-receiving wire sub gets reported as pending/unknown, and the real relay signals (`events_rx`, `eose_zero_events`, `retained`) never attach. Silent UNDER-correlation (false miss, not mis-correlation) — defeats #2868 for exactly the multi-author feed interests it targets.

Two non-issues: cross-*scope* collision is fine (wire id is also scope-free and the wire genuinely dedups those to one REQ). And no producer of `XrayWireSubscriptionSnapshot` exists yet (only struct + fn + tests) — so it's unvalidated, and whenever a producer lands it will source `wire_id` from `wire::sub_id_for` (post-merge hash), precisely what the hint won't equal.

### Recommended fix
- **Proper:** correlate on `SubShape.originating_interests` (real `InterestId`s), not the shape hash. Wire rows can carry `originating_interests`; the blocker is threading the registry-assigned `InterestId` onto the feed-session receipt — exactly the `InterestId(0)` linkage codex1 flagged. Right long-term join, bigger plumbing.
- **Honest interim (if merging now):** keep the hint but (a) relabel it "best-effort shape fingerprint," (b) change the miss outcome from `pending`/`missing_wire_row` to explicit `unknown("uncorrelated")` so it isn't misread as a real relay state, (c) document that partitioned/merged/coverage-mutated interests won't correlate, and gate the producer+correlation behind an "experimental, lossy" flag.

Bottom line for @codex1: ship only as a flagged, documented-lossy scaffold. The scope-drop and `InterestId(0)` angles are fine; the pre-vs-post-compile shape divergence (author partitioning + lattice merge) is what makes the shape-hash join the wrong authoritative key.
