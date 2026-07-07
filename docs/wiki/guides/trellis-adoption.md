---
title: Trellis Adoption in tenex-edge
slug: trellis-adoption
topic: trellis-adoption
summary: Trellis is a deterministic reconciliation engine â state in, effect plans and receipts out â explicitly designed to be legible to tooling and AI agents that
tags:
  - capture
volatility: warm
confidence: medium
created: 2026-07-03
updated: 2026-07-04
verified: 2026-07-03
compiled-from: conversation
sources:
  - session:065295ad-311d-4965-a3c1-6f749135f2b8
  - session:c7805f5d-42c5-44b6-8eaa-ecd2453ed822
---

# Trellis Adoption in tenex-edge

## What Trellis Is

Trellis is a deterministic reconciliation engine — state in, effect plans and receipts out — explicitly designed to be legible to tooling and AI agents that need to reason about why a system did what it did without parsing logs. tenex-edge adopts Trellis as a deterministic reconciliation engine for subscriptions, status, and hook context.

The architectural boundary for adoption is that Trellis owns decisions while the host owns observations and effects. Canonical inputs are facts the world hands the system — `ProcessExited`, `DistillCompleted`, `RelayPublishAccepted`, `ClockTick`, and similar world-fact events — and Trellis derives sessions, status, who, context, outbox, and subscriptions. The world's facts enter as a single input journal and Trellis derives sessions/status/who/context/outbox/subscriptions.

Partial adoption of Trellis is strictly worse than the status quo because two systems then disagree about the same rows and each believes it is authoritative. The hazard lives only at promotion — not during shadow mode, where nothing is applied. Each adoption slice must convert every writer of a surface in the same change at promotion time — full cutover with no split-brain.

ADR-0075 is the NMP architecture decision record establishing that NMP may use Trellis as its private reconciliation substrate, with a hard boundary forbidding Trellis types from any app/native/web-facing API, enforced by doctrine-lint and a leak ratchet.

NMP already applies Trellis ResourcePlan as the `ApplyDependentInterestDelta` kernel operation to feed sessions and needs to retain and export bounded diagnostic records from this path rather than relying on test-only traces.

The `InputFact` journal enum for Trellis adoption in tenex-edge comprises `SessionStarted`, `TurnStarted`, `TranscriptWindowCaptured` (carrying a `window_hash`, not text), `DistillCompleted`, `TurnEnded`, `RelayEventObserved`, `RelayPublishAccepted`, `ProcessExited`, and `ClockTick` variants, each replacing a specific existing writer.

<!-- citations: [^06529-b7e0d] [^06529-8aa0a] [^06529-8b746] [^c7805-627a1] [^c7805-cb352] [^c7805-11848] [^06529-0adb7] [^06529-67b5c] [^06529-c5ba6] -->
## tenex-edge as a Trellis Host

tenex-edge is a Rust/Nostr daemon that derives live resources — relay subscriptions, kind:30315 status events, presence, hook context — from changing state. `build_entity_coverage(state)` is the pure function that recomputes `EntityCoverage` (channels, addressed pubkeys, group state) from subscribed_projects, identities, channel membership, and per-alive-session joined channels.

tenex-edge currently has a hand-rolled partial receipt system: `turn_start_audit()`/`turn_check_audit()` structured JSON and a `HookCallLog` JSONL forensic log. This audit is an approximation that diverges from the actual render because `awareness_json()` considers a different channel set than `build_view`. Additionally, the PostToolUse hook uses a compare-and-swap race on `seen_cursor` across concurrent hook processes, so two parallel tool calls in the same turn can nondeterministically produce different injected context depending on which hook process wins the race. The PostToolUse CAS race is not fully solved by Trellis alone; it requires cursor advancement to move into the daemon's single graph with hooks becoming read-only frame consumers — a decision Trellis motivates but does not make. These defects are the motivation for replacing the hand-rolled audit with Trellis-derived hook snapshots.

Effects in tenex-edge are imperatively interleaved with state mutation and hand-ordered, most acutely in `rpc_session_start` (370 lines alternating DB writes, relay round-trips, task cancels/spawns, tmux calls, and tail emits with load-bearing ordering and inline rollback) and `handle_incoming` demux. This interleaving is the core structural problem Trellis adoption addresses.

