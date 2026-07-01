# Trellis Charter

Status: normative draft for the first docs/spec PR.

This document defines the product and architecture contract for Trellis. It exists to prevent the implementation from drifting into a generic signal runtime, callback framework, query cache, async scheduler, or domain-specific application kernel.

Any implementation PR must be reviewable against this charter.

## North-star contract

Trellis is a deterministic reactive resource-graph runtime for Rust application kernels.

It accepts canonical input changes. It recomputes explicit derived nodes. It produces structural diffs. It turns diffs into resource plans. It emits revisioned materialized output frames. It scopes teardown. It never performs external side effects during graph propagation. It makes incremental behavior checkable against full recompute.

In compact form:

```text
canonical input changed
 -> explicit derived state changed
 -> structural diff produced
 -> resource plan emitted
 -> materialized output revised
 -> full-recompute check remains possible
```

The graph computes what should happen. The host applies it.

## One-sentence positioning

Trellis is a small Rust runtime for actor-owned dependency graphs whose derived values drive scoped external resources and revisioned materialized outputs, with deterministic propagation and full-recompute testing hooks.

## Product thesis

Many application kernels need more than value reactivity.

A normal reactive runtime answers:

```text
When value A changes, recompute B and rerun effects that observed B.
```

Trellis answers:

```text
When canonical fact A changes, recompute derived source B, diff B against the previous B, withdraw external resources that are no longer valid, install new resources, revise materialized output, and make the result auditable and full-recompute-checkable.
```

The reusable problem is not ordinary UI invalidation. The reusable problem is dependency tracking plus resource and output lifecycle.

## What Trellis is

Trellis is:

- a deterministic graph propagation core;
- a vocabulary for canonical inputs, derived nodes, collection diffs, resource plans, scopes, transactions, revisions, and materialized outputs;
- a way to turn derived collection changes into scoped resource lifecycle plans;
- a way to produce output frames as data rather than callbacks;
- a testing surface for asserting that incremental behavior equals full recompute;
- a host-owned application-kernel primitive.

## What Trellis is not

Trellis is not:

- a UI framework;
- a component runtime;
- a signal library;
- an Rx implementation;
- a query cache;
- a database;
- a scheduler;
- a networking library;
- a retry system;
- an actor framework;
- a macro DSL;
- a domain-specific application framework.

The graph must remain smaller than the applications that use it.

## Intended users

Trellis is for Rust systems where application state determines live resource ownership and materialized output.

Representative users include:

- local-first sync engines;
- offline-first application cores;
- language servers;
- telemetry dashboards;
- collaborative document kernels;
- market-data terminals;
- desktop/mobile app cores;
- plugin hosts;
- build or analysis tools with live views;
- applications that maintain scoped live queries, subscriptions, file watchers, background jobs, or materialized result surfaces.

Trellis is not intended for simple local UI state, ordinary form state, one-shot HTTP requests, or pure calculations with no resource lifecycle.

## Core invariants

The following invariants are architectural, not implementation details.

### Host owns I/O

The host application owns all external side effects:

- network I/O;
- file I/O;
- timers;
- task spawning;
- database writes;
- UI callbacks;
- resource retries;
- platform bridges.

The graph does not perform these actions.

### Graph computes deterministic plans and frames

The graph computes:

- derived node values;
- collection diffs;
- resource plans;
- materialized output frames;
- audit traces;
- revision numbers.

For the same initial graph and same committed input sequence, the graph must produce the same transaction result.

### Scopes own teardown

Every live resource and materialized output surface must be owned by one or more scopes.

Closing a scope must deterministically remove that scope's ownership and produce the required resource close commands and output clear or rebaseline frames.

A resource with no owning scope is a bug.

### Effects are data

Trellis does not run arbitrary effect closures during propagation.

The graph returns plain data:

```text
ResourcePlan
OutputFrame
AuditEntry
```

The host decides how and when to apply those results.

### Incremental behavior must be checkable

For supported graph shapes, the current incremental graph state must be comparable against a full recompute from canonical inputs.

If the implementation cannot define full recompute, the state model is too implicit.

### Empty means empty

An empty derived source must produce empty resource demand unless a separate explicit fallback node says otherwise.

Absence must not mean wildcard. Empty must not mean all. Missing must not broaden demand.

### Dependencies are explicit first

Version 0.1 is explicit-dependency-first.

A derived node declares the nodes it depends on. A resource planner declares the collection diffs or values it consumes. A materialized output declares the values it materializes.

Automatic dependency discovery is not part of the initial architecture.

## Boundary between graph and host

The host application is responsible for:

- receiving external events;
- opening a transaction;
- writing canonical inputs;
- calling graph commit;
- applying returned resource plans;
- delivering returned output frames;
- feeding resource status or external observations back as later canonical inputs.

Trellis is responsible for:

- validating graph operations;
- computing derived values;
- computing structural diffs;
- computing desired resource ownership;
- producing resource plans;
- producing revisioned output frames;
- producing audit information;
- preserving deterministic semantics.

