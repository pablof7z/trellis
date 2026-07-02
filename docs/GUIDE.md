# Guide

Trellis is a deterministic reconciler: state changes go in; resource commands,
output frames, and an auditable receipt come out. The core loop is:

```text
host receives event
host opens transaction
host writes canonical inputs
graph recomputes declared dependencies
graph computes structural diffs
graph returns resource plans and output frames
host applies plans and emits frames
host reports resource status later as canonical input
```

## Build A Graph

Create inputs for canonical facts, then create derived nodes from explicit
dependencies:

```rust
let mut graph = trellis_core::Graph::<Command, Output>::new_with_command_type();
let mut tx = graph.begin_transaction()?;
let source = tx.input::<u32>("source")?;
tx.set_input(source, 1)?;
let doubled = tx.derived(
    "doubled",
    trellis_core::DependencyList::new([source.id()])?,
    move |ctx| Ok(ctx.input(source)? * 2),
)?;
tx.commit()?;
```

Derived closures receive read-only contexts. They cannot mutate the graph.

## Model Collections

Use set/map collection nodes when downstream behavior depends on structural
changes. The graph owns the old-vs-new diff.

```text
source set changed
 -> collection diff added/removed/updated
 -> resource planner returns open/close/replace commands
```

## Plan Resources

A resource planner returns `ResourcePlan<C>` as data. The command payload `C` is
application-defined, but resource identity stays visible to Trellis through
`ResourceKey`.

The host applies returned commands outside graph propagation.

## Scope Lifetimes

Attach resource planners and outputs to scopes. Closing a scope produces
deterministic teardown and output clear frames.

## Verify

After each meaningful transaction in tests:

```rust
graph.assert_incremental_equals_full()?;
```

When ordering matters, compare transaction traces from independent runs.

For multi-step tests, use `trellis_testing::TransactionScript` and
`trellis_testing::TrellisHarness` to replay typed canonical input changes
against a fresh graph builder. Use snapshots only for redacted debug dumps;
assert resource plans, output frames, replay, and oracle results structurally.
