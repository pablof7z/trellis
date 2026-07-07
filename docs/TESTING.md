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

## Site And Demo Gates

GitHub CI runs a dedicated site/demo job on every push and pull request. The job
installs the Node toolchain and then runs:

```sh
npm ci
npm run check
npm run build:site
npm run test:observatory
```

`npm run check` validates static routes, asset links, and bundled Flight
Recorder trace fixtures. `npm run build:site` copies the public site and runs
the Observatory Vite production build against the checked-in Leak Duel WASM
bundle. `npm run test:observatory` runs the Observatory Vitest suite.

The checked-in WASM artifacts are not rebuilt on every PR. Per-PR CI consumes
them through the site/demo gates above, but source-to-WASM rebuilds require
`wasm-pack` and a full Rust-to-WASM compile. Treat those rebuilds as a
manual/release gate until a scheduled or manually dispatched CI workflow owns
the higher-cost diff check.

When a change touches `crates/trellis-observatory-engine` or generated WASM
artifacts, rebuild both checked-in bundles and commit the resulting artifacts:

```sh
npm run build:observatory:wasm
npm run build:leak-duel:wasm
npm run check
npm run build:site
npm run test:observatory
```

## Proof-Packet Validation Matrix

Public claims about Trellis need a proof packet before they appear in product
docs, demos, release notes, essays, or marketing copy. A proof packet names the
claim, the boundary being proven, the scenario, the executable command or
artifact, the expected evidence, and the known limitation.

Trellis proves graph, receipt, oracle, resource-plan, output-frame, and
structural trace consistency. It does not prove arbitrary host-world side
effects succeeded. Database writes, relay publishes, socket opens, UI bridges,
file writes, and job starts need host-owned evidence at the executor boundary,
usually through `HostConformanceLedger` or an app-specific projection ledger.

