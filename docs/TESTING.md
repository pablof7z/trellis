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
- reusable testing helpers: `crates/trellis-testing/tests/*.rs`.

## trellis-testing

The Cargo package for the companion crate is `trellis-testing`; the Rust crate
path is `trellis_testing`. The old `trellis-test` package name is not usable
because crates.io already has the normalized `trellis_test` name, so the public
companion package uses `trellis-testing`.

- `Scenario` for named transaction trace recording and deterministic replay;
- `TransactionScript` and `TrellisHarness` for deterministic typed transaction
  scripts against app-supplied graph builders;
- `DataTransactionScript` and `SerializedScenario` behind the `serde` feature
  for versioned JSON scripts and structural trace files;
- `ResourceLedger` for scoped lifecycle assertions, forbidden broad demand,
  structural command-order assertions, and host-status classification;
- `OutputLedger` for output revision and clear/rebaseline coherence;
- `ConformanceReport` for explicit supported/unsupported conformance levels.

The testing promise is narrow:

```text
Trellis does not only run your graph. It helps you test that your incremental
graph is equivalent to full recompute, that resource lifetimes are scoped, that
source shrink withdraws demand, and that materialized outputs remain coherent
across revisions.
```

`trellis-testing` provides structural semantic assertions:

- `Scenario` records named transaction traces, checks deterministic replay, and
  asserts expected resource command or output frame traces by step.
- `TransactionScript` records typed canonical input writes and custom
  transaction operations, then replays them against a fresh builder.
- `DataTransactionScript` records app-defined data operations instead of
  closures. The app supplies the decoder that stages those operations into
  typed Trellis input writes, so persistent scripts stay serializable without
  making core guess how to decode application payloads.
- `SerializedScenario` writes named `TransactionTrace` values with
  `TRACE_FORMAT_VERSION`. Loading rejects version mismatches explicitly instead
  of silently treating old trace files as current.
- `TrellisHarness` commits exactly one transaction per step, applies resource
  and output ledgers, records invariant-hook results into the step trace, and
  compares final deterministic graph dumps after replay.
- `FullRecomputeOracle` lets the application own canonical truth while Trellis
  owns the comparison harness.
- `ResourceLedger` applies resource plans without executing resources and checks
  lifecycle ownership, duplicate closes, forbidden broad demand, command
  generations, closed-scope leaks, and stale/duplicate/late status classes.
- `FakeHost` converts emitted resource plans into explicit host status events
  that tests feed back through normal canonical input APIs.
- `OutputLedger` applies materialized output frames and checks monotonic
  revisions, clear/rebaseline coherence, and closed-scope terminal-frame rules.
- Audit helpers assert resource commands and output frames are explainable from
  graph-visible cause data.
- `ConformanceSuite`, `conformance()`, and `ConformanceReport` make
  unsupported conformance levels explicit instead of treating skipped checks as
  passes.

Snapshots are useful for audit/debug output, not semantic correctness. Use
`Scenario::to_redacted_debug_string`, `ResourceLedger::to_redacted_debug_string`,
`OutputLedger::to_redacted_debug_string`, and `Graph::debug_dump` when a stable
dump helps debug a failure. Redact application-specific resource keys and output
payloads before snapshotting. Keep the actual pass/fail condition structural.

Use focused scenario tests when the application needs to prove one concrete
behavior, such as "closing workspace A closes subscription X." Use the
conformance suite when the application wants one executable gate that declares
which Trellis invariant families it supports and which hooks are intentionally
absent.

Minimal apps can start with deterministic trace checks:

```rust
use trellis_testing::{
    ConformanceCheckResult, ConformanceLevel, conformance,
};

#[test]
fn trellis_conformance() {
    let report = conformance()
        .check(
            ConformanceLevel::DeterministicTrace,
            "same input sequence produces same trace",
            || {
                if replay_trace(build_graph) == replay_trace(build_graph) {
                    ConformanceCheckResult::passed()
                } else {
                    ConformanceCheckResult::failed("scenario workspace-open trace differed")
                }
            },
        )
        .unsupported(
            ConformanceLevel::GeneratedModelSequences,
            "app has not opted into generated sequences yet",
        )
        .run()
        .unwrap();

    assert!(report.supports(ConformanceLevel::DeterministicTrace));
}
```

