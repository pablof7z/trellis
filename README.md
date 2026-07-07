# Trellis

**Deterministic reconciliation for Rust. State changes in; effect plans and
receipts out.**

If your application derives live resources from changing state — relay
subscriptions, file watchers, sync windows, database queries, socket rooms —
you have a class of bug that no unit test catches: the subscription that
outlives the state that justified it. The watcher opened twice. The window
that never closed because the workspace switched while a request was in
flight. You cannot assert your way out, because the bug lives in the *gap*
between your state and your side effects.

Trellis closes that gap by refusing to let the two drift apart. Your state
lives in a transactional graph. Side effects never happen inside it. Instead,
every committed transaction returns plain data:

- a **resource plan** — the exact open/close/replace/refresh commands implied
  by what changed, with refcounted ownership across scopes;
- **output frames** — revisioned materialized values for external consumers;
- a **receipt** — a deterministic trace of what changed, what recomputed,
  which commands were emitted, and why.

Your host code applies the commands. Trellis guarantees they are the right
ones — and can prove it, on demand, against a full recompute from scratch.

```text
canonical input changes
 -> derived nodes recompute
 -> collection diffs are produced
 -> resource plans are returned
 -> output frames are emitted
 -> the receipt explains all of it
 -> tests compare incremental state to full recompute
```

Trellis is not a UI framework, a signal library, a query cache, or an async
runtime. The graph does no I/O. It decides what should happen and hands you
the evidence.

## The shape of the bug it kills

Every long-lived client eventually grows an implicit graph:

```text
active workspace -> visible projects -> desired sync windows
open files -> module graph -> affected files -> file watchers
followed accounts -> relay set -> live subscriptions
```

Callbacks and subscription handles work until the source set shrinks, a scope
closes, permissions are revoked, or a stale status arrives late. Then a
resource leaks — silently, unfalsifiably, in production. Trellis makes every
one of those transitions an explicit, ordered, testable command.

## Show me

A workspace drives sync windows. Switch the workspace; Trellis tells you
exactly which windows to close and which to open — you never write that diff
yourself:

```rust
use std::collections::{BTreeMap, BTreeSet};
use trellis_core::{DependencyList, Graph, ResourceKey, ResourcePlan};

#[derive(Clone, Debug, Eq, PartialEq)]
enum SyncCommand {
    OpenWindow(String),
}

fn key(project: &str) -> ResourceKey {
    ResourceKey::from_segments(["sync", project])
}

let mut graph = Graph::<SyncCommand>::new_with_command_type();
let mut tx = graph.begin_transaction()?;
let scope = tx.create_scope("workspace")?;

// Canonical inputs: the state you own.
let active = tx.input::<Option<String>>("active-workspace")?;
let grants = tx.input::<BTreeMap<String, BTreeSet<String>>>("grants")?;
tx.set_input(active, Some("one".to_owned()))?;
tx.set_input(grants, my_grants)?;

// Derived: the projects the active workspace may see.
let projects = tx.derived(
    "project-set",
    DependencyList::new([active.id(), grants.id()])?,
    move |ctx| {
        let active = ctx.input(active)?;
        let grants = ctx.input(grants)?;
        Ok(active
            .as_ref()
            .and_then(|ws| grants.get(ws))
            .cloned()
            .unwrap_or_default())
    },
)?;

// Collection: diffed structurally on every commit.
let windows = tx.set_collection(
    "sync-window-set",
    DependencyList::new([projects.id()])?,
    move |ctx| Ok(ctx.derived(projects)?.clone()),
)?;

// Planner: turns the diff into commands. Data out, no I/O.
tx.set_resource_planner(windows, scope, move |ctx| {
    let mut plan = ResourcePlan::new();
    for added in &ctx.diff().added {
        plan.open(
            key(&added.value),
            ctx.scope(),
            SyncCommand::OpenWindow(added.value.clone()),
        );
    }
    for removed in &ctx.diff().removed {
        plan.close(key(&removed.value), ctx.scope());
    }
    Ok(plan)
})?;

tx.commit()?;
```

Now the payoff. Switch workspaces:

