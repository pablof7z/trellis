# Trellis Design Essay

Trellis is a reconciler. Its relatives are Terraform's plan phase, the
Kubernetes reconcile loop, and React's commit phase — not signal libraries.

The intended use case is an application kernel that must reconcile changing
canonical facts with derived state, scoped external resources, and materialized
outputs. The hard problems are not notification ergonomics. The hard problems
are determinism, teardown, replay, output coherence, and proving that the
incremental path still matches a full recompute.

## The Loop

The core loop is:

```text
host receives external event
host starts graph transaction
host writes canonical input changes
graph recomputes explicit derived nodes
graph computes collection diffs
graph produces resource plans
graph produces output frames
graph commits revision
host receives transaction result
host applies resource plans
host emits output frames
host writes later host statuses as canonical inputs
```

Everything important about Trellis is meant to protect that loop.

## Why Resource Plans Are Data

The graph never opens sockets, reads files, writes databases, spawns tasks, or
calls UI callbacks during propagation. It returns plans.

That separation is the reason transaction output can be inspected, replayed,
audited, and tested. If resource identity were hidden inside closures or if
callbacks ran during propagation, the core could not explain teardown, detect
stale statuses, or compare incremental state to a full recompute.

## Why Collections Are First-Class

Many application resources are keyed by sets and maps:

- open a subscription per visible topic;
- keep a watcher per affected file;
- maintain rows for permitted projects;
- close resources when the source shrinks.

Consumers should not each rediscover `old set - new set`. Trellis makes
structural diffs part of graph propagation so resource planners and output
materializers receive deterministic added, removed, updated, and unchanged
facts.

## Why Scopes Are Core

External resource leaks are usually lifetime bugs. Trellis treats scope as a
semantic owner, not a convenience label.

Closing a scope must close owned resources, clear owned outputs, reclaim scoped
nodes, and leave no orphan resource ownership. Shared resources stay alive while
another scope still owns them and close when the last owner leaves.

## Why Outputs Are Frames

Hosts should not need to read graph internals to render or publish state.
Materialized outputs are returned as frames with output key, scope, transaction
id, revision, frame kind, and payload.

An honest note on the vocabulary: every frame kind except `Clear` carries the
complete payload. A `Delta` frame is a state replacement that signals "this
changed since your last frame", not a patch to be composed. The frame kinds
distinguish consumer bookkeeping states — first value, changed value,
explicit re-baseline, removal — so a consumer can be a simple state machine:

```text
Baseline  -> adopt the value
Delta     -> replace the value
Rebaseline -> replace the value, reset any downstream history
Clear      -> drop the output
```

Payload-level patches would require collection diffs to reach the output
layer; they do not today.

## Why Full Recompute Matters

Incremental systems drift when hidden state becomes a second truth source.
Trellis keeps enough state explicit that supported graph shapes can compare
incremental results with a full recompute from canonical inputs and graph
structure.

The full-recompute hook is not an optimization. It is a design constraint.

## What 0.1 Is For

The 0.1 release is for design feedback from people building application kernels
with difficult lifecycle and output correctness problems.

It should be judged on these questions:

- Does the transaction loop stay explicit?
- Are resource plans and output frames inspectable data?
- Do scopes make teardown bugs harder to miss?
- Does the testing story catch bugs that ordinary unit tests miss?
- Can examples from different domains use the same core without leaking domain
  concepts into `trellis-core`?
- Is the API smaller than expected?

It should not be judged as production-stable infrastructure. The public API is
unstable, names may change, and adapters are intentionally minimal.

## Why Adoption Starts In Shadow Mode

Because propagation returns data instead of performing effects, a Trellis
graph can run beside an application's existing reconciliation logic on real
traffic, with the existing path staying authoritative, until the comparison
has earned trust. This is the same epistemic move the core makes internally
with the full-recompute oracle, applied at the adoption boundary. See
[SHADOW_MODE.md](SHADOW_MODE.md).

## What Trellis Should Keep Resisting

Trellis should keep resisting:

- automatic dependency discovery in core;
- effect closures;
- hidden async schedulers;
- global runtimes;
- query-cache positioning;
- UI framework binding as the core abstraction;
- domain resources in `trellis-core`;
- compatibility shims before 1.0.

The project is useful only if the reusable layer stays narrow: dependency
tracking, collection diffs, resource planning, scope teardown, transaction
semantics, materialized output, auditability, and testing.
