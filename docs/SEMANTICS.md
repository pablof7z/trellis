# Trellis Semantics

Status: normative draft.

This document defines the intended runtime semantics for Trellis. It is written before implementation so code can be reviewed against a stable contract.

## Normative language

The words MUST, MUST NOT, SHOULD, SHOULD NOT, and MAY are used deliberately.

- MUST means required for a conforming implementation.
- MUST NOT means forbidden for a conforming implementation.
- SHOULD means expected unless a documented reason exists.
- MAY means permitted but not required.

## Core execution model

Trellis is an actor-owned, deterministic graph propagation core.

The host application owns the event loop and all I/O. Trellis owns graph state and deterministic propagation inside explicit transaction boundaries.

The intended control flow is:

```text
external event arrives at host
host begins transaction
host stages canonical input/scope changes
host commits transaction
graph validates staged changes
graph computes derived state
graph computes diffs
graph computes resource plans
graph computes output frames
graph commits revision
host receives TransactionResult
host applies resource plans
host emits output frames
host reports external resource status as later inputs
```

The graph MUST NOT run indefinitely. It computes one transaction result at a time.

## Graph ownership

A `Graph` owns:

- node registry;
- dependency edges;
- current node values;
- current materialized collection state;
- current desired resource ownership state;
- current materialized output state;
- scope registry;
- revision counters;
- deterministic audit state needed for debugging and testing.

The host owns:

- threads and tasks;
- async runtimes;
- timers;
- network connections;
- files;
- databases;
- UI callbacks;
- retry and backoff policy;
- actual application resource handles.

The graph MAY know that a resource with key `K` is desired by scope `S`. The graph MUST NOT know how to open a socket, read a file, execute a query, render UI, or perform any domain-specific side effect.

## Single-writer mutation

Version 0.1 assumes single-writer graph mutation.

All graph mutations MUST occur through a transaction API. The graph MUST NOT expose mutation APIs that bypass transaction boundaries.

The graph MUST reject:

- nested mutable transactions;
- mutation during propagation;
- mutation of closed scopes;
- resource attachment without an owning scope;
- output attachment without an owning scope;
- invalid node references;
- dependency cycles.

Concurrent hosts MAY send events to the actor that owns the graph, but they MUST NOT mutate the graph directly.

## Nodes

A node is an identified value or collection in the graph.

Every node MUST have:

- stable graph-local identity;
- kind;
- debug name;
- declared dependencies;
- last-changed revision;
- scope ownership, unless it is explicitly global.

Node identity MUST NOT be derived from debug name. Debug names are for diagnostics only.

### Input nodes

An `InputNode<T>` represents canonical state supplied by the host.

Input nodes are changed only by transactions.

Input nodes SHOULD represent facts, not side effects. For example, an external resource failure should be written back as a resource-status input rather than handled by hidden graph retry logic.

### Derived nodes

A `DerivedNode<T>` represents a deterministic value computed from declared dependencies.

A derived node computation MUST:

- read only declared dependencies;
- be deterministic for the same dependency values;
- avoid external I/O;
- avoid task spawning;
- avoid graph mutation;
- avoid calling host callbacks.

A derived node computation SHOULD be pure. If it uses internal caches, those caches MUST NOT become a second source of truth and MUST NOT affect deterministic output.

### Collection nodes

A `CollectionNode<K, V>` represents a derived set or map with structural diff semantics.

A collection node MUST maintain enough committed state to compute deterministic diffs between the previous committed collection and the next committed collection.

Set diffs MUST distinguish:

- added members;
- removed members;
- unchanged members when needed for audit or materialization.

Map diffs MUST distinguish:

- added keys;
- removed keys;
- updated keys;
- unchanged keys when needed for audit or materialization.

Diff ordering MUST be deterministic.

## Dependencies

Version 0.1 is explicit-dependency-first.

Every derived node, collection node, resource planner, and materialized output MUST declare the nodes or diffs it depends on.

The initial implementation MUST NOT rely on automatic dependency discovery by recording reads inside closures.

Dynamic dependencies MAY be introduced in a future ADR only if:

- the resulting dependency set is inspectable;
- the dependency set is deterministic;
- full recompute remains possible;
- audit output can explain why the dependency exists;
- resource teardown remains scoped and deterministic.

## Transactions

A transaction is the only unit of committed graph change.

A transaction may stage:

- input changes;
- scope creation;
- scope closure;
- node creation, if graph construction is dynamic;
- output attachment or detachment;
- resource-owner attachment or detachment;
- host-reported resource status changes.