| Claim category | Boundary | Scenario | Command or artifact | Expected evidence | Known limitation |
| --- | --- | --- | --- | --- | --- |
| Deterministic replay from fresh graph builders | Same graph shape and same canonical input sequence produce matching payload-free traces and final deterministic dumps. | Fixed `Scenario`, `TransactionScript`, or `TrellisHarness` steps replayed against an app-supplied fresh builder. | `cargo test --workspace`; for serialized scripts also `cargo test -p trellis-testing --features serde`. | `assert_transaction_traces_match` succeeds; replayed harness dumps match. See [Deterministic Replay](INVARIANTS.md#deterministic-replay). | A structural trace file is a receipt, not an executable app script. Cross-process re-execution also needs `DataTransactionScript`, an app decoder, and the same graph builder. |
| Incremental state equals full recompute | Supported graph state can be rebuilt from canonical inputs and graph structure. | Transactions touching derived values, collections, resource ownership, output state, or scope lifecycle call `assert_incremental_equals_full`. | `cargo test --workspace`; `cargo bench -p trellis-bench --bench performance_smoke` for the smoke cost signal. | `FullRecomputeCheck` reports equivalence for supported shapes. See [Incremental Equals Full Recompute](INVARIANTS.md#incremental-equals-full-recompute). | The oracle proves Trellis graph state, not external host side effects or app-owned stores. |
| Source shrink closes removed resources | Removed collection members withdraw demand and emit closes for resources no longer owned. | Release-gate scenarios shrink source collections and apply the resulting plan through `ResourceLedger`. | `cargo test -p trellis-testing --test release_gate`; covered by `cargo test --workspace`. | Closed resources have no orphan owners and no duplicate close. See [Resource Plans Are Data](INVARIANTS.md#resource-plans-are-data). | The ledger applies plans as data; it does not prove the real host closed a socket or unsubscribed a relay. |
| Empty source means empty demand | Empty collections open no broad, wildcard, default, or fallback resources unless an explicit fallback node models that. | Empty-source release gate plus proof examples such as Workspace Sync Board and FleetPulse. | `cargo test --workspace`. | No forbidden or wildcard open appears in `ResourceLedger`; examples keep empty inputs empty. See [Empty Means Empty](INVARIANTS.md#empty-means-empty). | Domain fallback behavior must be modeled explicitly by the application. |
| Scope close tears down resources and outputs | Closing a scope closes owned resources, clears owned materialized outputs, and rejects later mutation on the closed scope. | Core scope teardown tests plus release-gate resource/output ledgers. | `cargo test --workspace`; targeted gate `cargo test -p trellis-testing --test release_gate`. | Resource ledgers report no orphan ownership; output ledgers see clear frames for closed scopes. See [Scope Owns Lifecycle](INVARIANTS.md#scope-owns-lifecycle). | Post-close history lives in returned transaction results, traces, audit events, or app ledgers, not in reclaimed graph nodes. |
| Shared resource closes only after last owner leaves | Resource identity is structural and close commands respect owner sets across scopes. | Shared-key scope teardown and FleetPulse examples. | `cargo test --workspace`. | Shared resources stay open while at least one owner remains and close once after the last owner leaves. See [Shared Resources Close On Last Owner](INVARIANTS.md#shared-resources-close-on-last-owner). | Conflicting host payload semantics are application policy; Trellis proves structural owner lifetimes. |
| Stale, duplicate, and late host status is classified | Host results come back as canonical input and cannot silently corrupt graph ownership. | `ResourceLedger` status classification and release-gate stale-status scenarios. | `cargo test -p trellis-testing --test resource_host_ledger`; `cargo test -p trellis-testing --test release_gate`; covered by `cargo test --workspace`. | Status events classify as current, stale, duplicate, or late, and closed-scope status does not mutate ownership. See [Failure Transparency](INVARIANTS.md#failure-transparency). | Trellis classifies reported status; it cannot observe unreported host work. |
| Adapter boundary preserves transaction results | Adapters add ergonomics without hidden propagation semantics. | Adapter boundary tests compare adapter-driven commits with direct graph transaction results. | `cargo test -p trellis-adapter`; covered by `cargo test --workspace`. | Adapter results preserve resource plans, output frames, traces, and graph state. See [Resource Plans Are Data](INVARIANTS.md#resource-plans-are-data). | Adapters remain responsible for their own API stability and host integration policy. |
| Output rebaseline reconstructs current truth | Output frames are revisioned per output and deltas/rebaselines reconstruct coherent current state. | Core materialized-output tests plus `OutputLedger` release-gate scenarios. | `cargo test --workspace`; targeted gate `cargo test -p trellis-testing --test output_audit_ledger`. | Output revisions are monotonic; clears and rebaselines reconstruct expected state. See [Output Frames Are Revisioned](INVARIANTS.md#output-frames-are-revisioned). | Output ledgers prove emitted frame coherence, not that a database, relay, or UI bridge consumed the frame correctly. |
| Serialized traces and scripts fail clearly on version mismatch | `SerializedScenario` and `DataTransactionScript` include `TRACE_FORMAT_VERSION` and reject incompatible files explicitly. | Serde trace/script fixtures and bundled Flight Recorder trace fixtures. | `cargo test -p trellis-testing --features serde`; `npm run check`. | Version mismatch produces a typed error; bundled trace fixtures match the current format. See [Replay Tests](#replay-tests). | Exact format matching is not migration support; old trace files may need a new compatibility issue. |
| Shadow-mode equivalence compares desired state, not command streams | Shadow adoption proves Trellis desired resource/output state against the existing authoritative path while Trellis is shadow-only. | Production or representative traffic mirrors the same canonical inputs into both paths and compares desired resources plus materialized outputs. | App-owned shadow harness; docs gate in [Shadow-Mode Adoption](SHADOW_MODE.md); product copy must use boundary-specific shadow-mode wording. | Zero unadjudicated divergences over defined traffic and teardown criteria; full-recompute oracle remains green. | Shadow-only has zero effect-collision risk, not zero total adoption risk. Input drift, comparison bugs, runtime cost, and promotion remain app responsibilities. |
| Demo and marketing claims link to runnable evidence or are gated | Public copy names the artifact that proves the claim or states the missing gate. | Site/demo checks, Flight Recorder structural fixtures, Leak Duel WASM demo, and marketing issue cross-links. | `npm run check`; `npm run build:site`; `npm run test:observatory`; marketing alignment issues [trellis-marketing#1](https://github.com/pablof7z/trellis-marketing/issues/1), [#2](https://github.com/pablof7z/trellis-marketing/issues/2), [#3](https://github.com/pablof7z/trellis-marketing/issues/3), and [#4](https://github.com/pablof7z/trellis-marketing/issues/4). | Site routes, asset links, bundled trace fixtures, Observatory tests, and explicit marketing gates stay aligned. | Flight Recorder currently inspects bundled structural fixtures; live production replay or a PR Trace Bot needs its own product issue and artifact before copy can claim it. |

When a public-facing claim does not fit one of these rows, add the missing
product issue first and keep the copy gated until the executable evidence
exists.

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
  `TRACE_FORMAT_VERSION` and a `GraphLabelRegistry`. Loading rejects version
  mismatches explicitly instead of silently treating old trace files as current.
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
- `HostConformanceLedger` records previewed plans, committed plans, declared
  host executors, applied host effects, and host statuses so applications can
  prove the host seam separately from graph correctness.
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
`proptest` feature is enabled. Register `ConformanceLevel::HostSeam` only when
the app records previewed plans, committed plans, and actual host effect sites.
If a required level has no registered check or explicit unsupported reason, the
runner reports that level as unsupported.

The release-gate examples in `crates/trellis-testing/tests/release_gate.rs`
cover:

- source shrink closes removed resources;
- empty source opens no broad demand;
- scope close releases resources and clears output;
- output deltas/rebaselines match current truth;
- incremental state is checked against full recompute after transactions;
- stale host status after scope changes does not mutate graph ownership.

## Host-Seam Conformance

Graph correctness does not prove host correctness. A graph can pass replay,
oracle, resource-ledger, and output-ledger checks while the host still has a
bypass path that opens a socket, starts a job, writes a file, or publishes an
event without first receiving a previewed and committed Trellis plan.

Use `HostConformanceLedger` at application executor seams:

```rust
let mut host = HostConformanceLedger::new();
host.declare_executor("subscription-executor");
host.record_preview("join workspace", &previewed_result);
host.record_commit("join workspace", &committed_result);
host.record_effects_from_commit(
    "join workspace",
    "subscription-executor",
    &committed_result,
);
host.assert_host_seam_conforms().unwrap();
```

Applications with scan hooks can also call `record_effect_site` for every
static or runtime effect site they discover. `assert_effects_use_declared_executors`
then fails if an effect site or recorded effect sits outside the declared
executors. Keep this check separate from graph conformance: a failure means the
host boundary is unsafe even if Trellis graph invariants are green.

## Projection-Frame Tests

For host projections, validate the Trellis side and the host side separately.
Use `OutputLedger` to apply returned frames and assert revision monotonicity,
clear/rebaseline coherence, closed-scope terminal frames, and typed current
state per output key. The ledger verifies that Trellis emitted coherent frames;
it does not prove that a database write, relay publish, or UI bridge consumed
those frames correctly.

Application tests should add a small host-owned projection ledger for the
external surface. Feed it the same frames the production executor receives, then
assert that the host projection matches the expected rows, outbox entries,
cursor state, or UI model. Keep that assertion at the host boundary instead of
moving database or relay I/O into Trellis.

Prefer separate `MaterializedOutput<T>` handles for unrelated projection
families. Because output payloads are typed per output, tests can call
`OutputSnapshot::state_as::<T>()` or `OutputFrame::payload_for(&output)` for the
specific surface under test instead of matching one graph-wide output enum.

The crate is not a mocking framework, async runtime, domain fixture library,
snapshot framework, property-testing framework, UI harness, database harness, or
network simulator.

The serialized artifacts have different replay boundaries. A
`SerializedScenario` is a structural trace receipt: tests can deserialize it,
inspect it, redact it, reconstruct a recorded `Scenario`, and compare it with a
freshly recorded scenario. It does not contain enough data to execute the app
again. Re-execution across process boundaries starts from
`DataTransactionScript`, an app-defined operation enum, an app-owned decoder,
and an app-owned graph builder.

## Feature Flags

`trellis-testing` has no default optional integrations. Enable only the gate
you are using:

```toml
[dev-dependencies]
trellis-testing = { version = "0.2", features = ["proptest"] }
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

The current golden-trace CI gate is
`cargo test -p trellis-testing --features serde`. It round-trips the
`serialized_trace_v4.json` fixture and validates the bundled Flight Recorder
trace files against the current `TRACE_FORMAT_VERSION`. A future standalone CLI
must preserve the same boundary: a trace-only command can validate, inspect, and
compare structural receipts, while graph re-execution requires an app-provided
data script and graph builder.

When serialized traces are intended for offline diagnostics, pass
`graph.label_registry()` to `SerializedScenario::from_scenario_with_labels`.
The serializer preserves supplied labels and fills in fallback labels for ids
referenced by the trace but missing from the final graph snapshot. Serialized
transaction traces also carry payload-neutral audit explanation receipts so
offline readers can inspect input causes and dependency paths without the live
graph. Redaction and symbolication policy remain host-owned: labels help readers
avoid interpreting bare numeric ids, but they are not graph identity.

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
