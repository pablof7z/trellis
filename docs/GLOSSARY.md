# Glossary

Status: normative draft.

This glossary defines the vocabulary used by Trellis. New implementation concepts should use these terms unless an ADR introduces a new term.

## Actor

The host-owned execution context that serializes access to a graph.

Trellis does not implement an actor runtime. The term describes the intended ownership pattern: one owner receives external events, mutates the graph through transactions, and applies returned plans.

## Application

The domain-specific program using Trellis.

Also called the host.

The application owns I/O, runtime integration, command execution, retries, persistence, UI bridges, and domain-specific payloads.

## Audit entry

A deterministic explanatory record produced by the graph.

Audit entries explain why nodes changed, why resource commands were produced, why outputs emitted frames, and which scopes own which resources.

Audit entries are diagnostic data. They are not external side effects.

## Baseline

An output frame containing a complete current materialized state for an output key at a specific revision.

A baseline can be used by a consumer to initialize or recover state without replaying older deltas.

## Canonical input

A fact supplied by the host as source-of-truth input to the graph.

Canonical inputs are changed only through transactions.

Examples in host applications might include selected workspace, file contents, permission state, resource status, visible route, device inventory, or time supplied by the host.

## Collection diff

A deterministic structural difference between a previously committed collection and a newly computed collection.

Set diffs distinguish added and removed members.

Map diffs distinguish added, removed, and updated keys.

Collection diffs are first-class because resource lifecycle often depends on structural change, not just scalar equality.

## Collection node

A node that materializes a derived set or map and computes structural diffs across transactions.

A collection node is used when downstream resource plans or outputs need to know exactly what was added, removed, or updated.

## Command

A plain data value telling the host to perform some resource lifecycle operation.

The graph may produce commands inside a `ResourcePlan`, but it does not execute them.

The application defines the command payload type.

## Delta

An output frame that describes a change from a previous materialized output revision.

A sequence of deltas applied to a baseline should reconstruct the same output state as a later baseline for the same output key.

In the initial implementation, deltas are state-replacement deltas: applying a
delta replaces the consumer's current state for that output key with the delta
payload.

## Dependency

An explicit edge from one node, planner, or output to another node or diff.

Dependencies are part of the semantic model, not merely performance hints.

In v0.1, dependencies are declared explicitly.

## Derived node

A node whose value is computed deterministically from declared dependencies.

A derived node computation must not perform external side effects.

## Determinism

The property that the same graph, same committed input sequence, and same host-reported resource statuses produce the same transaction results.

Determinism applies to values, diffs, plans, frames, revisions, and audit output.

## Dynamic dependency

A dependency edge whose target set is determined by graph state rather than fixed at graph construction time.

Dynamic dependencies are not part of the v0.1 core. They may be introduced later only if deterministic, inspectable, scoped, and full-recompute-compatible.

## Effect

Avoid this term in the core API.

In many reactive systems, an effect is a closure that runs because a value changed. Trellis deliberately does not center that model.

Use more precise terms:

- `ResourcePlan` for resource lifecycle commands;
- `OutputFrame` for materialized output changes;
- `AuditEntry` for explanatory records.

The ADR title “Effects are data, not closures” uses the term only to contrast Trellis with closure-driven side effects.

## Empty source

A derived collection with no members.

Empty source means empty demand. It must not imply wildcard, all, default, or fallback demand.

Fallback behavior must be modeled explicitly.

## Equality gating

The rule that downstream propagation can be skipped when a newly computed value equals the previously committed value.

Equality gating must be deterministic and explicit.

## Full recompute

A recomputation of current graph state from canonical inputs and live scopes without relying on incremental dirty history.

Full recompute is used in tests to check that incremental propagation has not drifted from truth.

## Graph

The Trellis-owned state machine containing nodes, dependencies, scopes, desired resources, materialized outputs, revisions, and audit data.

The graph computes transaction results. The graph does not execute external effects.

## Host

The application using Trellis.

The host owns external events, I/O, resource command execution, runtime integration, UI bridges, and feedback of external observations into the graph as canonical inputs.

## Input node

A node whose value is supplied by the host as canonical input.

Input nodes are the root facts of graph propagation.

## Materialized output

A graph-owned output surface represented as revisioned output frames.

Materialized outputs allow external consumers to render or consume state without reading graph internals.

## Node

An identified graph element that holds or derives state.

Common node kinds:

- input node;
- derived node;
- collection node.

## Node id

A stable graph-local identifier for a node.

Node ids are not debug names. Debug names may collide; node ids must not.

## Output frame

A plain data value emitted by the graph to describe a materialized output change.

Common frame kinds:

- baseline;
- delta;
- clear;
- rebaseline;
- status.

Every output frame should include output key, owning scope, transaction id, revision, frame kind, payload, and cause or audit pointer.

## Output key

A stable graph-local or application-supplied identifier for a materialized output surface.

Output keys let consumers associate frames with the output they update.

## Plan

A plain data value describing what the host should do after a transaction.

A plan is computed by the graph and applied by the host.

## Planner

A deterministic graph computation that turns derived values or collection diffs into resource plans.

A planner must not execute the plan it returns.

## Rebaseline

An output frame that replaces or refreshes a consumer's current state with a coherent current state after a discontinuity, recovery, scope change, or configuration change.

A rebaseline differs from a normal baseline mainly in cause and intended use.

## Resource

An external thing whose desired lifecycle is determined by the graph but whose actual lifecycle is managed by the host.

Examples in applications might include subscriptions, file watchers, live queries, worker jobs, topic subscriptions, bridge handles, or background tasks.

The core treats all of these generically as resources.

## Resource command

A single plain-data operation within a resource plan.

Examples:

- open resource;
- close resource;
- replace resource;
- refresh resource.

The graph computes resource commands. The host executes them.

## Resource key

A stable identity for a desired resource.

The graph uses resource keys to compute ownership, diff desired resource state, avoid duplicate opens, and produce correct close commands.

The core may understand resource identity without understanding domain-specific payloads.

## Resource plan

The set of resource commands produced by a transaction.

A resource plan represents changes from previous desired resource state to next desired resource state.

It is data, not execution.

## Revision

A monotonically increasing version marker.

Graph revisions identify committed transaction results.

Output revisions identify materialized output state for an output key.

Revisions are used for ordering, replay, audit, and consumer recovery.

## Scope

A lifetime owner inside the graph.

Scopes own nodes, desired resources, materialized outputs, and possibly child scopes.

Closing a scope deterministically closes its child scopes first, removes its
resource demand, detaches scoped node metadata, and later clears or rebaselines
owned outputs once materialized outputs exist.

## Source

A generic term for a value or collection that downstream nodes depend on.

Prefer more precise terms in APIs: input node, derived node, or collection node.

## Structural diff

Same as collection diff.

Use structural diff when emphasizing that the graph knows what changed inside a set or map, not merely that the collection is unequal.

## Teardown

The deterministic removal of resource and output ownership caused by scope closure or source shrink.

Teardown must be represented as data in transaction results, not hidden callbacks.

## Transaction

The atomic unit of graph mutation and propagation.

A transaction stages operations, validates them, computes derived state, computes diffs, produces resource plans and output frames, commits a revision, and returns a transaction result.

## Transaction result

The data returned by a committed transaction.

A transaction result should include changed nodes, collection diffs, resource plans, output frames, revision information, and audit entries.

## Wildcard demand

A broad resource request representing all resources or an unbounded default set.

Trellis must never produce wildcard demand merely because a source is empty or missing. Wildcard behavior, if valid for an application, must be explicit.