```rust
let mut tx = graph.begin_transaction()?;
tx.set_input(active, Some("two".to_owned()))?;
let result = tx.commit()?;

// The exact lifecycle commands implied by the change — nothing more:
// Close(sync:a), Close(sync:b), Open(sync:c)
for command in result.resource_plan.commands() {
    my_runtime.apply(command);
}

// And the engine will prove its own bookkeeping honest:
graph.assert_incremental_equals_full()?;
```

Shared ownership comes free: if two scopes open the same key, the resource
opens once and closes only when the last owner leaves. Close a scope and every
resource it solely owned gets a deterministic close command, children-first —
Rust's ownership discipline, extended across the process boundary.

## The receipt

Every commit returns a `TransactionResult` that is a complete, deterministic
record of the transaction: which inputs changed, which nodes recomputed and
which actually changed, the structural diffs, the resource plan, the output
frames, scope lifecycle events, an audit log, and the phase trace. Audit
queries use bounded latest-state indexes on the graph; shortest dependency-path
explanations are available when a transaction opts into them.

Output payload types belong to each materialized output, not to the graph. One
graph can emit different frame payload types for different output surfaces
without wrapping them in a shared enum.

That record is the product. It is what makes Trellis systems reviewable in a
PR, diffable across runs, replayable in tests — and legible to tooling and AI
agents that need to reason about why a system did what it did, without
parsing logs.

## It checks itself

Incremental systems drift; Trellis assumes its own implementation is guilty
until proven innocent. `assert_incremental_equals_full()` rebuilds all derived
state, collections, resource ownership, and outputs from canonical inputs and
compares against the incrementally maintained state. The oracle ships in the
core crate, not the test crate. The companion `trellis-testing` crate adds
resource and output ledgers (no duplicate closes, no orphans, no revision
regressions), scenario replay with trace equality, and conformance levels.

## Don't trust it — shadow it

Because Trellis returns plans instead of executing effects, shadow-only
adoption has zero effect-collision risk: run a graph *beside* your existing
bespoke reconciliation logic, feed both paths the same canonical inputs, and
compare desired resource/output state on real production traffic while your
existing path stays authoritative. Two engines that perform effects would
collide; two engines that return data just get compared. Deleting your bespoke
code is the *last* step of adoption, not the first.

