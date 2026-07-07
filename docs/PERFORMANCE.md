# Performance Model

Trellis optimizes for correctness, determinism, and auditability first. This
document states what that costs, so nobody has to discover it in a profiler.

## The cost model

**Per-transaction cost is O(total graph state), not O(size of the change).**

This is a consequence of deliberate design choices, not an accident:

- `begin_transaction` deep-clones the graph once into a private working copy.
  All mutation happens on the copy, so any failure — including a derive error
  halfway through recompute — discards the copy and leaves the committed graph
  untouched. Atomicity is structural, not best-effort.
- A successful commit swaps the baked working copy into place. There is no
  second whole-graph clone on commit.
- Collection values are snapshotted only when the collection is dirty, so
  structural diffs can be computed against the previous value without cloning
  clean collections.
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

## Measured constants

Fresh smoke run on 2026-07-07:

- Machine: Apple M2, arm64, macOS 26.4.
- Crate: `trellis-core` 0.2.1.
- Command: `cargo bench -p trellis-bench --bench performance_smoke`.
- Profile: Cargo `bench` profile, optimized build.

| Case | Scenario | Raw elapsed | Per transaction |
| --- | --- | ---: | ---: |
| No-op transaction | empty graph commit | 241.375µs / 100 | 2.4µs |
| Input change, no downstream change | equality-gated downstream skip | 479.291µs / 100 | 4.8µs |
| Input change with recompute | one input, one derived node | 418.625µs / 100 | 4.2µs |
| Deep graph propagation | 64-node derived chain | 3.402333ms / 20 | 170µs |
| Wide graph propagation | 64 derived nodes from one input | 1.518667ms / 20 | 75.9µs |
| 512-member set growth | 512 resource opens | 4.819625ms / 5 | 0.96ms |
| 512-member set shrink | 512 resource closes | 9.28275ms / 5 | 1.86ms |
| 512-entry map update | deterministic map diff | 325.25µs / 5 | 65.1µs |
| Scope close releasing 512 resources | teardown by ownership | 9.194708ms / 5 | 1.84ms |
| Shared resource close, many owners | 128 shared resources, 16 owners | 8.355333ms / 3 | 2.79ms |
| Output baseline then delta | materialized output frame | 95.958µs / 10 | 9.6µs |
| Full-recompute oracle | 64-resource graph with output | 1.070333ms / 5 | 214µs |
| Trace replay comparison | two 8-step serialized traces | 4.480333ms / 10 | 448µs |

These numbers are scale anchors, not contractual thresholds. The effects
Trellis coordinates — network round trips, subscription opens, watchers,
queries — commonly cost 10-100ms, which is three to four orders of magnitude
above the microsecond-level reconciler overhead in the small and medium graph
shapes Trellis is designed for.

If a real workload hits this boundary, the clean upgrade path is internal
structural sharing: persistent maps can reduce clone cost toward O(changed
structure) while preserving the same transaction semantics. The full-recompute
oracle is the guardrail that makes that kind of internal optimization safe to
land without changing the public contract.

## Rules that hold today

Enforced by tests and CI:

- No hash-order nondeterminism anywhere in core.
- No quadratic path for standard set/map diffs.
- Performance optimizations must not weaken graph semantics.
- Runtime dependencies stay outside `trellis-core`.

## Known gaps, tracked openly

Aspirations are listed as issues, not stated here as rules:

- The redundant commit copy and clean collection snapshots were removed in
  [#115](https://github.com/pablof7z/trellis/issues/115). The remaining
  transaction-wide clone happens once at `begin_transaction`.
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
