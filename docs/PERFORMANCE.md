# Performance And Memory Discipline

Trellis optimizes for correctness first, but the model must remain viable for
application-kernel workloads.

M15 adds a stable benchmark-smoke harness:

```sh
cargo bench -p trellis-bench --bench performance_smoke
```

The harness uses only `std::time::Instant` and `std::hint::black_box`; it does
not add Criterion or runtime dependencies to `trellis-core`.

## Current Coverage

The smoke harness covers:

- no-op transaction;
- deep graph propagation;
- wide graph propagation;
- input changed with no downstream value change;
- input changed with downstream recompute;
- large set growth;
- large set shrink;
- large map update;
- scope close with many owned resources;
- shared resource close with many owners;
- output baseline followed by delta;
- full-recompute oracle;
- transaction trace comparison.

## Regression Rules

- No hash-order nondeterminism.
- No known quadratic path for standard set/map diffs.
- No recompute of unaffected derived branches unless documented.
- Scope close should scale with owned resources where practical.
- Runtime dependencies belong outside `trellis-core`.
- Performance optimizations must not weaken graph semantics.

## Known Gaps Before Public 0.1

- Trace/audit disabled-vs-enabled cost is not separately measurable yet because
  there is not yet an audit-retention toggle.
- Allocation counts are not instrumented yet.
These should become explicit benches once those controls exist.
