# ADR 0002: Resource identity is separate from command payload

Status: Accepted
Date: 2026-07-01

## Context

Trellis resource plans must be auditable, replayable, shareable across scopes,
and testable by ledgers.

If resource identity is hidden inside arbitrary command payloads, Trellis
cannot reliably reason about ownership, teardown, sharing, stale statuses, or
deterministic replacement.

## Decision

Resource identity is represented by a stable `ResourceKey` that is separate
from command payload. ADR 0007 refines `ResourceKey` into ordered structured
segments so hosts do not need to parse identity out of flattened strings.

A resource command exposes:

- resource key;
- owning scope or ownership set;
- operation and transition policy;
- command revision or generation through the transaction result;
- application-defined payload, if any;
- cause or audit metadata.

The core understands resource keys, scopes, ownership, transition policy, and
ordering. The core does not inspect application command payloads.

Operation and transition policy are distinct structural fields in transaction
traces. The operation says what kind of resource command the graph emitted
(`Open`, `Close`, `Replace`, or `Refresh`). The transition policy says what
host-side transition is required. Today those map directly except `Replace`,
which carries `ReplaceAtomically` to make clear that the host must use a native
replacement operation or report the transition unsupported.

## Consequences

Resource ownership can be tracked generically.

Shared resources can close on last-owner removal.

Stale host statuses can be classified by key, scope, and command revision.

Command ordering can be tested structurally.

The core remains domain-neutral because application payloads remain
application-defined.

## Alternatives considered

### Alternative 1: identity inside command payload

Rejected.

Payload-only identity would make ownership, teardown, stale status rejection,
and replay depend on application-specific decoding.

### Alternative 2: domain-specific resource managers in core

Rejected.

The core should not know how a subscription, file watcher, query, job, or UI
bridge is identified beyond a stable resource key.

### Alternative 3: compatibility aliases for legacy payloads

Rejected while Trellis is pre-1.0.

The right shape is an explicit identity boundary rather than adapter layers
that preserve obsolete hidden-identity command formats.

## Required tests or documentation changes

The implementation must document and test that:

- resource commands expose `ResourceKey` separately from payload;
- source shrink emits deterministic close transitions;
- updated collection members can emit replace transitions without lowering to
  close plus open;
- operation and transition policy appear separately in transaction traces;
- host statuses are classified by resource key, scope, command revision, and
  status revision;
- unsupported transitions are host status input, not graph failure.
