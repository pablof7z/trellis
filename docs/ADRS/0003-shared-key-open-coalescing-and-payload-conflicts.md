# ADR 0003: Shared-key open coalescing is explicit, and payload conflicts fail the commit

Status: Accepted
Date: 2026-07-03
Amended: 2026-07-10

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

`Replace` and `Refresh` participate in the same aggregate desired-state
contract. A transaction computes the final owner set and final payload for
each key before emitting host commands. Every final owner of a key must agree
on one payload. If a shared owner emits `Replace` or `Refresh` with a payload
that another final owner does not desire, the transaction fails with the same
typed payload-conflict error used for divergent `Open`.

Ownership handoff is resolved from aggregate final state, not from planner
registration order. If one scope stops owning a key while another scope starts
owning the same key with the same payload in the same transaction, ownership
moves without host churn. If the payload changes and no old owner remains,
Trellis emits a deterministic `Close` followed by `Open` for that key.

## Consequences

What improves:

- Coalescing becomes auditable: "why was no Open emitted?" has a receipt.
- The payload-conflict bug class is structurally impossible instead of
  silent.
- The oracle's owner-set and payload equivalence remains valid because
  reconciliation commits only an aggregate state all final owners agree on.

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
- Applications that want one owner to authoritatively update a shared resource
  must model that authority in their desired inputs so every final owner
  converges on the same payload, or use separate keys.

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
- Test: same-payload ownership handoff → owner set changes with no outgoing
  host command, independent of planner registration order.
- Test: changed-payload ownership handoff → deterministic `Close` then `Open`,
  independent of planner registration order.
- Test: shared-owner `Replace` and `Refresh` with a divergent payload → typed
  conflict and no partial state.
- Oracle: replay from baseline reproduces owner sets with coalescing in
  effect.
- `trellis-testing`: `ResourceLedger` assertion for "no unexplained
  coalescing" and a helper asserting a conflict fails as specified.
- Docs: SEMANTICS.md resource reconciliation section; GLOSSARY entry for
  "coalesced open".
