# Trellis

**A deterministic resource graph runtime for Rust application kernels.**

Trellis helps you model the part of an application where state changes imply
resource changes: subscriptions, watchers, sync windows, materialized views,
diagnostic output, or other live demand that must be opened, closed, revised,
and audited.

It is not a UI framework, a signal library, a query cache, or an async runtime.
The graph does not do I/O. It computes what should happen and returns plain
data for the host application to apply.

```text
canonical input changes
 -> derived nodes recompute
 -> collection diffs are produced
 -> resource plans are returned
 -> output frames are emitted
 -> tests can compare incremental state to full recompute
```

## Status

Trellis is early and pre-1.0. The core semantics are the stable part:
transactions, explicit dependencies, structural diffs, scoped resource
lifecycle, revisioned outputs, deterministic traces, and full-recompute checks.

Names and exact APIs may change before the project is ready to promise API
stability.

## Why Use It

Many application cores eventually grow an implicit graph:

```text
active workspace
 -> visible projects
 -> desired sync windows
 -> local materialized rows
 -> output frames
```

or:

```text
open files
 -> module graph
 -> affected files
 -> diagnostics
 -> file watchers
```

Callbacks and subscription handles work until the source set changes, a scope
closes, permissions shrink, output must be rebaselined, or a stale resource
status arrives late. Trellis makes those transitions explicit.

Use Trellis when you need to answer questions like:

- Which resources should exist after this transaction?
- Which resources must be closed because the source set shrank?
- Which output frame belongs to which revision?
- Did closing this scope tear everything down?
- Does incremental propagation match a full recompute from canonical inputs?

## Install

```toml
[dependencies]
trellis-core = "0.1"
```

To use unreleased repository changes:

```toml
[dependencies]
trellis-core = { git = "https://github.com/pablof7z/trellis", package = "trellis-core" }
```

Optional serialization support:

```toml
trellis-core = { version = "0.1", features = ["serde"] }
```

## Quick Sketch

```rust
use trellis_core::{DependencyList, Graph};

#[derive(Clone, Debug, Eq, PartialEq)]
struct Command;

#[derive(Clone, Debug, Eq, PartialEq)]
struct Output;

fn main() -> trellis_core::GraphResult<()> {
    let mut graph = Graph::<Command, Output>::new_with_command_type();
    let mut tx = graph.begin_transaction()?;

    let source = tx.input::<u32>("source")?;
    tx.set_input(source, 1)?;

    let doubled = tx.derived(
        "doubled",
        DependencyList::new([source.id()])?,
        move |ctx| Ok(ctx.input(source)? * 2),
    )?;

    let result = tx.commit()?;
    drop(tx);

    assert_eq!(result.changed_inputs, vec![source.id()]);
    assert_eq!(graph.derived_value(doubled)?, Some(&2));

    Ok(())
}
```

Derived functions read declared dependencies through a read-only context. They
do not receive `&mut Graph`, host callbacks, async handles, or resource
executors.

## Core Ideas

**Transactions are the boundary.** All graph mutation happens through explicit
transactions. A failed transaction does not partially commit.

**Dependencies are declared.** Inputs, derived nodes, collection nodes,
planners, and outputs declare what they depend on. The first version does not
use automatic dependency discovery.

**Collections produce structural diffs.** Sets and maps report added, removed,
updated, and unchanged members in deterministic order.

**Effects are data.** Resource planners return `ResourcePlan<C>`. The host owns
all I/O, retries, task spawning, timers, platform bridges, and real resource
handles.

**Scopes own lifecycle.** Resources and materialized outputs are attached to
scopes. Closing a scope produces deterministic resource teardown and output
clear frames.

**Outputs are revisioned frames.** Hosts consume baselines, deltas, clears, and
rebaselines without reading graph internals.

**Incremental behavior is testable.** Supported graph shapes can be compared
against a full recompute from canonical inputs.

## Crates

- `trellis-core`: deterministic graph runtime and public core API.
- `trellis-testing`: scenario scripts, replay checks, resource/output ledgers,
  fake host status helpers, audit assertions, and conformance support. This is
  currently kept in-repo while the public testing crate name is settled.
- `trellis-adapter`: runtime-neutral adapter boundary for applying returned
  plans and emitting returned frames outside graph propagation.
- `trellis-examples`: proof examples for workspace sync, a mini language
  server, telemetry dashboard, and a wrapper-friendly protocol subscription.
- `trellis-bench`: benchmark smoke coverage for propagation, diffs, teardown,
  output, oracle, and replay paths.

## Examples

The examples live outside the core crate so domain vocabulary does not leak into
the runtime:

- workspace-driven sync;
- mini language server diagnostics;
- telemetry dashboard subscriptions;
- wrapper-friendly protocol subscription API.

Run the workspace tests:

```bash
cargo test --workspace
```

## Documentation

Start here:

- [Guide](docs/GUIDE.md): short usage walkthrough.
- [Charter](docs/CHARTER.md): product and architecture contract.
- [Semantics](docs/SEMANTICS.md): transaction and runtime semantics.
- [Invariants](docs/INVARIANTS.md): rules mapped to tests.
- [Testing](docs/TESTING.md): oracle, replay, ledgers, and conformance support.
- [Examples](docs/EXAMPLES.md): proof example descriptions.
- [Non-goals](docs/NON_GOALS.md): what Trellis deliberately excludes.
- [Design essay](docs/DESIGN_ESSAY.md): longer rationale.

## Development

Common checks:

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

Core constraints:

- no unsafe code in the first implementation;
- no hidden global runtime;
- no external side effects during graph propagation;
- no domain-specific concepts in `trellis-core`;
- no compatibility shims before v1.0 when a cleaner shape is available.

See [AGENTS.md](AGENTS.md) and [Contributing](docs/CONTRIBUTING.md) before
opening a PR.

## License

Licensed under `MIT OR Apache-2.0`.
