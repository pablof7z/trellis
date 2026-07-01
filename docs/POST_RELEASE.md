# Post-Release Hardening

After 0.1, Trellis should use feedback without widening the core casually.

Every requested feature, bug report, example request, or integration idea must
be classified before implementation starts.

## Triage Categories

### Core Semantic Gap

Use this only when the request improves a generic Trellis primitive:

- node identity;
- dependency propagation;
- collection diffs;
- resource planning;
- scope teardown;
- transaction semantics;
- materialized output;
- testing/oracle;
- auditability.

Core semantic gaps may belong in `trellis-core`, but only after the issue
explains the invariant being improved and the tests that will prove it.

### Adapter Concern

Use this for runtime or host integration work:

- async runtime application of returned plans;
- wasm host helpers;
- tracing/logging adapters;
- serialization of trace or audit values;
- framework-specific glue outside `trellis-core`.

Adapter concerns must not change graph propagation semantics.

### Domain Concern

Use this for application vocabulary and policies:

- protocol-specific subscriptions;
- database queries;
- file watching policy;
- retry/backoff policy;
- UI presentation.

Domain concerns belong in applications, examples, or adapters. They do not
belong in `trellis-core`.

### Ergonomic Sugar

Use this for APIs that reduce ceremony without changing semantics:

- builders;
- helper constructors;
- optional macro ergonomics;
- naming cleanups;
- typed batching helpers.

Ergonomic sugar should wait until the semantic model is stable enough that the
helper does not hide important lifecycle, transaction, or scope facts.

### Documentation Gap

Use this for missing explanation, examples, migration notes, or issue-template
guidance.

Documentation gaps should cite the behavior they explain. They must not invent
unimplemented semantics.

### Non-Goal

Use this for requests that would move Trellis away from the north-star contract:

- automatic UI bindings in core;
- in-core async scheduler;
- general actor runtime;
- database query language;
- networking library;
- framework-specific adapters in core;
- effect closures during graph propagation.

Non-goals should usually be closed with a pointer to the relevant doc or kept as
prior-art discussion only.

## Current Backlog Classification

| Issue | Category | Notes |
| --- | --- | --- |
| #58 host resource status semantics | Core semantic gap | Defines canonical status input loop and stale/duplicate behavior. |
| #59 resource transition and ordering policies | Core semantic gap | Defines structural transition intent as data. |
| #60 resource identity ADR | Core semantic gap | Locks identity separate from command payload. |
| #61 wrapper-friendly protocol subscription example | Domain concern | Example proves Trellis can hide behind app-owned APIs; no core domain vocabulary. |
| #33 testing product boundary | Documentation gap | Defines `trellis-testing` boundary and non-goals. |
| #34 stable test-observable traces | Core semantic gap | Trace data underpins replay, ledgers, and audit assertions. |
| #35 scenario runner | Core semantic gap | Testing/oracle primitive for transaction scripts; belongs in `trellis-testing`, not core propagation. |
| #36 full-recompute harness | Core semantic gap | Testing/oracle primitive that preserves incremental/full-recompute equivalence. |
| #37 ResourceLedger | Core semantic gap | Testing/oracle primitive for lifecycle checks without executing resources. |
| #38 fake host executor | Adapter concern | Simulates host boundary without callbacks into graph propagation. |
| #39 OutputLedger | Core semantic gap | Testing/oracle primitive for frame coherence checks. |
| #40 trace replay/redaction/snapshots | Documentation gap | Snapshot-friendly guidance should not require serialization in core. |
| #41 proptest helpers | Ergonomic sugar | Optional integration, not mandatory dependency. |
| #42 conformance suite | Core semantic gap | Testing/oracle primitive that reports unsupported levels explicitly. |
| #43 audit assertions | Core semantic gap | Testing/oracle primitive for structural assertions over trace/audit facts. |
| #44 trybuild coverage | Ergonomic sugar | Optional compile-fail coverage for public API misuse. |
| #45 fuzzing hooks | Core semantic gap | Testing/oracle primitive for graph/lifecycle invariants without changing runtime semantics. |
| #46 testing docs | Documentation gap | Explains `trellis-testing` promise and non-goals. |

## 0.2 Candidate Filter

Possible 0.2 candidates should still pass this filter:

- controlled dynamic dependencies;
- resource sharing policies;
- typed plan batching;
- transaction compaction;
- async adapter;
- serde audit logs;
- graph visualization;
- macro ergonomics.

Each candidate must state why it belongs in its category and why it does not
weaken determinism, scope ownership, side-effect boundaries, or full-recompute
testability.

## Review Rule

Do not add a feature to core because one domain wants it. Add to core only when
it improves the generic model and can be tested through deterministic
transactions, scoped lifecycle, revisioned output, full recompute, or structural
audit facts.
