---
title: X-Ray Diagnostic Surface
slug: x-ray-diagnostic-surface
topic: x-ray-devtools
summary: The X-Ray diagnostic surface is a dev-build-only nmp-devtools crate that may legally expose Trellis trace/audit receipts to tooling
tags:
  - capture
volatility: warm
confidence: medium
created: 2026-07-03
updated: 2026-07-04
verified: 2026-07-03
compiled-from: conversation
sources:
  - session:c7805f5d-42c5-44b6-8eaa-ecd2453ed822
  - session:065295ad-311d-4965-a3c1-6f749135f2b8
---

# X-Ray Diagnostic Surface

## Overview

The X-Ray diagnostic surface is a dev-build-only nmp-devtools crate that may legally expose Trellis trace/audit receipts to tooling. It is never linked by app code and is compiled out of release artifacts, while app/native/web surfaces still cannot expose Trellis types. nmp-devtools is implemented as a sidecar with no runtime/app dependents; receipts never touch the projection pipeline.

No epic previously existed binding this workstream together; the closest ancestors (NMP #2626 and #2627) are both closed. The X-Ray epic is tracked under NMP issue #2858, which ties together NMP #2809 (ADR amendment), the nmp-devtools stream, the agent-first CLI/MCP prover, Chirp #30 (X-Ray pane), and the time-travel capsule phase. The epic was created in the nostr-multi-platform repo because that is where the ADR, the predecessor epic #2626, and the majority of workstream items live. The epic plan has five phases in order: Phase A (ADR-0075 amendment), Phase B (nmp-devtools stream + receipt shape), Phase C (agent-first CLI/MCP prover), Phase D (Chirp X-Ray pane), Phase E (time-travel capsules).

All upstream prerequisites for NMP #2809 are closed: Trellis #114 (serializable traces via PR #141), #122 (typed resource keys), #125 (bounded audit retention), #126 (scope reclamation), and NMP #2808 (shadow→authority promotion via PR #2832 merged the same day). Nothing blocks #2809 except the ADR decision itself. The plan was sent to @fable for a second-set-of-eyes review via a local Fable-model reviewer subagent reading the epic, Chirp #30, NMP #2809, ADR-0075, and Trellis trace code (trace.rs, serialized.rs, scenario.rs), after the fabric-based @fable invite did not come online.

NMP #2809 proposes a dev-build-only nmp-devtools crate exposing Trellis trace/audit data via an MCP server and redacted trace export for bug reports, gated on Trellis #114, #125, #122, and #126 — all four now closed. The diagnostic surface is a dev-build-only category in ADR-0075 where Trellis trace/audit vocabulary may legally appear, distinct from the app surface which still cannot expose Trellis types. The X-Ray diagnostic surface is receipts-only and explicitly excludes metrics, dashboards, and log aggregation. ADR-0075 is the NMP decision record establishing Trellis as a private reconciliation substrate below typed read sessions, forbidding exported Trellis types in app/native/web-facing APIs with no current carve-out for diagnostic surfaces.

<!-- citations: [^c7805-5a75d] [^c7805-efdf4] [^c7805-968ad] [^c7805-36698] [^c7805-9a3a0] [^c7805-903f3] [^c7805-7f071] -->
## Phase A — ADR Amendment & Mechanical Enforcement

Phase A amends ADR-0075 to explicitly define a 'diagnostic surface' category (dev-build-only, allowed to expose Trellis trace/audit data) distinct from the 'app surface' (unchanged: no Trellis vocabulary crosses UniFFI/web). The ADR-0075 amendment (PR #2859) adds the diagnostic surface category with the lint classifier scoped to crates/nmp-devtools/src and a regression test, closing NMP #2809. It updates doctrine-lint and the Trellis public-API leak ratchet (#2642) so the diagnostic-surface exemption is enforced mechanically, not case-by-case. The nmp-devtools release-build exclusion is mechanically enforced by a dependency-graph-based CI ratchet: an allowlist of crates permitted to depend on trellis-*, where adding a line is a ratchet violation, plus cargo tree/symbol-table checks on release artifacts.

The Phase D transport decision (declared projection vs. devtools side channel) is moved into Phase A / #2809 as a decision-gate item because the lint exemption wording depends on which transport is chosen.

<!-- citations: [^c7805-494e1] [^c7805-fcd82] [^c7805-a59bc] [^c7805-a31da] -->
## Diagnostic Surface Event Model

The diagnostic surface exposes ordered open/replace/refresh/close events with scope/projection owner, reason/provenance, transaction/revision, shared-owner/refcount state, and teardown cascade. NMP joins Trellis ResourcePlan (applied as ApplyDependentInterestDelta) outcomes into bounded retained diagnostic records instead of test-only traces. The nmp-devtools stream correlates existing relay diagnostics (wire subscriptions, relay rows, consumer counts, event counts) with the reconciliation command stream.

Receipts use structured reason codes (an NMP enum with typed params rendered to text at the edge) instead of free-form reason strings, to prevent Trellis vocabulary from leaking via debug-formatting. Rendered receipts are tested via a snapshot/regex test asserting they contain no trellis, ResourcePlan, or NodeId( pattern substrings.

The receipt shape includes transaction id, revision, sequence number, wall-clock timestamp (NMP-stamped since TransactionTrace has none), command kind (open/replace/refresh/close), outcome/status, owner-count transition (e.g. refcount 2→1), parent-scope reference for teardown cascade hierarchy, and a structured cause/parent link for why_subscription_open, in addition to projection key, view/scope label, interest shape, owner key, and relay rows. The interest shape field is flagged as the primary privacy-bearing field because it is a Nostr filter containing pubkeys and event ids that Trellis redaction never sees.

XrayReceipt and bounded ordered XrayReceiptStream plus the Trellis→receipt adapter are delivered in PRs #2860/#2861, implementing most of Phase B. The next critical-path work item after Phase A/B PRs is wiring the live feed-session reconciler into the XrayReceiptStream (Phase B's last gap), because probes over an unfed stream prove nothing.

<!-- citations: [^c7805-975cf] [^c7805-2a0b6] [^c7805-2d098] [^c7805-f5508] -->
## Owned Receipt Boundary

NMP defines an owned receipt shape for the diagnostic surface so that Chirp never receives or parses raw Trellis types (ResourcePlan, ResourceKey, Graph, etc.). The Chirp X-Ray pane (Chirp #30 v1) is a dev-build-only declared projection or devtools channel where Chirp renders receipts but never parses Trellis keys or receives ResourcePlan, ResourceKey, or Graph types. Capsule scrubbing in the Chirp X-Ray pane goes through the same NMP receipt translation as the live stream, never parsing Trellis trace JSON directly.

<!-- citations: [^c7805-0f5cf] [^c7805-7e205] -->
## nmp-devtools MCP Server

The nmp-devtools MCP server exposes five debug queries: why_subscription_open, why_view_stale, what_closed_this_relay, list_scope_inventory, and replay_session. replay_session is marked as a Phase C stub excluded from acceptance or moved to Phase E's deliverables, since capsules do not exist until Phase E. XrayCapsule envelope with schema versions, producer metadata, redaction mode, symbolication manifest, stable shareable pseudonymization, and XrayProbe helpers for all five debug queries is delivered in PR #2862.

<!-- citations: [^c7805-912fe] [^c7805-62086] [^c7805-98fbc] [^c7805-c7090] -->
## Agent-First Prover

An agent-first CLI/REPL/TUI prover is delivered before the Chirp X-Ray pane so agents can iterate on the diagnostic itself. Its exit criterion (Phase C) is that an agent can answer 'why is my feed empty?' from receipts alone. The prover includes a headless CLI that drives a real Chirp session against real relays and also loads and scrubs exported trace capsules.

Phase C defines N seeded failure scenarios against a local relay fixture: empty-source fail-closed; scope teardown closed a subscription it shouldn't; refcount held a subscription alive; subscription open but relay never connected; subscription open, EOSE, zero matching events. Each scenario has an expected root cause verified by CI running a scripted MCP call sequence. The zero-events failure scenario is deliberately included to pin down that 'receipts' includes the relay-diagnostic join.

A headless NMP host runner (Rust kernel plus feed-session running without a platform shell, including signer, storage, and network capabilities) is explicitly itemized as a Phase C work item.

A diagnosis-metric experiment targets an off-the-shelf model identifying the root cause of a seeded lifecycle bug from serialized commit/trace streams at ≥90% accuracy versus a raw-logs baseline. <!-- [^06529-b850b] -->

<!-- citations: [^c7805-75262] [^c7805-c1af4] -->
## Chirp #30 Phasing

Chirp #30 (X-Ray Feed) is a developer overlay showing live relay-subscription reconciliation with causal chains. The v0 'Shadow X-Ray' phase is obsolete and skipped (Chirp #30 is updated to strike it) because NMP #2808 promoted Trellis from shadow to authority (merged via PR #2832), leaving no shadow plan to visualize. The plan skips straight to v1 authority receipts. The v1 'Authority X-Ray' phase (gated on #2808, now done) renders receipts in a dev-build-only declared projection or devtools channel. The v2 'Time Travel' phase is gated on Trellis #114 and NMP #2809.

<!-- citations: [^c7805-e6902] [^c7805-91894] [^c7805-92d47] -->
## Phase E — Time Travel

Time travel is committed scope (Phase E of the X-Ray epic), not a stretch goal. It covers recorded/replayable debugging capsules with scrubbing, not log viewing. It provides a dev-only recorder with ring-buffer retention (bounded by both bytes and count, not just count, because a single teardown-cascade transaction can be enormous), explicit export, and a redaction policy distinguishing local/debug vs shareable modes. Time-travel capsules persist correlation data so scrubbing reconstructs full cause chains (e.g., refcount 2→1 vs 1→0→CLOSE), enabling replay-from-capsule-alone so a redacted trace attached to an issue is debuggable without the device or live relays. The time-travel phase feeds NMP capsules into Trellis's Flight Recorder workstation (Trellis #160) for desktop-side inspection rather than building a second scrubber.

Capsule v1 is explicitly defined as trace plus receipts for scrubbing/reconstruction only; replay_session(trace) is documented as reconstruction, not re-execution; re-executable capsules (script capture, which would require payload-bearing DataTransactionScripts plus identical graph topology) are an explicit non-goal or later phase with their own redaction story. A capsule embeds a symbolication manifest — a node/scope registry (NodeId/ScopeId to NMP-semantic-label mapping) captured at record time, so a capsule replayed by an agent who never saw the session is readable and cause chains can be reconstructed. Registry labels pass through the same local/shareable redaction modes as the rest of the capsule.

NMP owns a redactor over the receipt shape and ResourceKey segments, with capsule-wide stable pseudonyms (same key maps to the same token capsule-wide, preserving the type segment) so refcount/causality analysis survives redaction. This is necessary because Trellis's TraceRedactor only covers step names, ResourceKeys, and invariant names — not pubkeys, relay URLs, or event ids. Furthermore, Trellis's redact_trace function does not cover the audit_log: it redacts resource_commands, resource_coalescences, and invariant names but leaves audit_log un-redacted, and AuditEvent::ResourceOpenCoalesced carries a full un-redacted ResourceKey, so a shareable capsule built on those hooks leaks resource keys. This leak is filed as trellis#161 and is a hard gate for the Phase E time-travel phase. redact_trace must cover the audit_log and any future key-carrying fields, with a test that serializes a redacted capsule and asserts no original key material survives anywhere in the JSON.

A capsule envelope carries its own version plus embedded TRACE_FORMAT_VERSION, NMP receipt schema version, and producer metadata (app version, NMP/Trellis versions, platform, recording window, redaction-mode marker), with a compat policy of supporting N-1 or degrading to raw-JSON view instead of refusing on version mismatch. Trellis trace versioning (validate_format_version) is exact-match single-layer, hard-failing on anything but equality, with no migration path — only a typed error. Shareable-mode capsule exports note residual privacy risk from interest-set sizes, per-relay subscription counts, and transaction timing (follow count is basically recoverable), and consider bucketing counts in shareable exports to mitigate fingerprinting.

Trace recording is opt-in at runtime (disabled implies zero clones, not clone-and-drop), with a measured per-transaction overhead budget and a benchmark gate. The remaining Phase E residuals are the recorder plus byte-bounded ring buffer plus export, the reader compat policy, and merging trellis#162 for the redaction leak.

<!-- citations: [^c7805-7a79a] [^c7805-80f5b] [^c7805-fcc03] [^c7805-22252] [^c7805-1528b] -->
## Trellis Flight Recorder

Trellis Flight Recorder (Trellis #160) is a full trace-replay UI with provenance, transaction timeline, phase ladder, receipt inspector, and replay/raw-trace tabs; it is the closest existing artifact to time-travel tooling and serves as the reference implementation for time-travel tooling.

<!-- citations: [^c7805-9b551] [^c7805-439ec] -->

## tenex-Edge Forensic Instrumentation

tenex-edge has a hand-rolled structured JSON receipt system and per-session forensic logging that predate the NMP diagnostic surface. `turn_start_audit()` and `turn_check_audit()` produce a structured JSON receipt describing cursors before/after, joined channels, evaluated ambient chat, evaluated awareness (candidate channels, raw Status rows), and the exact output text emitted. The audit is an approximation that diverges from the actual render because `awareness_json()` considers a different channel set than `build_view`. `HookCallLog` is a per-session JSONL forensic log that records every hook invocation's raw stdin, process/parent-chain snapshot, redacted env, context-audit JSON, and a verbatim context-injection note with the exact injected text.

<!-- citations: [^06529-2b102] [^06529-05f9c] -->
## tenex-Edge explain CLI (Slice #210)

Slice #210 adds a `tenex-edge explain <handle>` CLI verb for retrospective instrumentation over artifact handles (`event:`, `hook:`, `llm:`, `txn:`, `session:@ts`), backed by a persisted `txn_receipts` table and an `llm_calls` ledger. The `llm_calls` ledger captures the exact transcript slice, system prompt, model, and raw response behind every distilled activity update. The instrumentation surface includes `replay` via trace-matching and a redaction policy for user content in prompts. <!-- [^06529-0510d] -->
