# ADR 0003: Shared-key open coalescing is explicit, and payload conflicts fail the commit

Status: Accepted
Date: 2026-07-03

## Context

Resource ownership is refcounted across scopes: when a scope emits `Open` for
a `ResourceKey` that another scope already owns, the joining scope's ownership
is recorded and no outgoing `Open` command is emitted (first-owner-wins,
last-owner-close).

Today the joining scope's command payload `C` is silently discarded. Two
scopes opening the same key with *different* payloads is undetectable: the
live resource reflects whichever scope planned first, deterministically but
arbitrarily. Nothing in the transaction result records that a coalescing
decision was made at all.

This violates the auditability contract: a lifecycle decision (join instead of
open) happens during reconciliation and leaves no trace, and a disagreement
between two planners about what a shared resource *is* cannot be observed,
tested, or explained.

ADR 0002 already establishes that resource identity is the `ResourceKey`,
separate from payload, and that the core does not inspect payload meaning.
That principle decides this question when followed to its conclusion: if two
demands require different payloads, they are demands for different resources,
and must use different keys.

## Decision

When a scope emits `Open` for a key that already has one or more owners:

1. **Equal payload → coalesced join, recorded.** The scope is added to the
   owner set, no outgoing `Open` command is emitted, and the transaction
   result records an explicit coalescing event (key, joining scope, existing
   owner count) in the trace and audit log. Joining is a lifecycle decision
   and must be visible like any other.

2. **Divergent payload → typed commit failure.** The transaction fails with
   an error carrying the key, the joining scope, and the existing owners.
   A divergent payload for one key is an identity modeling error in the
   application: the two demands are different resources sharing a key. Trellis
   fails closed rather than letting the live resource silently mean two
   different things.

To compare payloads, `commit` gains a `C: PartialEq` bound. The core still
does not interpret payloads; it only requires that equality be decidable,
which is the minimum needed to distinguish "shared demand" from "conflicting
demand".

`Replace` and `Refresh` semantics are unchanged: they require ownership and
pass through. A `Replace` emitted by one owner of a shared resource replaces
it for all owners; that is already observable in the plan and is the
application's coordination problem by design.

## Consequences

What improves:

- Coalescing becomes auditable: "why was no Open emitted?" has a receipt.
- The payload-conflict bug class is structurally impossible instead of
  silent.
- The oracle's owner-set equivalence remains valid unchanged, because
  coalescing still resolves to the same owner sets.

What gets worse:

- `C: PartialEq` is a new bound on commit. Pre-1.0, acceptable; all known
  consumers already derive it.
- A payload conflict aborts the whole commit. This is deliberate fail-closed
  behavior, but it makes good conflict diagnostics mandatory: the error must
  carry key and scopes (see issue #123 for the diagnostics standard).

Constraints that follow:

- Applications that want distinct payloads for overlapping demand must encode
  the distinction in the key. This is ADR 0002's identity rule, now enforced
  rather than assumed.

## Alternatives considered

### Alternative 1: keep silent first-owner-wins

Rejected. Deterministic but unauditable; the conflict case is a real bug
(observed as a risk by the first production consumer's shared-interest
sessions) that would remain invisible.

### Alternative 2: emit Refresh or Replace with the new payload

Rejected. It guesses at domain semantics the core cannot know (is the joining
payload an upgrade, a duplicate, or a mistake?) and makes plan output depend
on planner ordering in a way that surprises.

### Alternative 3: last-writer-wins on payload

Rejected. Same observability problem as the status quo, plus it makes the
live resource churn on every join.

### Alternative 4: record the conflict as a ledger fact instead of failing

Rejected for the default. Divergent identity is a programming error, not a
runtime condition the host should adjudicate transaction by transaction. A
future transaction option could downgrade the failure to a recorded conflict
if a real consumer demonstrates the need.

## Required tests or documentation changes

- Test: second `Open`, equal payload → owner set grows, no outgoing `Open`,
  coalescing event present in trace and audit log.
- Test: second `Open`, divergent payload → commit fails with the typed error
  carrying key and scopes; no partial state (guaranteed by copy-on-commit).
- Test: coalesced join then last-owner close → single `Close`, correct owner
  attribution.
- Oracle: replay from baseline reproduces owner sets with coalescing in
  effect.
- `trellis-testing`: `ResourceLedger` assertion for "no unexplained
  coalescing" and a helper asserting a conflict fails as specified.
- Docs: SEMANTICS.md resource reconciliation section; GLOSSARY entry for
  "coalesced open".