A transaction MUST be atomic. If validation or propagation fails, the graph MUST NOT partially commit staged changes, resource desired state, output state, or revision counters.

A transaction result MUST include enough information for the host and tests to observe what happened:

- transaction id;
- resulting graph revision;
- changed inputs;
- changed derived nodes;
- collection diffs;
- resource plans;
- output frames;
- audit entries;
- errors, if any.

## Transaction phase order

A conforming implementation MUST define a stable transaction phase order.

The initial phase model is:

```text
1. StageOperations
2. ValidateTransaction
3. CommitCanonicalInputs
4. MarkDirtyNodes
5. RecomputeDerivedNodes
6. RecomputeCollectionNodes
7. ComputeStructuralDiffs
8. ResolveScopeLifecycle
9. ProduceResourcePlans
10. ProduceOutputFrames
11. CommitGraphRevision
12. ReturnTransactionResult
```

Committed transaction results include this phase trace as deterministic data.

The host applies resource plans and output frames only after the transaction result is returned.

No phase MAY perform external I/O.

No phase MAY call host callbacks.

If a future implementation changes phase order, it MUST update this document and add or update an ADR.

## Determinism

For the same:

- initial graph definition;
- initial graph state;
- transaction sequence;
- host-reported resource statuses;
- configured equality rules;

Trellis MUST produce the same:

- derived values;
- collection diffs;
- desired resource ownership;
- resource plans;
- output frames;
- revisions;
- audit trace.

Implementations MUST avoid nondeterministic iteration order in public transaction results. Hash-map iteration order MUST NOT leak into diffs, plans, frames, or audit output.

Time, randomness, I/O, task scheduling, and external resource status MUST enter the graph as canonical inputs supplied by the host.

## Transaction Trace Observability

Every committed transaction exposes a deterministic structural trace. The trace
is the public contract between core runtime semantics and reusable test support.

Core trace data includes:

- transaction id and committed graph revision;
- staged input writes and whether each write changed committed state;
- changed input nodes;
- initial dirty roots;
- recomputed and changed derived nodes;
- recomputed and changed collection nodes;
- payload-neutral collection diff summaries;
- resource command identity, operation, scope, and transition policy;
- output frame identity, scope, revision, and frame kind;
- scope lifecycle events;
- audit entries;
- transaction phase order;
- optional invariant results added by test support.

Trace order MUST be stable. Node ids, collection diff summaries, resource
commands, output frames, scope events, audit entries, and phase events MUST NOT
depend on hash-map iteration order or host callback timing.

Core trace values are structural. Application command payloads, output payloads,
and collection member payloads are not required to appear in the core trace.
`trellis-testing` MAY layer typed script data, invariant results, ledgers,
redaction, and snapshot-friendly dumps on top of the core trace without making
core depend on a snapshot or serialization framework.

Serialization support for structural trace data is optional and gated by the
`serde` feature. The default core build does not require `serde`.
`TransactionResult` can carry application output payloads and is not the stable
serialized replay boundary.

## Equality and propagation

Nodes MAY use equality gating to avoid downstream propagation when the computed value is unchanged.

Equality rules MUST be explicit and deterministic.

A no-op input change SHOULD NOT advance node revisions or produce downstream diffs unless the node is configured to treat identical writes as significant.

## Resource plans

A resource plan is plain data describing desired lifecycle changes for external resources.

A resource plan MUST NOT execute resources.

A resource command SHOULD include:

- resource key;
- operation;
- owning scope or ownership delta;
- command payload supplied by the application;
- transaction id;
- graph revision;
- cause or audit pointer.

Resource identity MUST be represented by graph-visible `ResourceKey` data. A
resource key is an ordered list of identity segments; hosts MUST recover product
identity from those segments rather than parsing a flattened string. Payloads
remain application-defined, but identity, owning scope, operation, transition
policy, and cause metadata are Trellis-visible structural data.

Allowed operation vocabulary SHOULD begin small:

```text
Open
Close
Replace
Refresh
Noop
```

Allowed transition policy vocabulary SHOULD begin small:

```text
Open
Close
ReplaceAtomically
Refresh
Noop
```

`ReplaceAtomically` means the host must use a domain-native replacement
operation or report that the transition is unsupported as later host resource
status. Trellis does not pretend to guarantee external atomicity.

Replace is distinct from close plus open. A collection member update MAY emit a
replace transition, and a source shrink MUST emit deterministic close
transitions for removed resource keys.

Transition policy MUST appear in structural transaction trace data, not only in
debug prose.

