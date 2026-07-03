# Trellis 0.2.0 Release Notes

Status: prepared by issue #147.

Trellis 0.2.0 is a pre-1.0 stabilization release after the #113+ hardening
batch. It intentionally changes public APIs where the cleaner long-term shape
was better than carrying compatibility shims.

## Highlights

- Resource planning now has open/close desired-state helpers and stronger
  lifecycle coverage for scoped resources.
- Transaction cloning and dependency reachability were tightened so ordinary
  commits do less redundant whole-graph work.
- Scope close now reclaims owned nodes, specs, planners, resources, and outputs
  instead of leaving closed-scope state alive.
- Output payload typing moved from graph-wide to per-output, so one graph can
  materialize different output payload types without a fat enum.
- `ResourceKey` is now structured identity data. Hosts recover close identity
  from key segments instead of parsing flattened strings.
- Audit history is transaction-local. Graphs keep bounded latest explanation
  indexes, and dependency-path explanations are explicit transaction options.
- Host resource status classification moved into `trellis-core`.
- Serializable scripts and traces make replay artifacts portable across
  processes.
- Proptest and fuzz smoke checks now run in CI.

## API Notes

- `Graph` is parameterized by command payload type only; output payloads are
  typed per `MaterializedOutput`.
- `ResourceKey::from_segments` should be used for multi-part product identity.
  `ResourceKey::as_str()` is diagnostic output, not an application parser
  boundary.
- `Graph::audit_log()` was removed. Use `TransactionResult.audit_log` for
  durable history and `Graph::why_*` methods for latest retained explanations.
- `TransactionOptions::audit_explanations` controls graph-retained explanation
  depth: `Disabled`, `Summary`, or `DependencyPaths`.
- Path-level audit assertions in `trellis-testing` use path-enabled
  transactions.

## Demos And Docs

- Added browser-facing launch demos for trace replay and leak comparison.
- Relaunched the minimal Trellis website and linked the current semantic ADRs.
- Added ADRs for output payload typing, structured resource keys, and
  transaction-local audit history.

## Validation

The release branch is expected to pass:

- `cargo fmt --all --check`
- `git diff --check`
- touched Rust file line-count check
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo test -p trellis-testing --features serde`
- `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps`
- `cargo bench -p trellis-bench --bench performance_smoke`
- `cargo test -p trellis-testing --features proptest`
- `cargo fuzz run resource_lifecycle -- -runs=256`
- `cargo fuzz run trace_replay -- -runs=256`
