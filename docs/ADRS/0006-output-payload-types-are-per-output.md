# ADR 0006: Output Payload Types Are Per Output

Status: Accepted
Date: 2026-07-03

## Context

Trellis originally modeled graphs as `Graph<C, O>`, where `C` was the
application resource command payload and `O` was the materialized output
payload. That made the command payload a graph-wide type, which is appropriate:
resource plans leave the graph through one command stream.

The output payload did not have the same shape. A single graph can reasonably
emit a UI view model, an index snapshot, a protocol bridge frame, and a test
projection in the same transaction. A graph-wide `O` forced applications to
create fat enums or split one logical graph into smaller graphs only to satisfy
the type system.

## Decision

The graph type is `Graph<C>`. Command payloads remain graph-wide because they
belong to one resource plan stream.

Materialized output payload typing belongs to each `MaterializedOutput<T>`.
The graph stores and compares output values through an erased `OutputPayload`
wrapper, while typed handles and frame accessors expose `T` back to consumers
when they ask for a matching output key and payload type.

`OutputFrame` is payload-bearing and cloneable, but not serialized as the
stable replay boundary. `TransactionTrace` and output-frame traces remain
payload-free structural records.

The testing output ledger also stores erased output payloads and provides typed
assertions per output key. It must not reintroduce a single graph- or
harness-wide output payload type.

## Consequences

Applications can emit multiple unrelated output payload types from one graph
without fat enums or adapter shims.

Consumers that need typed payloads must use a typed output handle or request a
specific payload type from `OutputPayload`. A mismatched key or mismatched type
returns no value instead of inventing a conversion.

Because output frames can carry arbitrary application payloads, structural
serialization is provided by traces, not by `TransactionResult` itself.

Full recompute and output reconciliation still compare payload equality inside
the graph, but the comparison is erased and scoped to the concrete output's
stored type.

## Alternatives considered

Keep `Graph<C, O>` and require an application enum for all output payloads.
Rejected because it pushes an artificial graph-wide coupling onto independent
output surfaces.

Add adapter traits or compatibility aliases around the old generic. Rejected
because Trellis is pre-1.0 and the clean API shape is preferable to preserving a
temporary boundary.

Make output frames structural only and force hosts to pull payloads from the
graph. Rejected because output frames are the data that consumers apply; forcing
hidden graph reads would weaken replay and ledger validation.

## Required tests or documentation changes

- Core tests must prove one graph can emit multiple output payload types in the
  same transaction and that typed frame access respects both output key and
  payload type.
- Semantic documentation must describe output payload typing as per-output and
  traces as payload-free.
- Testing support must store erased output payloads so harnesses can validate
  mixed-output graphs.
