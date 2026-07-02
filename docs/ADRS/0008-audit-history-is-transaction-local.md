# ADR 0008: Audit History Is Transaction Local

Status: Accepted
Date: 2026-07-03

## Context

`TransactionResult` already carries deterministic audit entries for the commit
that just happened. Keeping every entry again inside `Graph` made long-running
hosts accumulate unbounded history, and every transaction cloned that retained
history while preparing candidate graph state.

Graph-retained audit explanations also computed dependency paths on every
commit. That made normal commits pay for deep explanation queries even when a
host only needed the transaction result or latest summary metadata.

## Decision

Audit history belongs to `TransactionResult`, not to `Graph`.

`Graph` keeps only bounded latest explanation indexes for nodes, resources, and
outputs. The amount of retained explanation detail is selected per transaction
with `TransactionOptions::audit_explanations`:

- `Disabled` clears graph-retained explanations and records no latest indexes.
- `Summary` records latest payload-free summaries without input causes or
  dependency paths.
- `DependencyPaths` records summaries plus shortest dependency paths from
  changed inputs.

Dependency paths are computed with a reverse dependency index and breadth-first
search so returned paths are shortest in edge count and stable by node id.

## Consequences

Long-running graphs do not retain an ever-growing audit log.

Hosts that need durable history must keep the `TransactionResult` records they
care about.

Path-level explanations are explicit, so production commits do not pay for path
search unless the caller requests it.

`why_changed`, `why_resource_command`, and `why_output_frame` report latest
graph-retained explanations only. They are not historical queries.

## Alternatives considered

Bound the graph audit log with a ring buffer. Rejected because it still keeps a
second history store in the graph while `TransactionResult` already provides the
transaction receipt.

Keep path explanations always-on and optimize only the search. Rejected because
even optimized path search is optional diagnostic work.

Document first-found path semantics. Rejected because callers should get stable
shortest paths when they pay for dependency-path explanations.

## Required tests or documentation changes

- Tests must prove default explanations omit dependency paths.
- Tests must prove disabled explanations clear stale latest indexes.
- Tests must prove dependency path queries return shortest stable paths.
- Semantic docs must state that graph audit history is not retained.