In the Trellis adapter pattern for tenex-edge, a private `Graph` and `InputNode` handles are kept inside a host-shaped wrapper struct that exposes typed methods staging transactions and returning typed effects/frames, never exposing Trellis types externally — mirroring how `protocol_subscription`'s `engine.rs` hides Trellis behind `ArticleFeedApp`. <!-- [^06529-52de2] -->

<!-- citations: [^06529-568b2] [^06529-08611] [^06529-78f81] -->
## Audit API

Trellis's audit API provides `why_resource_command(key)` to explain why a specific effect was emitted, `why_changed(node)` to explain why a derived value recomputed, `why_output_frame(key)` to explain why a materialized output emitted, and `dependency_path(from, to)` for the shortest causal chain from an input to an effect.

`graph.why_resource_command` returns a `ResourceCommandExplanation` with fields for key, scope, kind, cause, collection_diffs, changed_nodes, input_causes, and dependency_paths — the latter two only populated when the transaction opts into `AuditExplanationLevel::DependencyPaths`.

Audit explanations are latest-per-key and ephemeral: `record_transaction_audit` overwrites per key every commit and clears them when audit is `Disabled`, so `why_changed`/`why_resource_command` answer only the most recent change per key.

`redact_trace` must cover the audit_log and any future key-carrying fields so that a shareable capsule does not leak un-redacted ResourceKeys. The current implementation redacts resource_commands, resource_coalescences, and invariant names but leaves trace.audit_log untouched, and `AuditEvent::ResourceOpenCoalesced` carries a full ResourceKey. A test must serialize a redacted capsule and assert no original key material survives anywhere in the JSON. This leak is tracked as Trellis issue #161, which is a hard gate for Phase E; a fix draft (trellis#162) is in progress.

Free-form reason strings in receipts are replaced with structured reason codes (an NMP enum plus typed params, rendered to text at the edge). A snapshot/regex test asserts that rendered receipts contain no `trellis`/`ResourcePlan`/`NodeId(` pattern substrings. Interest shape is explicitly flagged as the primary privacy-bearing field in the receipt — a Nostr filter containing pubkeys and event ids — that Trellis redaction never sees.

<!-- citations: [^06529-7f5c8] [^c7805-42937] [^c7805-822a4] [^c7805-79148] [^06529-1c4ba] [^06529-4646f] -->
## Reference Example

The `crates/trellis-examples/src/protocol_subscription.rs` example is a structural twin of the tenex-edge daemon pattern: typed session handles, subscription effects, and feed frames with Trellis as a private reconciliation engine. <!-- [^06529-42267] -->

## Adoption Epic

The Trellis adoption epic is tracked as GitHub issue #202 in `pablof7z/tenex-edge`, titled 'Adopt Trellis for deterministic reconciliation of subscriptions, status & hook context.' It has eight child slice issues: #203 through #210, sequenced shadow-mode lowest-friction-first.

**Slice #203** audits and maps tenex-edge surfaces to Trellis primitives with no behavior change.

**Slice #204** shadow-modes the subscription planner, diffing Trellis's plan against the existing `ensure_subscription` on live traffic with the bespoke path authoritative.

**Slice #205** resurrects subscription teardown, fixing the documented tens-of-GB leak by killing the dead `close_subs`/`compact` code paths. It retires the bespoke incremental subscription path entirely when the close plan turns on.

**Slice #206** models 30315 status as a derived node and frame, collapsing five uncoordinated triggers and two independent timers to emit only on change. All five status triggers and both timers are collapsed at once, not incrementally.

**Slice #207** adds deterministic status teardown so dead sessions and stale `h`-tags are retracted instead of aging off via TTL.

**Slice #208** replaces the divergent hand-rolled `turn_start_audit` with the hook snapshot as a derived Trellis node. The hook render reads only Trellis projections.

**Slice #209** adds an equivalence oracle and characterization harness with `assert_incremental_equals_full` as a CI gate.

**Slice #210** adds the `tenex-edge explain <handle>` retrospective instrumentation verb.

The subscription cutover moves tenex-edge from an aggregate-REQ model to a refcounted per-entity model under Trellis, because the aggregate-REQ model fundamentally cannot do clean teardown.

