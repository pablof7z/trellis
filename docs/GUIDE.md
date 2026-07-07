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
let mut graph = trellis_core::Graph::<Command>::new_with_command_type();
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
structured `ResourceKey` segments.

The host applies returned commands outside graph propagation.

## Use Projection Frames

Use materialized outputs when Trellis should decide that a host projection must
change, while the host still owns applying that change to SQLite, a relay, a UI,
an outbox, or another external surface. The output frame is an intent or small
view payload, not an instruction for Trellis to perform I/O.

Prefer one `MaterializedOutput<T>` per projection family. The output debug name
should name the consumer surface and entity, such as
`hook-context/session/<id>`, `cursor/session/<id>`, or `outbox/publish-intent`.
Attach the output to the same scope that owns the facts it projects; scope close
will then emit the terminal clear frame for that projection.

Do not create a fat enum just to carry unrelated output families. Output payload
typing is per output, so one graph can emit a hook-context frame, cursor frame,
and outbox frame with different payload types. Split graphs when the families
have different authority boundaries, lifetimes, release cadence, or resource
command payload types. Command payload `C` is still graph-wide, so unrelated
resource command families should either use a deliberate command enum or live in
separate graphs.

Keep bulk state out of output frames. Store large rows, files, relay messages,
or historical projection tables in host-owned storage and put stable handles,
small summaries, revisions, or intent payloads in Trellis. Most projections
should stop at authoritative output frames: Trellis decides the desired
projection and the host applies it. Make a projection fully projection-owned
only when it is small, single-typed, and drift from Trellis state is a bug.

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
