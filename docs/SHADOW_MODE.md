# Shadow-Mode Adoption

Shadow mode is the recommended way to adopt Trellis inside an application that
already has working reconciliation logic.

Run a Trellis graph beside the existing path. Feed it the same canonical
inputs. Commit real transactions. Compare what Trellis planned against what
the existing code decided. Keep the existing path authoritative until the
comparison has earned trust. Delete the existing path last.

```text
                 +--> legacy reconciliation --> effects (authoritative)
external event --+
                 +--> Trellis transaction  --> plan + frames + receipt
                                                    |
                                                    v
                                              compare, log divergence
```

## Why this works for Trellis specifically

Shadow mode is only possible for a component whose output is inert data.

Two engines that perform effects cannot run side by side: both would open
subscriptions, both would spawn work, and every side effect would double. Two
engines that return data are just compared.

Trellis never performs external side effects during propagation. A shadow
graph therefore costs one transaction per event and collides with nothing.
This is a consequence of the effects-are-data decision
([ADR 0001](ADRS/0001-effects-are-data-not-closures.md)), not an add-on
migration feature.

Shadow mode is also the same epistemic move Trellis makes internally. The core
never trusts its incremental engine; it checks it against full recompute
(`assert_incremental_equals_full`). A host adopting Trellis should not trust
Trellis either; it checks it against the code it already trusts. The clever
thing proves itself against the simple thing, at every altitude.

## The pattern

### 1. Mirror inputs

Route the same canonical input changes into the graph that the legacy path
consumes, from the same single-writer host loop. One event, one transaction.

If inputs cannot be observed at a single point, fix that first. That refactor
is a prerequisite for Trellis in any mode, and it usually improves the legacy
path by itself.

### 2. Shadow

Commit transactions normally. Do not apply the returned resource plan or
output frames. The legacy path remains the only source of effects.

### 3. Compare desired state, not command streams

Reduce both paths to the same comparable value and assert equality after each
event:

- for resources: the desired resource set (which keys should be open, with
  which owners) — not the command sequence;
- for outputs: the materialized payloads — not the frame kinds.

Command sequences may differ legitimately between an incremental path and a
recomputing path. Desired state may not. This is the same equivalence relation
the core oracle uses when it compares owner sets instead of command streams.

### 4. Adjudicate every divergence

A divergence is a bug in exactly one of the two paths. Do not assume it is the
new one. Shadow mode routinely discovers that the bespoke path was leaking or
double-opening all along.

Record each divergence with the transaction receipt (`TransactionResult`) that
produced it. The receipt states which inputs changed, what recomputed, and why
each command was emitted; adjudication starts from evidence, not from
reproduction.

### 5. Promote

When the exit criteria are met, apply Trellis's resource plan and output
frames as the authority. Optionally keep the legacy computation as a
reverse-shadow for one more release.

### 6. Delete

Remove the legacy path. Adoption is complete when the bespoke reconciliation
code is gone. Until it is deleted, you are paying for two implementations and
trusting one.

## Exit criteria

Define promotion criteria before entering shadow mode, or shadow mode becomes
a place code goes to live forever. Reasonable criteria:

- N transactions or M days of production traffic with zero unadjudicated
  divergences;
- every feature surface exercised, including teardown: real scope closes,
  source-set shrink to empty, permission revocation, rebaselines;
- every past divergence explained and fixed, on whichever side was wrong;
- the full-recompute oracle passing throughout
  (`assert_incremental_equals_full` in debug builds or sampled in release).

## Failure modes

**Permanent shadow.** The steady state of shadow mode is deletion, not
coexistence. If the new path never earns authority, either the evidence was
never collected, the criteria were never defined, or the divergences were
never adjudicated. All three are process failures, not tool failures.

**Comparing at the wrong altitude.** Asserting on command sequences produces
false divergences from legitimate ordering differences; asserting on logs
produces false agreement. Compare desired state.

**Input drift.** If the two paths observe inputs at different points, the
comparison is meaningless. Both must consume the same canonical inputs from
the same host loop.

**Shadow-only coverage.** Traffic that never exercises teardown or shrink
paths proves nothing about the transitions where lifecycle bugs live. Exit
criteria must include those transitions explicitly, even if they must be
forced.

## Real-world reference

The first production consumer of Trellis, the
[nostr-multi-platform](https://github.com/pablof7z/nostr-multi-platform)
client framework, adopted Trellis under exactly this pattern. Its ADR-0075
mandates that the first production use "must prove equivalence against the
existing path before deleting bespoke machinery": each feed-session
transaction is committed through Trellis, the plan's non-emptiness gates the
legacy full-replacement path, and an equivalence harness runs the old
computation beside the graph and asserts agreement, including scoped teardown.

## Cost

One graph clone per transaction plus the comparison. Shadow graphs are
control-plane sized (see the performance model in the README); for graphs of
hundreds of nodes this is microseconds per event. If shadow-mode cost is
measurable in your profile, the graph is probably holding bulk data that
belongs outside it.