This is not hypothetical: Trellis's first production consumer, the
[nostr-multi-platform](https://github.com/pablof7z/nostr-multi-platform)
client framework, runs Trellis in exactly this equivalence mode — every
feed-session transaction is computed by both paths and checked for agreement
before the bespoke path earns deletion.

See [Shadow-mode adoption](docs/SHADOW_MODE.md) for the input-mirroring
prerequisite, desired-state comparison rule, exit criteria, and failure modes.

## Where it sits

Trellis is a reconciler, not an incremental-computation engine. Its relatives
are not Salsa or fine-grained signals; they are:

- **Terraform's plan phase** — desired changes as reviewable data, before
  anything executes;
- **the Kubernetes reconcile loop** — ownership, teardown, convergence — but
  in-process and deterministic;
- **React's commit phase** — generalized beyond the DOM to any external
  resource;
- **Elm's managed subscriptions** — extended to *every* effect type, not just
  subscriptions.

## The performance deal

Trellis buys determinism and atomicity with copies: transactions snapshot the
graph, and per-commit cost scales with total graph state, not with the size of
the change. This is deliberate. Trellis is a **control plane, not a data
plane**: it is built for graphs of hundreds to thousands of nodes coordinating
effects that each cost far more than a graph clone (network subscriptions,
watchers, queries). Keep bulk payloads out of the graph — store handles,
keys, and summaries, not megabytes.

## When not to use Trellis

- Compiler-style workloads with hundreds of thousands of nodes needing
  sub-millisecond incremental updates — use Salsa.
- High-frequency state (games, animation frames, per-keystroke recomputation
  over large graphs).
- Apps whose side effects are trivial or one-shot — a reconciler with receipts
  is overkill for a script.
- UI signal graphs — Trellis can feed a UI through output frames, but it is
  not a rendering library.

## Status

Trellis is early and pre-1.0. The semantics are the stable part: transactions,
explicit dependencies, structural diffs, scoped resource lifecycle, revisioned
outputs, deterministic traces, and full-recompute checks. Names and exact APIs
may change before an API-stability promise. The current implementation
optimizes for correctness, small code, and auditability; known performance and
semantic gaps are tracked openly in the issue tracker.

Trellis is consumed in the production tree of a real multi-platform client
today, in shadow mode: its reconciliation plans are computed and
equivalence-checked against a bespoke path on real traffic. Promotion of the
plan to authority is the milestone the roadmap is sequenced around.

## Install

```toml
[dependencies]
trellis-core = "0.2"
```

To use unreleased repository changes:

```toml
[dependencies]
trellis-core = { git = "https://github.com/pablof7z/trellis", package = "trellis-core" }
```

Optional serialization support:

```toml
trellis-core = { version = "0.2", features = ["serde"] }
```

## Core rules

**Transactions are the boundary.** All mutation goes through explicit
transactions. A failed transaction commits nothing — rollback is structural,
not best-effort.

**Dependencies are declared.** Nodes, planners, and outputs state what they
read; an undeclared read is an error. The dependency graph is a reviewable
artifact, not an emergent property.

**Effects are data.** Planners return `ResourcePlan<C>`; the host owns all
I/O, retries, task spawning, and real handles. Nothing inside propagation can
touch the world.

**Scopes own lifecycle.** Nodes, resources, and outputs attach to scopes. Scope
close produces deterministic teardown commands, output clear frames, and node
reclamation.

**Empty sources fail closed.** A source that derives no targets opens no
demand. `empty -> nothing`, never `empty -> everything`.

**Outputs are revisioned frames.** Consumers see baselines, deltas, clears,
and rebaselines with transaction and revision identity — never graph
internals.

**Incremental behavior is checkable.** Every supported graph shape can be
compared against a full recompute. If the engine ever lies, you can catch it.

## Crates

- `trellis-core`: deterministic graph runtime, resource plans, output frames,
  audit queries, and the full-recompute oracle.
- `trellis-testing`: scenario scripts, replay checks, resource/output ledgers,
  fake host status helpers, audit assertions, and conformance support.
- `trellis-adapter`: runtime-neutral boundary for applying returned plans and
  emitting returned frames outside graph propagation.
- `trellis-examples`: proof examples — Workspace Sync Board, a mini language
  server, FleetPulse, and a wrapper-friendly protocol subscription
  engine.
- `trellis-bench`: benchmark smoke coverage for propagation, diffs, teardown,
  output, oracle, and replay paths.

## Examples

The examples live outside the core crate so domain vocabulary does not leak
into the runtime:

- Workspace Sync Board;
- mini language server diagnostics;
- FleetPulse telemetry dashboard;
- wrapper-friendly protocol subscription API (dynamic per-session scopes,
  shared resources, and a host command loop).

Run the workspace tests:

```bash
cargo test --workspace
```

## Documentation

Start here:

- [Guide](docs/GUIDE.md): short usage walkthrough.
- [Charter](docs/CHARTER.md): product and architecture contract.
- [Semantics](docs/SEMANTICS.md): transaction and runtime semantics.
- [Invariants](docs/INVARIANTS.md): rules mapped to tests.
- [Testing](docs/TESTING.md): oracle, replay, ledgers, and conformance support.
- [Shadow-mode adoption](docs/SHADOW_MODE.md): adopting Trellis beside an
  existing reconciliation path, with exit criteria.
- [Examples](docs/EXAMPLES.md): proof example descriptions.
- [Showcase API boundary](docs/SHOWCASE_API_BOUNDARY.md): app-owned wrappers
  that keep Trellis private while exposing domain APIs.
- [Performance model](docs/PERFORMANCE.md): the honest cost model and its
  rationale.
- [Non-goals](docs/NON_GOALS.md): what Trellis deliberately excludes.
- [Design essay](docs/DESIGN_ESSAY.md): longer rationale.

## Development

Common checks:

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

Core constraints:

- no unsafe code in the first implementation;
- no hidden global runtime;
- no external side effects during graph propagation;
- no domain-specific concepts in `trellis-core`;
- no compatibility shims before v1.0 when a cleaner shape is available.

See [AGENTS.md](AGENTS.md) and [Contributing](docs/CONTRIBUTING.md) before
opening a PR.

## License

Licensed under `MIT OR Apache-2.0`.
