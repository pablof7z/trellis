# ADR 0005: Scope close reclaims owned nodes

Status: Accepted
Date: 2026-07-03

## Context

Scopes are lifetime owners for resources, outputs, child scopes, and nodes.
Before this decision, closing a scope removed resource and output ownership but
only detached nodes from their scope metadata. Detached nodes kept their specs,
values, collection diffs, and planners, so they could continue recomputing
forever after the lifetime that created them had ended.

That made scope ownership weaker for nodes than for resources and outputs. It
also made dynamic-session workloads leak graph state: creating and closing
session scopes repeatedly accumulated detached nodes and closed scope metadata.

## Decision

Closing a scope reclaims nodes owned by that scope after the transaction has
produced terminal effects for the closed scopes:

1. resolve the scope subtree in deterministic child-before-parent order;
2. mark newly closed scopes and remove their resource planners;
3. reconcile resource ownership and output lifecycle for the closed scopes;
4. record transaction audit events;
5. remove node metadata, values, specs, diffs, and planners for nodes owned by
   closed scopes;
6. remove closed scope metadata and scope-tree index entries.

The graph keeps no committed tombstone for a closed scope. Reusing a scope id
after its close transaction commits is an `UnknownScope` error. Repeated close
requests for a scope already closed in the same candidate transaction may
remain a no-op because the scope has not been reclaimed yet.

Scope child lookup is backed by a parent-to-children index. Teardown must not
scan every scope to find the subtree being closed.

## Consequences

What improves:

- Scope ownership is real RAII for nodes, resources, outputs, and child scopes.
- Dynamic sessions can create and close scoped nodes without unbounded retained
  recompute work.
- Closed scope metadata no longer grows forever.
- Scope subtree lookup scales with the subtree and child count, not the total
  scope registry.

What gets worse:

- A scope id is no longer inspectable after its close transaction commits.
  Tests and hosts that want close history must use transaction results, audit
  events, traces, or their own ledger.
- Reopening or mutating by stale scope id now fails as `UnknownScope`, not
  `ScopeAlreadyClosed`.
- Debug dumps only show live scopes, not historical closed scopes.

Constraints that follow:

- Terminal resource commands and output frames must be produced before
  reclamation.
- Audit recording must happen before reclamation so close events remain visible
  in the transaction result.
- Full recompute equivalence must be asserted after reclamation, not against
  retained closed-scope tombstones.

## Alternatives considered

### Alternative 1: keep detach semantics and document them

Rejected. Documentation would make the leak intentional without solving the
dynamic-session workload that scopes are meant to support.

### Alternative 2: keep closed scope tombstones

Rejected. Tombstones preserve post-close introspection but keep a growing
registry and encourage APIs to distinguish "closed but known" from "unknown."
Pre-1.0, the cleaner model is that a closed lifetime leaves only explicit
events and host-owned ledgers behind.

### Alternative 3: add explicit `remove_node` or compaction APIs

Rejected for now. That makes cleanup an optional follow-up step and leaves the
safe default as a leak. Explicit removal can still be added later for live
nodes if a consumer needs it.

## Required tests or documentation changes

- Test: closing a parent closes child scopes first, then removes closed scopes
  and owned nodes.
- Test: closing a scope with input, derived, collection, resource planner, and
  output state emits terminal effects before reclaiming nodes and scope
  metadata.
- Test: stale scope and node ids fail as unknown after committed reclamation.
- Test: full recompute equivalence holds after reclamation.
- Docs: SEMANTICS.md teardown section, README principles, and design essay
  scope ownership language.
