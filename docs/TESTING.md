# Testing

Trellis tests are part of the specification. A feature is not complete unless
the invariant it touches is tested.

## Required Local Gates

Run these before opening or merging a non-trivial PR:

```sh
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
git diff --check
```

When performance or benchmark code changes, also run:

```sh
cargo bench -p trellis-bench --bench performance_smoke
```

## Invariant Coverage

Before adding behavior, identify the invariant in `docs/INVARIANTS.md` that the
behavior touches. Add or update tests beside the closest existing test file.

Preferred locations:

- identity, dependencies, and scopes: `crates/trellis-core/tests/identity.rs`,
  `dependencies.rs`, `scopes.rs`;
- transactions and phase order: `transactions.rs`, `transaction_phases.rs`;
- derived values: `derived.rs`, `derived_failures.rs`;
- collection diffs: `collections.rs`, `collection_boundaries.rs`;
- resources and teardown: `resource_plans.rs`, `resource_plan_boundaries.rs`,
  `scope_teardown.rs`;
- outputs: `materialized_outputs.rs`;
- oracle and replay: `oracle_model.rs`;
- auditability: `audit_observability.rs`, `audit_causes.rs`;
- proof examples: `crates/trellis-examples/src/*.rs`;
- adapter boundary: `crates/trellis-adapter/tests/*.rs`;
- reusable testing helpers: `crates/trellis-test/tests/*.rs`.

## trellis-test

`trellis-test` is the companion testing surface for downstream-style graph
checks. It currently provides:

- `Scenario` for named transaction trace recording and deterministic replay;
- `ResourceLedger` for scoped lifecycle assertions, forbidden broad demand,
  structural command-order assertions, and host-status classification;
- `OutputLedger` for output revision and clear/rebaseline coherence;
- `ConformanceReport` for explicit supported/unsupported conformance levels.

The crate does not execute resources, hide canonical inputs, or provide an
async runtime.

## Oracle Tests

Use `graph.assert_incremental_equals_full()` after transactions that touch
derived values, collections, resources, outputs, or scope lifecycle when the
graph shape is supported by the current oracle.

## Replay Tests

Use `TransactionTrace` and `assert_transaction_traces_match` when a change
touches phase order, audit ordering, diff ordering, resource command ordering,
or output frame ordering.

## Performance Smoke

The smoke harness in `trellis-bench` is not a final benchmark suite. It is a
regression signal for accidental quadratic paths, unexpected recomputation, and
scope teardown cost. See `docs/PERFORMANCE.md`.