<!-- citations: [^06529-06a72] [^06529-b3034] [^06529-1a3c3] -->
## Recommended First Step

The recommended first adoption step is shadow-moding the subscription planner. Model `subscribed_projects`, `identities`, and `session_channels` as inputs; add a resource planner keyed by existing deterministic subscription IDs; feed it the same input writes; and compare its plan to what `ensure_subscription` actually did on live traffic — with the bespoke path authoritative. The pure `state → Vec<PlannedReq>` function already exists, structured on the `protocol_subscription.rs` example shape.

<!-- citations: [^06529-198a3] [^06529-823b6] -->
## Modeling Constraints

The LLM distill step must remain a non-deterministic input to the Trellis graph, never a derived node. Wall-clock `now` (time-relative strings, 90s TTL, NIP-40 expirations) must be modeled as an explicit input to the Trellis graph for replay correctness — a pervasive mechanical change. Bulk data (transcripts, raw event JSON, chat bodies) must be kept out of the Trellis graph; the graph stores hashes or summaries, not full payloads — only pointers and hashes are stored in the graph.

<!-- citations: [^06529-9fbfb] [^06529-a6e48] -->
## Deferred Surfaces

The `rpc_session_start` function in tenex-edge is 370 lines of interleaved DB writes, relay round-trips, spawns, and tmux calls with hand-managed ordering and inline rollback, making it the hardest surface to extract into a Trellis plan/commit/effect boundary. It should be done last or never.

<!-- citations: [^06529-5af77] [^06529-7940f] -->
## Trellis Flight Recorder

The Trellis Flight Recorder is a trace-replay workstation providing provenance, transaction timeline, phase ladder, receipt inspector, and replay/raw-trace tabs, functioning as a time-travel/trace-replay UI shipping under Trellis epic #87. <!-- [^c7805-439ec] -->

## Public-API Leak Ratchet

The Trellis public-API leak ratchet uses a dependency-graph allowlist — a file listing crates permitted to depend on trellis-* (existing private adapters plus nmp-devtools), where adding a new line is the ratchet violation. <!-- [^c7805-0ea26] -->

## Trace Versioning

Trellis's trace versioning (`validate_format_version`) is exact-match-only, hard-failing on anything but equality with no migration path. Because `SerializedScenario` carries `TRACE_FORMAT_VERSION`, every format bump orphans all outstanding capsules.

<!-- citations: [^c7805-fc889] [^c7805-b8076] -->
## Trace Serialization

Trellis issue #114 (closed) delivered serializable traces via PR #141, including serde-gated `DataTransactionScript`, `SerializedScenario`, versioned format-mismatch errors, and a golden v1 trace fixture. This enables cross-process replay and golden-trace CI gating. The positioning doc's earlier claim that traces don't serialize yet is stale.

Trellis `TransactionTrace` is a serde-capable projection with ordered resource commands, coalescences, output frames, scope events, audit log, and phase trace; it records what happened but cannot be used to re-execute transactions. The serialized trace carries structural causality (dirty roots, recomputed nodes, changed nodes, resource commands, output frames, audit log) but NOT dependency-path explanations (input_causes/dependency_paths), which live only in the ephemeral audit index.

`SerializedScenario` carries `TRACE_FORMAT_VERSION` (currently 1), and `validate_format_version` hard-fails on anything but exact equality (a typed error, not migration), so every format bump orphans all outstanding capsules.

<!-- citations: [^c7805-cd5b0] [^c7805-d0a89] [^06529-5d24f] -->
## Adoption Execution Mandate

The user directs that the full Trellis adoption in tenex-edge be landed with real code — full cutover with no split-brain — and then validated with real containers via the tenex-edge-dev skill.

Progress updates are posted to the `#trellis-tenex-edge` channel (created per user direction), and milestones are announced via `say` (TTS) at least 10 to 20 times throughout execution.

Adoption work is done in an isolated git worktree at `~/src/tenex-edge-trellis` on branch `trellis-adoption` off `origin/master`, to avoid colliding with other agents actively working in the main repo tree.

