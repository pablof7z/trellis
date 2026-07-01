# ADR 0001: Effects are data, not closures

Status: Accepted
Date: 2026-07-01

## Context

Trellis exists for application kernels where derived state changes drive external resources and materialized output.

A common reactive pattern is to attach an effect closure to a value:

```text
value changed -> rerun closure -> closure performs side effect
```

That model is convenient for UI state and small applications. It is too implicit for Trellis' core target.

Trellis needs to make the following properties reviewable and testable:

- which canonical inputs affected a transaction;
- which derived values changed;
- which collection members were added, removed, or updated;
- which resources should open, close, replace, or refresh;
- which scope owns each resource;
- which output frames should be emitted;
- whether the incremental result equals full recompute;
- whether a transaction is deterministic and replayable.

Arbitrary closures that perform side effects during propagation obscure these properties. They make scheduler order, cancellation, callback ownership, and hidden I/O part of correctness.

## Decision

Trellis graph propagation will produce data, not execute external side effects.

The core will use plain data structures such as:

```text
ResourcePlan
ResourceCommand
OutputFrame
AuditEntry
TransactionResult
```

The graph may compute:

```text
Open(resource_key, payload)
Close(resource_key)
Replace(resource_key, payload)
Refresh(resource_key)
Baseline(output_key, payload)
Delta(output_key, payload)
Clear(output_key)
Rebaseline(output_key, payload)
```

The graph must not execute those commands.

The host application receives the transaction result and decides how to:

- open or close actual resources;
- send output frames to consumers;
- retry failed resources;
- report resource status back as canonical inputs;
- integrate with async runtimes, UI frameworks, files, databases, or networks.

## Consequences

### Positive consequences

Resource lifecycle becomes inspectable.

A transaction result can be logged, tested, replayed, diffed, audited, and compared against full recompute.

Scope ownership becomes enforceable because resources have graph-visible keys and owners.

The core stays runtime-neutral. It does not need to choose Tokio, async-std, JavaScript promises, native event loops, or any other scheduler.

The same graph semantics can run in tests, native apps, servers, or wasm as long as the host can apply returned plans.

Side-effect timing becomes explicit. The graph computes first; the host applies after commit.

### Negative consequences

The API is more explicit than closure-driven effects.

Applications must define command payload types and write host-side plan application code.

Some simple use cases may feel verbose compared with a signal/effect API.

The core cannot hide retry, cancellation, or scheduling convenience inside propagation.

This is acceptable. Trellis optimizes for deterministic kernel semantics over minimal syntax.

## Alternatives considered

### Alternative 1: effect closures in the graph

Rejected.

Example:

```rust
on_change(follows, || {
    open_new_resources();
    close_old_resources();
});
```

This hides resource identity, scope ownership, ordering, and failure behavior inside arbitrary code. It also makes full recompute and transaction replay harder.

### Alternative 2: async effects owned by the graph

Rejected for the core.

The graph would need to own task lifetimes, cancellation, scheduler selection, wake behavior, and runtime integration. Those are host concerns.

Optional adapters may apply returned plans asynchronously outside the core.

### Alternative 3: callback subscriptions to graph events

Rejected for the core.

Callbacks make output ordering and failure behavior harder to reason about. They also encourage consumers to observe intermediate states rather than complete transaction results.

The graph should return a complete transaction result.

### Alternative 4: domain-specific resource manager in core

Rejected.

The core should understand resource identity and ownership, not domain payload semantics.

Applications define command payloads. The core computes lifecycle deltas.

## Required implementation constraints

A conforming core implementation must enforce the following:

- derived computations must not receive mutable graph access;
- planners must return resource plan data;
- materializers must return output frame data;
- transaction commit must return a transaction result;
- graph propagation must not call host I/O;
- resource commands must include graph-visible resource keys;
- resource ownership must be associated with scopes;
- output frames must include revisions;
- audit data must connect plans and frames to their causes.

## Required tests

The implementation must eventually include tests proving:

- resource plans are returned, not executed, during propagation;
- source shrink produces close commands for removed resources;
- empty source produces no resource opens;
- scope close produces close commands and output clear frames;
- shared resources close only after the last owner closes;
- transaction failure emits no partial plans or frames;
- transaction replay produces the same plans and frames;
- incremental state equals full recompute for supported graph shapes.

## Notes on terminology

The word “effect” is intentionally avoided in the core API because it commonly implies closure execution.

Preferred Trellis terms:

- `ResourcePlan` instead of resource effect;
- `ResourceCommand` instead of side effect;
- `OutputFrame` instead of output callback;
- `TransactionResult` instead of event notification.

The title of this ADR uses “effects” only to state the design boundary: effects are represented as data, not closures.
