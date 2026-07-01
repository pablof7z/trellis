# Trellis 0.1 Release Notes

Trellis 0.1 is an early, experimental, semantics-first release for design
feedback.

It is designed for application kernels that need deterministic graph
propagation, scoped resource lifecycle planning, revisioned materialized output,
auditability, replay, and incremental/full-recompute checks.

It is not production-stable.

## Positioning

Use this release to evaluate the model:

- canonical inputs;
- explicit derived nodes;
- typed collection diffs;
- data-only resource plans;
- scoped teardown;
- revisioned output frames;
- deterministic transaction traces;
- full-recompute checks;
- reusable testing helpers.

Do not treat this release as:

- battle-tested;
- production-ready;
- a drop-in replacement for existing state libraries;
- a UI framework;
- a general actor runtime;
- a networking, database, or query-cache library.

## Crates

### `trellis-core`

Core deterministic graph runtime:

- typed graph-local ids;
- transaction boundary for graph mutation;
- explicit dependencies;
- pure derived nodes;
- set/map collection nodes and structural diffs;
- data-only resource plans;
- recursive scope teardown;
- materialized output frames;
- phase traces;
- audit explanations;
- full-recompute checks.

### `trellis-testing`

Companion testing helpers:

- named scenario trace recording and structural step expectations;
- deterministic replay comparison and redacted debug dumps;
- application-owned full-recompute oracle harness;
- resource lifecycle ledger with command history assertions;
- fake host status event generation and classification;
- output frame ledger;
- audit assertions for explainable plans and frames;
- conformance suite and conformance-level reporting;
- optional `proptest`, `trybuild`, `insta`, and cargo-fuzz guidance.

### `trellis-adapter`

Runtime-neutral adapter boundary helpers. Adapters apply returned plans and emit
returned frames outside graph propagation.

### `trellis-examples`

Proof examples that keep domain vocabulary outside `trellis-core`:

- workspace-driven sync;
- mini language server;
- telemetry dashboard;
- internal alpha seeded-bug harness.

### `trellis-bench`

Benchmark smoke harness for no-op transactions, graph propagation, collection
diffs, scope close, output emission, full recompute, and trace replay.

## Testing Story

The testing story is part of the product. 0.1 demonstrates:

- `cargo test --workspace`;
- `trellis-testing` scenario replay and structural trace expectations;
- resource ledger checks for ownership, duplicate closes, forbidden broad
  demand, and host-status classes;
- output ledger checks for revision order, clear frames, and rebaseline
  coherence;
- full-recompute checks for supported graph shapes;
- benchmark smoke through `trellis-bench`;
- GitHub Actions CI for formatting, linting, tests, docs, and benchmark smoke.

## Known Limits

- Public API is unstable.
- Names and exact signatures may change.
- No automatic dependency discovery in core.
- No effect closures.
- No hidden async scheduler.
- No global runtime.
- No UI framework integration.
- No production persistence story.
- No declared MSRV yet.
- No wasm support claim yet.
- No feature matrix claim yet.

## Feedback Wanted

The most useful feedback is about:

- whether the host/graph transaction loop is explicit enough;
- whether resource plans are useful as data;
- whether scope ownership catches real lifecycle bugs;
- whether output frames are sufficient for external consumers;
- whether full-recompute checks are practical;
- whether `trellis-testing` provides the right testing boundary;
- whether examples feel like the same abstraction or separate special cases;
- what can be removed before the API grows.

Please use the structured issue templates for determinism bugs, resource
lifecycle bugs, output revision bugs, API feedback, example requests, and
prior-art comparisons.
