# trellis-testing

Companion testing support for Trellis graph invariants.

`trellis-testing` helps applications test the parts of Trellis that matter most:
deterministic transaction traces, replayable canonical input scripts, scoped
resource lifecycle, materialized output coherence, host status classification,
audit explanations, and full-recompute equivalence.

It is intentionally narrow. It is not a general Rust testing framework, async
runtime, network simulator, UI harness, or mock library.

## Install

```toml
[dev-dependencies]
trellis-testing = "0.2"
```

Optional helpers are feature-gated:

```toml
[dev-dependencies]
trellis-testing = { version = "0.2", features = ["proptest"] }
```

Available optional features:

- `proptest`: model sequence strategy helpers;
- `insta`: snapshot-friendly debug output integration points;
- `trybuild`: compile-fail gate marker;
- `fuzz`: shared helper boundary for fuzz targets;
- `serde`: enables `trellis-core/serde`.

## What It Provides

- `TransactionScript` for replayable canonical input scripts.
- `TrellisHarness` for committing named transaction steps against an
  application-owned graph builder.
- `ResourceLedger` for scoped resource lifecycle assertions.
- `OutputLedger` for revisioned frame and rebaseline assertions.
- `FakeHost` and host status helpers for success, failure, duplicate, stale,
  future, and late status classification.
- `HostConformanceLedger` for preview-to-commit-to-host-effect evidence.
- Audit assertions for explaining resource commands and output frames.
- Conformance support levels for downstream application graph tests.
- Full-recompute oracle assertion helpers.

## Example Shape

Most applications wrap Trellis in an app-owned graph builder that returns both
the graph and stable typed handles. A test then records canonical input changes
and replays them against a fresh graph:

```rust
use trellis_testing::{TransactionScript, TrellisHarness};

let app = build_app_graph();
let handles = app.handles();

let mut script = TransactionScript::new();
script
    .step("select workspace")
    .input(handles.active_workspace, workspace_id)
    .commit();

let first = TrellisHarness::replay(build_app_graph, &script)?;
let second = TrellisHarness::replay(build_app_graph, &script)?;

first.assert_replay_matches(&second)?;
```

## Design Boundary

Applications own canonical truth and graph construction. `trellis-testing` owns
the reusable Trellis-specific test machinery: scripts, traces, ledgers, replay,
audit assertions, and conformance reporting.

The crate does not hide graph propagation behind callbacks, execute real host
resources, or replace ordinary Rust test tools.

## Documentation

- Testing guide: <https://github.com/pablof7z/trellis/blob/master/docs/TESTING.md>
- Release-candidate gate: <https://github.com/pablof7z/trellis/blob/master/docs/RELEASE_CANDIDATE.md>
- Invariants: <https://github.com/pablof7z/trellis/blob/master/docs/INVARIANTS.md>
- Core crate: <https://crates.io/crates/trellis-core>

## License

Licensed under `MIT OR Apache-2.0`.
