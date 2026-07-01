# Non-goals

Status: normative draft.

This document lists what Trellis deliberately does not do. These exclusions are part of the architecture. They are not missing features.

The purpose of this file is to keep the core small and prevent implementation drift.

## Non-goal: UI framework

Trellis is not a UI framework.

It does not own:

- components;
- rendering;
- virtual DOMs;
- widget trees;
- UI lifecycles;
- DOM bindings;
- native view bindings.

A UI framework may consume materialized output frames produced by Trellis. Trellis itself does not render.

## Non-goal: signal runtime

Trellis is not trying to be a general-purpose signal library.

It does not center the API around:

- `Signal<T>`;
- automatic observer registration;
- effect closures that rerun because they read a value;
- component-style ownership;
- UI invalidation.

Trellis borrows ideas from reactive systems, but its core abstraction is resource-aware graph reconciliation.

## Non-goal: Rx or stream framework

Trellis is not an Observable or stream-operator library.

It does not provide:

- stream combinators;
- schedulers;
- backpressure protocols;
- time operators;
- async stream routing;
- implicit cancellation semantics.

The host may use stream libraries outside Trellis to receive events or apply plans. Trellis itself remains transaction-oriented.

## Non-goal: async scheduler

Trellis does not own an async runtime.

The core MUST NOT:

- spawn tasks;
- choose a runtime;
- schedule timers;
- poll futures;
- perform background work;
- run continuously.

The host actor owns async work and calls Trellis synchronously at transaction boundaries.

Optional adapters may help apply returned plans in an async host, but adapters must not change core semantics.

## Non-goal: automatic dependency discovery in v0.1

The initial version does not discover dependencies by recording reads inside closures.

Dependencies are explicit by default.

Automatic dependency discovery is excluded because hidden dependency edges make it harder to audit:

- what inputs affect a session;
- what resource demand depends on;
- why a resource was opened;
- why an output was cleared;
- whether full recompute is possible.

Dynamic dependencies may be considered later only through an ADR and only if the resulting dependency set is deterministic and inspectable.

## Non-goal: macro-first API

Trellis should not require procedural macros to express the core model.

Macros may become optional ergonomic sugar later. They must not be necessary to understand semantics.

The first implementation should prefer explicit builders, typed handles, plain structs, and ordinary Rust functions.

## Non-goal: callback glue

Trellis is not a callback registry.

The graph MUST NOT expose its core behavior as:

```text
on_change(callback)
on_resource_added(callback)
on_output(callback)
effect(|| do_io())
```

The graph returns data. The host applies data.

Callback-oriented adapters may exist outside the core, but the core semantic model is transaction result data.

## Non-goal: direct side effects during propagation

The graph does not perform external side effects while computing a transaction.

Forbidden inside graph propagation:

- network calls;
- file reads or writes;
- database queries;
- UI callbacks;
- task spawning;
- sleeps;
- timers;
- host resource mutation.

The graph may compute `ResourcePlan` and `OutputFrame` values that describe what the host should do.

## Non-goal: retry or backoff system

Trellis does not define retry policy.

Resource failures are host observations. The host reports them as canonical inputs if the graph should react.

Retry and backoff policy belongs to the application because it is domain-specific.

## Non-goal: database or query engine

Trellis is not a database.

It does not provide:

- storage;
- indexing;
- query languages;
- transactions over external stores;
- SQL;
- object persistence;
- replication.

An application may use Trellis to derive desired live queries or sync windows, but Trellis does not execute those queries.

## Non-goal: resource implementation library

Trellis does not know how to open, close, or replace real resources.

It does not implement:

- sockets;
- file watchers;
- database subscriptions;
- telemetry topics;
- worker processes;
- language-server protocol messages;
- platform bridge messages.

The application defines resource command payloads and applies them.

## Non-goal: global runtime

Trellis does not provide a singleton runtime.

There is no hidden global graph, global scheduler, global dependency registry, or thread-local runtime in the core.

The host creates and owns graph instances explicitly.

## Non-goal: multi-writer concurrent graph mutation in v0.1

The first implementation assumes one graph owner.

Concurrency may exist around the graph, but graph mutation is serialized through the host actor or equivalent owner.

Multi-writer graph mutation, internal locking, distributed ownership, and lock-free mutation are out of scope.

## Non-goal: distributed graph execution

Trellis does not distribute graph computation across processes or machines.

It does not solve:

- cluster membership;
- distributed consensus;
- remote dependency invalidation;
- cross-process resource ownership;
- distributed transaction logs.

## Non-goal: persistence of graph state

Trellis may expose enough deterministic data for the host to log or replay transactions, but the core does not define persistent storage in v0.1.

Persistence policy is application-specific.

## Non-goal: hidden fallback behavior

Trellis does not treat empty or missing sources as wildcard demand.

Forbidden implicit behavior:

```text
empty collection -> all resources
missing filter -> subscribe broadly
closed scope -> keep resources alive just in case
```

Fallbacks must be explicit nodes or explicit host inputs.

## Non-goal: domain vocabulary in the core

The core must not contain application-domain nouns.

Examples of forbidden core concepts:

- project sync window;
- diagnostic;
- telemetry topic;
- document;
- query;
- route;
- workspace;
- feed;
- profile;
- issue;
- device.

Those concepts belong in examples, adapters, or host applications.

The core may contain generic vocabulary:

- resource;
- command;
- scope;
- output;
- revision;
- node;
- collection;
- diff;
- transaction.

## Non-goal: replacing pure incremental computation libraries

Trellis is not primarily a pure query/recompute engine.

If an application only needs deterministic derived values and does not need scoped resource plans or materialized output lifecycle, another incremental computation library may be a better fit.

Trellis exists for the additional resource/output lifecycle layer.

## Non-goal: maximum ergonomics in the first release

Version 0.1 should prefer explicitness over convenience.

It is acceptable for early APIs to be somewhat verbose if that verbosity preserves:

- dependency identity;
- transaction boundaries;
- scope ownership;
- deterministic plans;
- auditability;
- full-recompute testing.

Ergonomics should improve only after the semantics are stable.

## Non-goal: production stability in 0.1

The first public version should not claim production stability.

The 0.1 goal is to prove semantics, examples, and invariants. API names may change.

The semantic invariants should be more stable than the API surface.
