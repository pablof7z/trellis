# Performance Model

Trellis optimizes for correctness, determinism, and auditability first. This
document states what that costs, so nobody has to discover it in a profiler.

## The cost model

**Per-transaction cost is O(total graph state), not O(size of the change).**

This is a consequence of deliberate design choices, not an accident:

- `begin_transaction` deep-clones the graph into a working copy; commit writes
  the working copy back. All mutation happens on the copy, so any failure —
  including a derive error halfway through recompute — discards the copy and
  leaves the committed graph untouched. Atomicity is structural, not
  best-effort.
- Collection values are snapshotted each transaction so structural diffs can
  be computed against the previous state.
- The recompute pass walks the full node order each commit. Incrementality
  means unaffected user closures are *skipped*; it does not mean the walk is
  sub-linear.
- Every store is an ordered map. Deterministic iteration order is bought at
  the price of hash-map speed and cache locality.

**What this buys:** structural rollback correctness, deterministic traces,
replayable transactions, and a full-recompute oracle that can be run at any
time.

**What it rules out:** Trellis is a control plane, not a data plane. It is
sized for graphs of hundreds to low thousands of nodes, driven at human and
protocol tempo (state changes, not event firehoses), coordinating effects
that each cost far more than a graph clone. Keep bulk payloads out of the
graph: store keys, handles, and summaries — not megabytes. If per-transaction
cost is visible in your profile, the graph is probably holding data that
belongs outside it.

## Rules that hold today

Enforced by tests and CI:

- No hash-order nondeterminism anywhere in core.
- No quadratic path for standard set/map diffs.
- Performance optimizations must not weaken graph semantics.
- Runtime dependencies stay outside `trellis-core`.

## Known gaps, tracked openly

Aspirations are listed as issues, not stated here as rules:

- Redundant whole-graph copies at commit and for clean collections
  ([#115](https://github.com/pablof7z/trellis/issues/115)).
- The creation-time cycle check is unmemoized and exponential on
  diamond-heavy dependency graphs
  ([#116](https://github.com/pablof7z/trellis/issues/116)).
- Allocation counts are not instrumented.

Resolved audit-retention shape: audit history is transaction-local, graph
explanation indexes are bounded latest-state caches, and dependency paths are
opt-in per transaction ([ADR 0008](ADRS/0008-audit-history-is-transaction-local.md)).

## Benchmarks

A benchmark-smoke harness provides timing visibility (not enforced
thresholds):

```sh
cargo bench -p trellis-bench --bench performance_smoke
```

It covers: no-op transaction; deep and wide propagation; changed input with
and without downstream change; large set growth/shrink; large map update;
scope close with many owned resources; shared-resource close with many
owners; output baseline followed by delta; the full-recompute oracle; and
transaction trace comparison.

The harness uses only `std::time::Instant` and `std::hint::black_box`; it adds
no runtime dependencies to `trellis-core`.
