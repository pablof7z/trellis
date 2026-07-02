# Internal Alpha

Status: historical record of the pre-0.1 alpha gate. Kept for reference; not
a living document.

M17 uses Trellis in a serious prototype before public release: a small
application kernel that turns canonical source and permission inputs into
visible demand, scoped resource plans, materialized output, audit facts, and
full-recompute checks.

The alpha proof lives outside `trellis-core` in
`crates/trellis-examples/src/internal_alpha.rs` and
`crates/trellis-examples/src/internal_alpha/`. Domain vocabulary remains in the
examples crate.

## Prototype Shape

```text
source members
permission members
 -> visible derived set
 -> desired resource collection
 -> scoped resource plan
 -> materialized row output
 -> audit explanations
 -> deterministic trace replay
 -> full-recompute check
```

The prototype uses the normal public graph APIs. It does not add callbacks,
async runtime ownership, hidden retries, or domain concepts to core.

## Seeded-Bug Coverage

These tests are intended to fail when common implementation bugs are manually
introduced:

- `alpha_catches_source_shrink_missing_resource_close`
  catches forgetting to close removed resources and skipping collection diffs.
- `alpha_catches_empty_source_broadening_resource_demand`
  catches empty input sources opening broad, default, or wildcard demand.
- `alpha_catches_stale_derived_visibility_after_permission_shrink`
  catches stale derived values that keep forbidden rows/resources alive.
- `alpha_catches_scope_close_leaking_resources_or_output`
  catches resources or outputs surviving scope close.
- `alpha_catches_output_delta_sequence_that_disagrees_with_rebaseline`
  catches output ordering or delta/rebaseline coherence bugs.
- `alpha_catches_shared_resource_closing_before_last_owner`
  catches shared resources closing before the last owner leaves.
- `alpha_replay_trace_is_deterministic`
  catches nondeterministic transaction traces in the alpha integration path.

Each test also runs `assert_incremental_equals_full()` where the graph remains
live, so the alpha path exercises the oracle rather than only checking isolated
commands.

## Product Gate

### Did the abstraction simplify the integration?

Yes, for this prototype. The application describes canonical inputs, derived
visibility, collection demand, resource planning, and output materialization in
one graph loop. The host boundary remains explicit.

### Did it make bugs easier to find?

Yes. The tests assert resource ownership, output frames, audit explanations,
orphan detection, shared ownership, and full-recompute equivalence from the same
transaction results.

The audit assertion checks the resource key, transaction id, revision, command
kind, input cause, changed node set, and dependency path for a close command.
The replay assertion runs the same alpha script twice and compares
`TransactionTrace` values structurally.

### Did it create too much ceremony?

The ceremony is noticeable but acceptable for an application-kernel layer. The
heaviest parts are explicit dependency lists and separate collection/resource
steps, and both are buying determinism and auditability.

### Did resource plans feel natural?

Yes. Resource identity remains separate from command payload, and the graph can
reason about open, close, shared ownership, and teardown without executing any
host work.

### Did scopes prevent leaks?

Yes. Scope close produces resource close commands, output clear frames, empty
scope inventories, and no orphan resources.

### Did output revisions prevent stale state?

The alpha path verifies coherent delta application followed by rebaseline. That
is enough for internal confidence; later conformance work should generalize this
into reusable `trellis-testing` ledgers.

### Could a different domain use the same core?

Yes. The alpha path uses generic graph concepts only. The existing workspace
sync, mini language server, and telemetry dashboard examples use the same core
without core-domain leakage.

## Current Judgment

Keep Trellis internal and continue. The abstraction is promising enough to move
to release-candidate hardening, but public release still depends on the
`trellis-testing` story, conformance levels, host-status semantics, resource
transition policy, and release gates tracked in GitHub.