The intended loop is:

```text
host receives external event
host starts graph transaction
host writes canonical input changes
graph recomputes explicit derived nodes
graph computes collection diffs
graph produces resource plans
graph produces output frames
graph commits revision
host receives TransactionResult
host applies resource plans
host emits output frames
host writes resource status back as later input
```

## Minimal vocabulary

The core vocabulary is deliberately small:

```text
Graph
Node
InputNode
DerivedNode
CollectionNode
CollectionDiff
ResourceKey
ResourcePlan
ResourceCommand
Scope
Transaction
Revision
MaterializedOutput
OutputFrame
FullRecompute
AuditEntry
```

Domain-specific applications may define their own command payloads and output payloads. The core must not know what those payloads mean.

## Three reference examples

These examples are normative design pressure. They are not required as fully implemented examples in the first PR, but the semantics in this PR must be able to describe them without adding new core concepts.

### Example 1: workspace-driven sync

A local-first application syncs only the data required for the active workspace and visible projects.

```text
active workspace
 -> accessible project set
 -> sync window set
 -> resource plan
 -> materialized issue board
```

Canonical inputs:

- active workspace id;
- user permission state;
- local cache facts;
- route or visible screen state.

Derived nodes:

- accessible projects;
- visible issue query shapes;
- desired sync windows;
- materialized board rows.

Resource plans:

- open newly required sync windows;
- close sync windows no longer required;
- replace windows whose shape changed.

Output frames:

- issue board baseline;
- issue row deltas;
- clear frames when a workspace closes or becomes unauthorized;
- status frames for loading, stale, or offline states.

Required invariants:

- switching workspaces closes old workspace-specific sync windows;
- permission revocation removes unauthorized resources and clears unauthorized output;
- empty accessible-project set opens no sync windows;
- incremental board state equals full recompute from canonical inputs.

### Example 2: mini language server

A language server maintains diagnostics and editor-facing output as files and project configuration change.

```text
file contents
 -> parse trees
 -> module graph
 -> affected file set
 -> diagnostics
 -> editor output frames
```

Canonical inputs:

- open file contents;
- file-system notifications;
- project configuration;
- workspace folders.

Derived nodes:

- parsed files;
- module dependency graph;
- affected files;
- diagnostics;
- file watcher requirements.

Resource plans:

- add file watchers for newly relevant paths;
- remove watchers for no-longer-relevant paths;
- cancel or replace stale analysis jobs, if represented by the host as resources.

Output frames:

- diagnostic baseline;
- diagnostic deltas;
- clear frames for deleted files;
- rebaseline frames after configuration changes.

Required invariants:

- deleting a file clears its diagnostics;
- import graph changes invalidate dependent diagnostics;
- watcher demand follows project graph changes;
- incremental diagnostics equal full project analysis for supported cases.

### Example 3: telemetry dashboard subscriptions

A fleet dashboard subscribes to telemetry only for devices visible under the current customer, site, filter, and permission state.

```text
selected customer/site/filter
 -> visible device set
 -> topic subscription set
 -> resource plan
 -> dashboard output
```

Canonical inputs:

- selected customer;
- selected site;
- user permissions;
- device inventory;
- dashboard filter state;
- incoming telemetry facts.

Derived nodes:

- visible devices;
- desired telemetry topics;
- dashboard card rows;
- alert summaries.

Resource plans:

- subscribe to added telemetry topics;
- unsubscribe from removed topics;
- keep shared topics alive while at least one live scope owns them.

Output frames:

- dashboard baseline;
- card updates;
- clear frames for revoked devices;
- rebaseline frames after filter changes.

Required invariants:

- filter shrink unsubscribes removed topics;
- permission revocation clears unauthorized cards;
- empty visible-device set subscribes to nothing;
- shared resource ownership is explicit and deterministic.

## First implementation target

The first implementation should be the smallest core that can satisfy the charter:

- explicit graph and node identity;
- transaction boundaries;
- input nodes;
- pure derived nodes;
- collection nodes with structural diffs;
- resource plans as data;
- scopes and teardown;
- materialized output frames;
- full-recompute test hooks;
- audit traces.

It should not start with macros, async integration, UI adapters, or automatic dependency discovery.

## Merge acceptance bar for the docs/spec PR

The docs/spec PR is mergeable when the following is obvious from the documents:

1. the host owns I/O;
2. the graph computes deterministic plans and frames;
3. scopes own teardown;
4. external side effects do not run during graph propagation;
5. resource plans and output frames are data;
6. dependencies are explicit in the initial design;
7. empty sources fail closed;
8. incremental behavior must be checkable against full recompute;
9. the core vocabulary is domain-neutral;
10. future semantic changes require ADRs.

## Change policy

Changes to this charter require an ADR if they affect any of the following:

- transaction phase ordering;
- dependency declaration rules;
- resource plan semantics;
- scope teardown semantics;
- materialized output semantics;
- full-recompute requirements;
- whether graph propagation may perform side effects;
- whether automatic dependency discovery is allowed.