For richer apps, register checks at the level where the app has supplied the
required hooks: fixed scenarios for trace replay, `ResourceLedger` for lifecycle
checks, `OutputLedger` for output coherence, `FullRecomputeOracle` for
incremental/full equivalence, and generated sequence checks when the
`proptest` feature is enabled. If a required level has no registered check or
explicit unsupported reason, the runner reports that level as unsupported.

The release-gate examples in `crates/trellis-testing/tests/release_gate.rs`
cover:

- source shrink closes removed resources;
- empty source opens no broad demand;
- scope close releases resources and clears output;
- output deltas/rebaselines match current truth;
- incremental state is checked against full recompute after transactions;
- stale host status after scope changes does not mutate graph ownership.

The crate is not a mocking framework, async runtime, domain fixture library,
snapshot framework, property-testing framework, UI harness, database harness, or
network simulator.

## Feature Flags

`trellis-testing` has no default optional integrations. Enable only the gate
you are using:

```toml
[dev-dependencies]
trellis-testing = { version = "0.1", features = ["proptest"] }
```

Current feature boundaries:

```text
proptest   strategy helpers around Trellis model scripts
insta      snapshot-friendly trace/debug output examples
trybuild   compile-fail gate documentation; Trellis uses trybuild as a dev gate
fuzz       shared helpers for cargo-fuzz targets outside normal cargo test
serde      optional serialization for structural trace and script data
```

`proptest`, `insta`, `trybuild`, and cargo-fuzz are optional tools. They should
not be required for basic downstream scenario tests.

The `proptest` feature provides shrinkable sequence pieces rather than a new
property-testing framework. Applications keep their own domain enum and compose
Trellis generic pieces with app-owned strategies:

```rust
use proptest::prelude::*;
use trellis_testing::proptest::{
    InputChange, ModelSequence, OutputChange, ScopeChange, canonical_input_change,
    model_sequence_strategy, output_rebaseline, scope_change,
};

#[derive(Clone, Debug)]
enum AppStep {
    Input(InputChange<WorkspaceId>),
    Scope(ScopeChange<ScreenId>),
    Output(OutputChange<OutputName>),
}

fn app_sequence_strategy() -> impl Strategy<Value = ModelSequence<AppStep>> {
    let step = prop_oneof![
        canonical_input_change(workspace_id_strategy()).prop_map(AppStep::Input),
        scope_change(screen_id_strategy(), screen_id_strategy()).prop_map(AppStep::Scope),
        output_rebaseline(output_name_strategy()).prop_map(AppStep::Output),
    ];
    model_sequence_strategy(step, 0..=64)
}
```

`ModelSequence::to_replay_debug_string()` and the generated value's `Debug`
output are intended for failure messages and snapshots, so a shrunk failure can
be replayed as an ordered sequence.

## Oracle Tests

Use `graph.assert_incremental_equals_full()` after transactions that touch
derived values, collections, resources, outputs, or scope lifecycle when the
graph shape is supported by the current oracle.

## Replay Tests

Use `TransactionTrace` and `assert_transaction_traces_match` when a change
touches phase order, audit ordering, diff ordering, resource command ordering,
or output frame ordering.

## Compile-Fail Tests

Trellis itself uses `trybuild` under `crates/trellis-testing/tests/ui` for
type-level API guarantees such as typed input handles not crossing application
domains. Downstream wrappers should add compile-fail tests only when their
wrapper API relies on Rust type errors as part of the contract.

## Fuzz Tests

Fuzz targets live under `fuzz/` and are not workspace members. Run them with
`cargo fuzz` when investigating graph/lifecycle/replay invariants:

```sh
cargo fuzz run resource_lifecycle
cargo fuzz run trace_replay
```

Application-specific fuzz targets should generate canonical input, scope,
resource-status, and output sequences, then reuse `Scenario`, `ResourceLedger`,
`OutputLedger`, and application oracles for assertions.

## Performance Smoke

The smoke harness in `trellis-bench` is not a final benchmark suite. It is a
regression signal for accidental quadratic paths, unexpected recomputation, and
scope teardown cost. See `docs/PERFORMANCE.md`.
