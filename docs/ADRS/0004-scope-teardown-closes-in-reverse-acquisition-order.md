# ADR 0004: Scope teardown emits close commands in reverse acquisition order

Status: Accepted
Date: 2026-07-03

## Context

When a scope closes, close commands for the resources it solely owned are
currently emitted in `ResourceKey` lexicographic order. That is deterministic
but meaningless: it bears no relation to how the resources came to exist or
how a host can safely tear them down.

Hosts with inter-resource teardown constraints (close a subscription before
its connection; stop a consumer before its queue) have no supported
mechanism. The only workaround is encoding ordering into key spellings, which
silently becomes load-bearing API. The first production consumer documents
exact reverse-order effect execution in its host loop, so the conflict is
real, currently masked only by shadow mode.

Trellis cannot know domain teardown dependencies (ADR 0002: the core does not
interpret payloads). But it does know something meaningful and domain-neutral:
the order in which ownership was acquired.

## Decision

Within one scope's teardown, close commands are emitted in **reverse
acquisition order**: the resource the scope acquired most recently closes
first, the one acquired earliest closes last (LIFO).

Acquisition order is defined as the order in which the scope first became an
owner of each key, recorded as a per-scope monotonic sequence at reconciliation
time. This order is fully deterministic: it derives from transaction order,
planner registration order, and command order within each plan — all already
deterministic.

Across scopes, ordering is unchanged and remains the composition tool:
children close before parents (depth-first postorder). An application that
needs "A must outlive B" across resource groups expresses it by placing B in
a child scope of A's scope.

Lexicographic ordering is removed from teardown semantics entirely. It
remains nothing more than an incidental property of unrelated iteration.

## Consequences

What improves:

- Teardown matches the universal intuition of acquisition stacks (RAII; Rust
  drops locals in reverse declaration order). A resource acquired on top of
  another is released before it.
- The key-spelling workaround dies before it becomes de facto API.
- Host loops that apply commands in order get a safe default without any new
  vocabulary or policy machinery.

What gets worse:

- Reconciliation state grows a per-(scope, key) acquisition sequence. Small
  and deterministic, but it is new state that clones with the graph.
- Traces produced before this change will not match traces produced after it.
  Pre-1.0, acceptable; trace-equality tests must be updated in the same
  change.

Constraints that follow:

- Acquisition order must survive coalescing: if a scope joins a shared
  resource (ADR 0003), its acquisition position is when *it* joined, not when
  the resource first opened.
- Re-acquisition after a full release starts a new position; the sequence
  reflects current ownership history, not all-time history.

## Alternatives considered

### Alternative 1: keep lexicographic order and document it

Rejected. Deterministic but semantically arbitrary; documents a footgun
instead of removing it, and pushes ordering into key spellings — exactly the
hidden-identity pattern ADR 0002 exists to prevent.

### Alternative 2: host-declared ordering policies as data

Deferred. A `TeardownPolicy` per scope or per planner is more machinery than
any current consumer needs, and it invites domain semantics into core.
Reverse acquisition is the safe default; a policy layer can be added by a
later ADR if a real consumer demonstrates a need it cannot express with
nested scopes.

### Alternative 3: a dependency graph between resources

Rejected. Inter-resource dependencies are domain semantics. The core would
need to interpret what resources mean to each other, violating ADR 0002. The
scope tree is already the sanctioned dependency structure.

### Alternative 4: leave ordering unspecified

Rejected. Unspecified ordering in a deterministic system is a contradiction;
hosts would depend on whatever the implementation happens to do.

## Required tests or documentation changes

- Test: scope acquires A then B then C; scope close emits Close(C), Close(B),
  Close(A) in that order.
- Test: acquisition interleaved across multiple transactions preserves global
  per-scope order.
- Test: shared resource joined late closes according to the closing scope's
  own acquisition position, and only emits `Close` when the last owner
  leaves.
- Test: nested scopes — child's closes all precede the parent's, each in its
  own reverse acquisition order.
- Test: release then re-acquire moves the key to the top of the stack.
- Oracle: owner-set equivalence unaffected (ordering does not change final
  ownership).
- Trace: `ResourceCommandTrace` ordering covered by trace-equality tests;
  update golden traces in the same change.
- Docs: SEMANTICS.md teardown section; SHADOW_MODE.md note that command-order
  comparison across this change is expected to differ (compare desired state,
  as already prescribed).
