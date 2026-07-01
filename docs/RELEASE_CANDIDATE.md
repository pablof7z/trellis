# Public 0.1 Release Candidate Gate

M18 prepares Trellis for a deliberately narrow public 0.1 release candidate.
This document records what is demonstrated now and what remains explicitly out
of scope.

## Required Surface

Implemented:

- Three proof examples in `crates/trellis-examples`.
- Internal alpha proof in `docs/ALPHA.md`.
- Full-recompute checks via `trellis_core::FullRecomputeCheck`.
- Scope teardown tests.
- Transaction replay tests.
- Collection diff tests.
- Public API docs with `#![deny(missing_docs)]`.
- README with non-goals.
- Semantics and invariants docs.
- `#![forbid(unsafe_code)]` in crates.
- Minimal runtime dependencies.
- CI for formatting, linting, tests, docs, and benchmark smoke.

Not claimed:

- Stable API.
- Async runtime adapters as required release surface.
- UI adapters.
- Macros.
- Distributed graph support.
- Persistence.
- Final performance tuning.

## trellis-test Gate

`crates/trellis-test` is the companion testing crate for release-candidate
readiness. It currently demonstrates:

- `Scenario` for named transaction trace recording and deterministic replay.
- `ResourceLedger` for scoped resource ownership, duplicate close detection,
  forbidden broad demand checks, and stale/duplicate/late host status
  classification.
- `OutputLedger` for output frame application, revision monotonicity, clears,
  and rebaseline coherence.
- `ConformanceReport` and `ConformanceLevel` so unsupported conformance levels
  are explicit rather than silent passes.

The first `trellis-test` surface is intentionally small. It is a release
candidate proof of the testing product boundary, not a complete replacement for
the later #32 testing epic.

## CI Gate

`.github/workflows/ci.yml` runs:

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo bench -p trellis-bench --bench performance_smoke
```

Feature-combination, wasm, and MSRV jobs should be added only after the repo
declares actual feature flags, wasm support, or an MSRV.

## Review Agenda

Before publishing, review:

- Does core still match the charter?
- Did any domain concept leak into core?
- Are resource plans still data-only?
- Are scopes mandatory for resource lifecycle?
- Are output revisions coherent?
- Can every command be audited?
- Can incremental behavior be checked against full recompute?
- Are non-goals still excluded?
- Is the public API smaller than expected?
- What should be removed before release?

The last question is mandatory. Remove aggressively.