The tenex-edge repo enforces a 500-LOC-per-file ceiling, so new Trellis code must go in new modules rather than expanding existing files. <!-- [^06529-e5350] -->


Trellis's `assert_incremental_equals_full()` oracle is called after every meaningful transaction in tests to verify that incremental recomputation matches a full recompute. <!-- [^06529-477bf] -->
## Retrospective Explain Verb

The `tenex-edge explain <handle>` retrospective instrumentation verb (issue #210) operates over artifact handles — `event:`, `hook:`, `llm:`, `txn:`, `session:@ts` — backed by a persisted `txn_receipts` table and an `llm_calls` ledger. The `llm_calls` ledger captures the exact transcript slice, system prompt, model, and raw LLM response behind every distilled 'current activity.' The verb also provides `replay` via trace-matching, with a redaction policy for user content in prompts.

The `llm_calls` table stores verbatim system prompts, transcript slices, and raw responses with no retention policy, requiring bounded retention and end-to-end `--redact` capability for shareable exports.

The persisted receipt store and `llm_calls` ledger for Slice 8's explain CLI are developed as new-files-only additions in a dedicated tenex-edge worktree so they cannot collide with the subscription cutover work on the main line.

<!-- citations: [^06529-d9c9e] [^06529-7525d] [^06529-40932] -->
## Reconciler Spine — Adapter Decision

The `trellis-adapter` crate is skipped for the reconciler spine because the `protocol_subscription` example applies plans by hand matching on `ResourceCommand` from `result.resource_plan.commands()`, so the adapter is not needed.

The hook_context reconciler is currently advisory (not authoritative) because it is rebuilt fresh per render and discarded, leaving no live graph to inspect; promoting it requires holding a long-lived per-session `HookContextReconciler` in `DaemonState`. The hook-context reconciler replaces the drifting hand-rolled `turn_start_audit` with a derived `FabricView` node over six declared inputs feeding a materialized output frame, proven byte-for-byte identical to the old renderer.

<!-- citations: [^06529-ebcc1] [^06529-6827f] -->
## Authority Frontier — North Star

The North Star for the authority-frontier program is: make Trellis the first authority for tenex-edge state transitions such that any transition can be inspected, explained, reproduced, and simulated from its receipt alone. <!-- [^06529-e7806] -->

## Receipts and Commits Ledger

The `receipts` table stores per-transaction audit records keyed by surface — `subscriptions`, `status`, or `hook_context` — with `transaction_id`, `revision`, `changed_summary` (JSON), `commands` (JSON array), and `artifact_ref`. Receipts persist only for commits that produced effects: status skips when no event is published and subscriptions skip when no open/close effect occurs, meaning no-op and suppressed publishes leave no trace.

The Trellis all-commit ledger (`trellis_commits` table) records every transaction — including no-ops and suppressed publishes — separate from the artifact-keyed receipts table, ensuring a complete audit trail even when the receipts table has no entry.

A stable node-label registry persists both graph-local `NodeId` and a semantic label (e.g. `status/<session>/activity`) in receipts and the commits ledger, replacing opaque integer-only identifiers. <!-- [^06529-f6a33] -->

## Transaction::preview() — Dry-Run Simulation

`Transaction::preview()` runs the full commit pipeline on a working-copy clone and returns the `TransactionResult` without writing back to the live graph, providing a dry-run simulation that lets the host inspect what a transaction would produce before committing. This is merged into pablof7z/trellis master (squash 60e759d, PR #164) with three tests: `preview_does_not_mutate_graph`, `preview_matches_commit`, and `preview_can_be_followed_by_a_real_commit`. <!-- [^06529-614ee] -->

## Oracle Limits and Shadow-State Hazards

Trellis's `assert_incremental_equals_full()` oracle is called after every meaningful transaction in tests to verify that incremental recomputation matches a full recompute. However, oracle-green proves only the graph's internal incremental bookkeeping — it does NOT prove the host supplied the right facts, applied the right effects, or advanced the right cursor.

The host-side shadow state inside the reconcilers (`StatusReconciler.last`, `HookContextReconciler.last_view`, session node-handle maps) lives outside the graph, is invisible to the oracle, and is where split-brain can re-enter the system even after cutover. <!-- [^06529-ee5c4] -->