Resource plan order MUST be deterministic. Collection-driven plans SHOULD use
deterministic collection diff order. Scope teardown close commands MUST be
ordered by deterministic scope teardown and resource-key order.

The core MUST understand resource identity and ownership well enough to produce teardown commands. The core MUST NOT understand domain-specific command payload semantics.

### Desired resource state

The graph maintains desired resource ownership, not actual resource success.

Actual resource success or failure is reported by the host as later canonical input.

Example:

```text
graph emits Open(Resource A)
host attempts to open Resource A
host observes failure
host writes ResourceStatus(A, Failed) as input in a later transaction
```

The graph MUST NOT hide retry or backoff behavior inside propagation.

### Host resource status

Host resource status is canonical input to a later transaction.

The host applies a `ResourcePlan`, observes success, failure, unsupported
transition, closed, or stale outcomes, then writes that observation as input.

The canonical loop is:

```text
graph emits ResourcePlan
host applies ResourcePlan
host observes success/failure/closed/stale/unsupported
host reports HostResourceStatus as later canonical input
graph derives status, output, or retry demand from that input
```

A host status input SHOULD expose:

```text
resource_key
scope
command_revision
status_revision
status payload
```

Status for the current command revision MAY affect derived status or output by
normal input propagation.

Status for an old command revision is stale. Stale status MAY produce audit
facts but MUST NOT create resource ownership.

Status for a closed scope or removed owner is late. Late status MUST NOT reopen
a scope, reattach resource ownership, or resurrect resource demand.

Duplicate status delivery MUST be deterministic and idempotent or explicitly
rejected. A duplicate MUST NOT corrupt ownership or output revisioning.

Host status MUST NOT mutate graph state outside a transaction, run callbacks
into graph propagation, or trigger graph-internal retry/backoff. Retry and
backoff remain application policy.

## Scopes and teardown

A scope is a lifetime owner.

A scope may own:

- nodes;
- desired resources;
- materialized outputs;
- child scopes.

Every live desired resource MUST have at least one owning scope.

Every materialized output surface MUST have an owning scope.

Closing a scope MUST deterministically remove that scope's ownership from all
resources and outputs it owns. If a resource has no remaining owners after
scope closure, the transaction MUST produce a close command for that resource.
If an output is owned by the closed scope, the transaction MUST produce a
clear or finalization frame for that output.

Closing a scope MUST reclaim nodes owned by that scope after close commands,
output terminal frames, and transaction audit events have been produced. Node
metadata, values, specs, diffs, and planners attached to the reclaimed nodes
MUST NOT remain live.

Repeated close requests for a scope already closed in the same candidate
transaction MAY be a no-op. After a close transaction commits, the closed scope
is reclaimed; later use of that scope id MUST be rejected as unknown rather
than silently treated as a tombstone.

Closing a parent scope MUST close child scopes or otherwise remove their ownership according to a documented deterministic order.

The graph MUST provide a way to detect orphaned resources and outputs in tests.

The initial scope teardown order is:

```text
1. Resolve the scope subtree in deterministic child-before-parent order.
2. Reject unknown scopes before mutating the candidate graph.
3. For each newly closed scope in that order:
   a. mark the scope closed;
   b. remove resource planners owned by that scope.
4. Remove closed-scope resource ownership in the same child-before-parent order.
5. Emit resource close commands when a resource loses its final live owner.
6. Produce output clear/rebaseline frames for closed-scope outputs once outputs exist.
7. Record transaction audit events for the close.
8. Reclaim closed scope metadata and nodes owned by the closed scopes.
```

M7 implements this order for scopes, nodes, resources, and materialized output
clear frames.

## Shared resources

A resource MAY be desired by multiple scopes.

Shared ownership MUST be explicit.

If one scope closes but another live scope still owns the same resource key, the graph MUST NOT emit a final close command for that resource. It MAY emit an ownership-change audit entry.

A final close command MUST be emitted only when the resource key is no longer desired by any live scope.

## Empty-source semantics

An empty collection means an empty collection.

An empty source MUST NOT imply wildcard demand, all resources, default resources, or fallback resources.

If an application wants fallback behavior, it MUST model that fallback as an explicit derived node or explicit canonical input.

Examples:

```text
visible_device_set = {}
 -> desired_topic_set = {}
 -> no topic subscriptions
```

```text
accessible_project_set = {}
 -> desired_sync_windows = {}
 -> no sync windows
```

## Materialized outputs

A materialized output is a revisioned surface emitted by the graph as data.

The host may send output frames to a UI, bridge, log, network, test harness, or any other consumer. The graph does not call those consumers directly.

