# trellis-core

Deterministic resource graph primitives for Rust application kernels.

`trellis-core` is the runtime crate for Trellis. It models the part of an
application where canonical state changes imply resource lifecycle changes:
subscriptions, watchers, sync windows, materialized views, diagnostics, or
other live demand that must be opened, closed, revised, and audited.

The graph does not perform I/O. It computes deterministic transaction results:
derived values, structural diffs, resource plans, revisioned output frames,
audit traces, and full-recompute checks. The host application applies returned
plans and reports external status back as later canonical input.

```text
canonical input changes
 -> derived nodes recompute
 -> collection diffs are produced
 -> resource plans are returned
 -> output frames are emitted
 -> tests can compare incremental state to full recompute
```

## Status

Trellis is early and pre-1.0. The intended stable contract is semantic:
transactions, explicit dependencies, structural diffs, scoped resource
lifecycle, revisioned outputs, deterministic traces, and full-recompute checks.

Names and exact APIs may change before 1.0.

## Install

```toml
[dependencies]
trellis-core = "0.1"
```

Optional serialization support:

```toml
[dependencies]
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

## What It Provides

- Typed input, derived, and collection nodes.
- Explicit dependency lists.
- Atomic transactions.
- Deterministic set/map collection diffs.
- Data-only `ResourcePlan<C>` values.
- Scoped resource ownership and teardown.
- Revisioned materialized output frames.
- Deterministic transaction traces and audit queries.
- Full-recompute checks for supported graph shapes.

## What It Does Not Do

`trellis-core` is not a UI framework, signal library, query cache, database,
networking library, retry system, actor runtime, async scheduler, or macro DSL.
It should sit inside a host-owned application kernel. The host owns I/O,
runtime integration, retries, platform bridges, and real resource handles.

## Documentation

The full project documentation is in the repository:

- Guide: <https://github.com/pablof7z/trellis/blob/master/docs/GUIDE.md>
- Charter: <https://github.com/pablof7z/trellis/blob/master/docs/CHARTER.md>
- Semantics: <https://github.com/pablof7z/trellis/blob/master/docs/SEMANTICS.md>
- Invariants: <https://github.com/pablof7z/trellis/blob/master/docs/INVARIANTS.md>
- Testing: <https://github.com/pablof7z/trellis/blob/master/docs/TESTING.md>

## License

Licensed under `MIT OR Apache-2.0`.