Output payload type is per materialized output, not per graph. `Graph<C>` keeps
one graph-wide command payload stream, while each `MaterializedOutput<T>` names
its own payload type. Output frames carry erased `OutputPayload` values; typed
consumers recover `T` through the matching output handle or by explicitly asking
for that payload type.

An output frame SHOULD include:

- output key;
- owning scope;
- transaction id;
- output revision;
- frame kind;
- payload;
- cause or audit pointer.

Initial frame kinds SHOULD include:

```text
Baseline
Delta
Clear
Rebaseline
Status
```

The implementation uses state-replacement deltas: a `Delta` payload is a
coherent replacement for the output's consumer state. This keeps output frames
typed per output and deterministic while leaving structural output deltas for a
later design if the examples prove they are needed.

Output revisions MUST be monotonic per output key.

A consumer SHOULD be able to reconstruct the latest output state by starting from a baseline and applying subsequent deltas, or by accepting a later rebaseline.

A scope close MUST produce clear or finalization frames for outputs owned only by that scope.

## Full recompute

Full recompute is the process of deriving current graph state from canonical committed inputs and live scope state without using incremental dirty propagation history.

For supported graph shapes, Trellis MUST provide testing hooks to compare incremental state against full recompute.

Full recompute comparison SHOULD include:

- derived scalar values;
- materialized collections;
- desired resource ownership;
- materialized output state;
- output baseline equivalence;
- relevant audit or cause information where practical.

The M10 implementation provides `full_recompute()`,
`full_recompute_check()`, and `assert_incremental_equals_full()` for supported
graph shapes. The check recomputes derived scalar values and collections from
canonical inputs, rebuilds desired resource ownership from current collection
state, and rematerializes active output state. M10 also exposes deterministic
payload-free transaction traces and generated model scripts for replay and
property-style invariant tests.

If a feature makes full recompute impossible, it MUST be rejected or require a new ADR explaining why the feature belongs in the core despite that cost.

## Error semantics

A transaction MUST NOT partially commit if validation or propagation fails.

The initial implementation SHOULD treat user derivation, planning, or materialization failures as transaction failures unless a later ADR defines recoverable error nodes.

For M9, failed transactions return typed errors and MUST NOT emit partial
resource plans, output frames, or graph revisions.

The graph SHOULD distinguish:

- programmer errors;
- invalid graph references;
- cycle errors;
- derive errors;
- planning errors;
- materialization errors;
- host-reported resource status.

Host-reported resource failure is not graph failure. It is canonical input.

The M9 implementation exposes deterministic failure categories:

```text
ProgrammerError
DeriveError
PlanError
OutputError
HostResourceStatus
```

`HostResourceStatus` is plain input data. It does not retry, reopen, back off,
or otherwise execute policy inside the graph. Applications that want retry
policy must model it explicitly in host code or graph inputs/derived nodes.

## Audit semantics

The graph SHOULD produce deterministic audit information sufficient to answer:

- which transaction changed this node;
- which dependency caused this node to change;
- which collection diff produced this resource command;
- which scope owns this resource;
- why this output frame was emitted;
- whether a resource was closed due to source shrink or scope closure;
- whether an output was cleared due to empty source, rebaseline, or teardown.

If a resource command cannot be explained by graph state, it should not exist.

The M11 implementation persists deterministic audit state on the graph and
exposes `audit_log()`, `why_changed()`, `why_resource_command()`,
`why_output_frame()`, `dependency_path()`, and `scope_resource_inventory()`.
Resource and output explanations are payload-free: they identify transactions,
revisions, scopes, changed nodes, input causes, collection diffs where present,
frame/command kinds, and dependency paths without inspecting host command or
output payload semantics.

## Prohibited behavior

A conforming core implementation MUST NOT:

- open sockets;
- perform file I/O;
- execute database queries;
- spawn async tasks;
- sleep;
- schedule timers;
- call UI callbacks;
- call user closures for side effects during propagation;
- use hidden global state;
- use hidden dependency discovery in v0.1;
- allow resources without scopes;
- allow output frames without revisions;
- allow transaction partial commits.

## Required test invariants

Implementation MUST eventually provide tests for:

- source shrink closes removed resources;
- empty source opens no resources;
- scope close closes resources and clears outputs;
- shared resource closes only after last owner closes;
- no-op equality does not propagate unexpectedly;
- output deltas reconstruct the same state as a later baseline;
- transaction failure is atomic;
- transaction replay is deterministic;
- incremental state equals full recompute for supported graph shapes.
