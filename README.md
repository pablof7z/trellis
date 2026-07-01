# Trellis

**Deterministic reactive resource graphs for Rust application kernels.**

## North-star contract

Trellis is a deterministic reactive resource-graph runtime.

It accepts canonical input changes.
It recomputes explicit derived nodes.
It produces structural diffs.
It turns diffs into resource plans.
It emits revisioned materialized output frames.
It scopes teardown.
It never performs external side effects during graph propagation.
It makes incremental behavior checkable against full recompute.

The first version optimizes for correctness, determinism, auditability, small
API surface, testability, boring internals, and clear failure modes.

It does not optimize for ergonomics magic, macro-heavy APIs, automatic
dependency discovery, async runtime ownership, distributed execution, maximum
generality, or UI framework integration.

Trellis is a small Rust runtime for systems where changing state does more than recompute values.

It is for application kernels where derived state owns live resources, produces scoped command plans, and emits revisioned materialized output.

```text
canonical input changed
 -> derived state changed
 -> structural diff produced
 -> resource plan emitted
 -> materialized output revised
 -> full-recompute check passed
```

Trellis is not a UI framework, not an Rx implementation, not a database, and not a query cache. It is the layer between canonical application state and the live resources that state implies.

```text
Signals recompute values.
Trellis reconciles resources.
```

> Project status: early design / 0.1 preview. The examples in this README show the intended shape of the API. Names and exact signatures may change before stabilization.

> Current implementation status: `trellis-core` contains the M4 graph skeleton:
> typed identities, node handles, scopes, dependency lists, deterministic debug
> output, atomic canonical input transactions, and pure derived scalar node
> recomputation, plus typed collection nodes with deterministic structural
> diffs. It does not implement resource plans, materialized outputs, async
> behavior, or automatic dependency tracking yet.

---

## Table of contents

- [Why Trellis exists](#why-trellis-exists)
- [The problem](#the-problem)
- [The core idea](#the-core-idea)
- [When to use Trellis](#when-to-use-trellis)
- [When not to use Trellis](#when-not-to-use-trellis)
- [How Trellis differs from adjacent tools](#how-trellis-differs-from-adjacent-tools)
- [A small example](#a-small-example)
- [A more realistic example: workspace-driven sync](#a-more-realistic-example-workspace-driven-sync)
- [Core concepts](#core-concepts)
- [Execution model](#execution-model)
- [Resource plans](#resource-plans)
- [Scopes and teardown](#scopes-and-teardown)
- [Materialized output](#materialized-output)
- [Collection diffs](#collection-diffs)
- [Dynamic dependencies](#dynamic-dependencies)
- [Error handling](#error-handling)
- [Testing and verification](#testing-and-verification)
- [Examples of real application shapes](#examples-of-real-application-shapes)
- [Design principles](#design-principles)
- [Architecture](#architecture)
- [Runtime integration](#runtime-integration)
- [Performance model](#performance-model)
- [API sketch](#api-sketch)
- [Cargo features](#cargo-features)
- [Roadmap](#roadmap)
- [FAQ](#faq)
- [Contributing](#contributing)
- [License](#license)

---

## Why Trellis exists

Many applications eventually grow an implicit graph like this:

```text
current workspace
 -> visible projects
 -> live database queries
 -> network sync windows
 -> local materialized rows
 -> UI frames
```

or this:

```text
open files
 -> parsed modules
 -> import graph
 -> diagnostics
 -> editor output
 -> file watchers
```

or this:

```text
selected customer
 -> visible devices
 -> telemetry topic set
 -> broker subscriptions
 -> dashboard panels
```

At first, this graph is usually handwritten with callbacks, observers, subscription handles, invalidation counters, and reset hooks.

That works until the graph becomes dynamic:

- the active workspace changes;
- the visible project set shrinks;
- permissions are revoked;
- a file is deleted;
- a plugin is disabled;
- a dashboard panel closes;
- a query becomes empty;
- a route changes while fetches are still in flight;
- two scopes share the same live resource;
- an output surface needs a coherent rebaseline.

At that point, the difficult part is no longer “how do I recompute the derived value?”

The difficult part is:

```text
What resources did the old value own?
Which resources must be withdrawn?
Which new resources must be installed?
Which stale output must be cleared?
Which output revision is now coherent?
Can I prove the incremental result equals a full recompute?
Who owns teardown?
```

Trellis gives this pattern a small, explicit runtime.

---

## The problem

A normal reactive system often answers:

```text
When A changes, recompute B.
```

That is useful, but many Rust application kernels need a stronger contract:

```text
When canonical fact A changes:
  recompute derived source B;
  diff B against the previous B;
  withdraw external demand that is no longer valid;
  install new demand;
  update materialized output;
  emit a coherent revision;
  make the transition inspectable and testable.
```

Consider a local-first issue tracker.

```text
active_workspace
 -> permitted_project_ids
 -> sync_shape_set
 -> local replica subscriptions
 -> materialized board rows
```

If the active workspace changes, the kernel must not merely recompute `permitted_project_ids`. It must close old sync shapes, open new sync shapes, replay cached rows into the new view, clear rows that are no longer visible, and emit a revisioned board update.

If permissions are revoked, the old data windows must close. Empty permission sets should fail closed. Stale rows should not remain visible because some callback forgot to clear them.

This is **reactive resource reconciliation**.

---

## The core idea

Trellis models application state as an explicit graph.

The graph has:

- **canonical inputs** changed by one owner;
- **derived nodes** computed from declared dependencies;
- **collection nodes** that produce structural diffs;
- **resource plans** that describe what external resources to open, close, replace, cancel, or rebaseline;
- **materialized outputs** that emit baselines, deltas, clears, and revisions;
- **scopes** that own lifetimes and teardown;
- **transactions** that give each propagation cycle deterministic phase ordering;
- **verification hooks** that compare incremental propagation against full recompute.

Trellis does not perform arbitrary I/O while propagating the graph. Instead, graph propagation produces plans:

```text
Plan:
  Close(QueryShape("workspace:old:issues"))
  Open(QueryShape("workspace:new:issues"))
  Clear(Output("issue-board"))
  EmitBaseline(Output("issue-board"), Revision(42))
```

Your application actor applies those commands in a known phase.

That distinction is central.

Bad shape:

```text
A closure reran because it read a reactive value.
The closure did I/O.
The scheduler decided when it ran.
Cancellation is now another concern.
Teardown is spread across callbacks.
```

Preferred shape:

```text
A transaction propagated through explicit dependencies.
The transaction produced a command plan.
The host actor applied the plan.
The transaction emitted revisioned output.
The test harness can compare the result to full recompute.
```

Trellis is designed to make the second shape easy.

---

## When to use Trellis

Use Trellis when your application has state-derived resources.

Typical signs:

- a derived set or map controls subscriptions, queries, watchers, jobs, or output rows;
- shrinking the set has important semantics;
- an empty set should mean “nothing,” not “everything”;
- resources need scoped teardown;
- output needs coherent baselines or revisions;
- two sessions may share one resource and release it independently;
- callbacks are starting to encode hidden dependency edges;
- testing requires “incremental result equals full recompute”;
- cancellation, ordering, batching, or rebaseline are part of correctness.

Good fits:

- local-first sync engines;
- live database clients;
- offline-capable application cores;
- language-server-like systems;
- collaborative document kernels;
- telemetry dashboards;
- market-data dashboards;
- plugin runtimes;
- workflow engines;
- build/task orchestration tools;
- multi-platform Rust cores rendered by native/web shells.

Trellis is most useful below the UI layer: inside the Rust-owned domain kernel that decides what resources exist and what output surfaces are current.

---

## When not to use Trellis

Do not use Trellis for simple state propagation.

Trellis is probably too much if your problem is only:

- rerendering a component when a value changes;
- memoizing a pure calculation;
- combining a few async streams;
- sending one HTTP request when a route changes;
- maintaining local form state;
- building a small CLI with direct control flow;
- caching function results without external resource lifecycle.

For those cases, direct Rust code, a UI signal framework, a stream library, or an incremental computation crate may be simpler.

Trellis is for the cases where “state changed” means “resources must be reconciled.”

---

## How Trellis differs from adjacent tools

### UI signal libraries

Signal libraries are excellent at fine-grained value propagation:

```text
state changed -> memo recomputed -> effect reran -> view updated
```

Trellis targets a different layer:

```text
state changed -> collection diff -> resource plan -> scoped teardown -> revisioned output
```

A UI framework can render Trellis output. Trellis should not be required to render UI.

### Rx / stream libraries

Stream libraries are good at event composition:

```text
stream A + stream B -> transformed stream C
```

Trellis is about deterministic reconciliation:

```text
old desired resources vs new desired resources -> command plan
```

Streams can be useful at the edges of a Trellis application, but Trellis avoids making hidden scheduler behavior part of domain correctness.

### Query caches

Query caches are good at fetching and caching server state.

Trellis is lower-level. It can decide which queries should exist, which should close, which output surfaces they affect, and what transaction revision should be emitted.

### Incremental computation engines

Incremental computation engines are excellent for pure derived values.

Trellis borrows that spirit but adds resource lifecycle:

```text
current value + previous value -> structural diff -> resource commands
```

### Dataflow systems

Dataflow systems are powerful for high-volume collection processing.

Trellis is smaller and application-kernel-oriented. It is meant to live inside one process, under an actor or reducer, where explicit resource ownership matters more than distributed throughput.

---

## A small example

Suppose a dashboard subscribes to telemetry topics for the devices visible under the current filter.

```text
selected_site
 -> visible_device_ids
 -> telemetry_topics
 -> broker subscriptions
 -> dashboard output
```

If the selected site changes, the application must unsubscribe from topics for devices no longer visible, subscribe to topics for newly visible devices, clear stale dashboard cards, and emit a coherent revision.

With Trellis, you model that lifecycle explicitly.

```rust
use trellis::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct SiteId(String);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct DeviceId(String);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Topic(String);

#[derive(Clone, Debug)]
enum AppCommand {
    Subscribe(Topic),
    Unsubscribe(Topic),
}

#[derive(Clone, Debug)]
enum DashboardFrame {
    Clear,
    Baseline { revision: Revision, devices: Vec<DeviceId> },
}

fn build_dashboard_graph(graph: &mut Graph<AppCommand, DashboardFrame>) {
    let selected_site = graph.input::<Option<SiteId>>("selected_site");
    let device_registry = graph.input::<DeviceRegistry>("device_registry");

    let visible_devices = graph.collection::<DeviceId>("visible_devices")
        .depends_on((selected_site, device_registry))
        .derive(|ctx| {
            let Some(site) = ctx.get(selected_site) else {
                return BTreeSet::new();
            };

            ctx.get(device_registry).devices_for_site(site)
        })
        .empty_means_empty();

    let telemetry_topics = graph.collection::<Topic>("telemetry_topics")
        .depends_on(visible_devices)
        .derive(|ctx| {
            ctx.get(visible_devices)
                .iter()
                .map(|device| Topic(format!("telemetry/{}/state", device.0)))
                .collect()
        })
        .empty_means_empty();

    graph.resource_plan("telemetry_subscriptions")
        .from_diff(telemetry_topics.diff())
        .plan(|diff, plan| {
            for topic in diff.removed() {
                plan.command(AppCommand::Unsubscribe(topic.clone()));
            }

            for topic in diff.added() {
                plan.command(AppCommand::Subscribe(topic.clone()));
            }
        });

    graph.materialized_output("dashboard")
        .depends_on(visible_devices)
        .emit(|ctx, out| {
            out.clear();
            out.baseline(DashboardFrame::Baseline {
                revision: ctx.revision(),
                devices: ctx.get(visible_devices).iter().cloned().collect(),
            });
        });
}
```

The important part is not the syntax. The important part is the lifecycle:

```text
site changed
 -> visible device set changed
 -> topic set diffed
 -> obsolete subscriptions closed
 -> new subscriptions opened
 -> dashboard cleared/rebaselined
 -> revision emitted
```

No wildcard behavior is implied by an empty device set. No subscription is left open merely because a callback forgot to close it.

---

## A more realistic example: workspace-driven sync

A local-first app often has a partial replica. The client should sync only the data relevant to the current workspace, route, permissions, and visible screen.

```text
active_workspace
 -> permission_snapshot
 -> visible_project_ids
 -> sync_shape_set
 -> local rows
 -> materialized screen output
```

### Domain types

```rust
use trellis::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct WorkspaceId(String);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct ProjectId(String);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct ShapeId(String);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct SyncShape {
    id: ShapeId,
    table: &'static str,
    predicate: String,
}

#[derive(Clone, Debug)]
struct PermissionSnapshot {
    projects_by_workspace: BTreeMap<WorkspaceId, BTreeSet<ProjectId>>,
}

#[derive(Clone, Debug)]
struct LocalStoreSnapshot {
    issues_by_project: BTreeMap<ProjectId, Vec<IssueRow>>,
}

#[derive(Clone, Debug)]
struct IssueRow {
    id: String,
    title: String,
    project_id: ProjectId,
}

#[derive(Clone, Debug)]
enum SyncCommand {
    OpenShape(SyncShape),
    CloseShape(SyncShape),
}

#[derive(Clone, Debug)]
enum ScreenFrame {
    Clear,
    Baseline {
        revision: Revision,
        rows: Vec<IssueRow>,
    },
    Delta {
        revision: Revision,
        inserted: Vec<IssueRow>,
        removed_ids: Vec<String>,
    },
}
```

### Graph construction

```rust
fn build_workspace_graph(graph: &mut Graph<SyncCommand, ScreenFrame>) {
    let active_workspace = graph.input::<Option<WorkspaceId>>("active_workspace");
    let permissions = graph.input::<PermissionSnapshot>("permissions");
    let local_store = graph.input::<LocalStoreSnapshot>("local_store");

    let visible_projects = graph.collection::<ProjectId>("visible_projects")
        .depends_on((active_workspace, permissions))
        .derive(|ctx| {
            let Some(workspace) = ctx.get(active_workspace) else {
                return BTreeSet::new();
            };

            ctx.get(permissions)
                .projects_by_workspace
                .get(workspace)
                .cloned()
                .unwrap_or_default()
        })
        .empty_means_empty();

    let sync_shapes = graph.collection::<SyncShape>("sync_shapes")
        .depends_on(visible_projects)
        .derive(|ctx| {
            ctx.get(visible_projects)
                .iter()
                .map(|project_id| SyncShape {
                    id: ShapeId(format!("issues:{}", project_id.0)),
                    table: "issues",
                    predicate: format!("project_id = '{}'", project_id.0),
                })
                .collect()
        })
        .empty_means_empty();

    graph.resource_plan("sync_shapes")
        .from_diff(sync_shapes.diff())
        .plan(|diff, plan| {
            for shape in diff.removed() {
                plan.command(SyncCommand::CloseShape(shape.clone()));
            }

            for shape in diff.added() {
                plan.command(SyncCommand::OpenShape(shape.clone()));
            }
        });

    graph.materialized_output("issue_board")
        .depends_on((visible_projects, local_store))
        .emit(|ctx, out| {
            let projects = ctx.get(visible_projects);
            let store = ctx.get(local_store);

            let mut rows = Vec::new();
            for project_id in projects {
                if let Some(project_rows) = store.issues_by_project.get(project_id) {
                    rows.extend(project_rows.iter().cloned());
                }
            }

            out.baseline(ScreenFrame::Baseline {
                revision: ctx.revision(),
                rows,
            });
        });
}
```

### Runtime usage

```rust
fn on_workspace_selected(
    graph: &mut Graph<SyncCommand, ScreenFrame>,
    active_workspace: InputNode<Option<WorkspaceId>>,
    workspace: Option<WorkspaceId>,
    runtime: &mut Runtime,
    screen: &mut ScreenPort,
) {
    let tx = graph.transaction(|tx| {
        tx.set(active_workspace, workspace);
    });

    // Apply resource commands in the host runtime.
    for command in tx.commands() {
        runtime.apply(command);
    }

    // Send materialized output after command planning.
    for frame in tx.frames() {
        screen.send(frame);
    }
}
```

### What this buys you

When `active_workspace` changes:

```text
old visible projects are diffed against new visible projects;
old sync shapes are closed;
new sync shapes are opened;
visible rows are rederived from local canonical state;
screen output receives a coherent revision.
```

When permissions are revoked:

```text
revoked projects disappear from visible_projects;
corresponding sync shapes close;
stale rows are omitted from the next baseline;
full-recompute tests can assert that no unauthorized row remains visible.
```

When no workspace is selected:

```text
visible_projects = empty;
sync_shapes = empty;
no sync demand is opened;
output can clear or emit an empty baseline.
```

The empty case is explicit. Empty does not mean “all workspaces.” Empty means no demand.

---

## Core concepts

### `Graph`

A `Graph` owns node definitions, cached node values, dependency edges, dirty state, resource plan definitions, output definitions, and the current revision.

A graph is usually owned by one actor, reducer, or application kernel.

Trellis does not require global mutable state.

```rust
let mut graph = Graph::<AppCommand, AppFrame>::new();
```

The two type parameters are commonly:

```rust
Graph<Command, Frame>
```

where:

- `Command` is the host application’s resource command type;
- `Frame` is the host application’s materialized output frame type.

Trellis does not know what those enums mean.

---

### `InputNode<T>`

An input node represents canonical state changed by the host application.

Examples:

- active workspace;
- current route;
- permission snapshot;
- local store revision;
- open file contents;
- selected dashboard filters;
- plugin registry;
- feature flags;
- time window;
- viewport range.

```rust
let active_workspace = graph.input::<Option<WorkspaceId>>("active_workspace");
```

Input nodes should be owned by one writer. Trellis is designed for deterministic propagation from explicit input changes.

---

### `DerivedNode<T>`

A derived node computes a value from declared dependencies.

```rust
let active_team = graph.derive::<Option<TeamId>>("active_team")
    .depends_on((active_workspace, permissions))
    .compute(|ctx| {
        let workspace = ctx.get(active_workspace)?;
        ctx.get(permissions).team_for_workspace(workspace)
    });
```

Derived nodes should be deterministic. Given the same dependency values, they should return the same value.

Trellis can skip propagation when the derived value does not change according to its equality policy.

---

### `CollectionNode<K>` / `CollectionNode<K, V>`

A collection node represents a derived set or map. It is first-class because many resource lifecycles are collection-shaped.

```rust
let visible_projects = graph.collection::<ProjectId>("visible_projects")
    .depends_on((active_workspace, permissions))
    .derive(|ctx| derive_visible_projects(ctx));
```

A collection node can produce a structural diff:

```rust
CollectionDiff<K> {
    added: BTreeSet<K>,
    removed: BTreeSet<K>,
    retained: BTreeSet<K>,
}
```

For maps:

```rust
MapDiff<K, V> {
    added: BTreeMap<K, V>,
    removed: BTreeMap<K, V>,
    updated: BTreeMap<K, ValueChange<V>>,
    retained: BTreeMap<K, V>,
}
```

Resource plans usually consume collection diffs.

---

### `ResourcePlan`

A resource plan turns graph changes into application commands.

The plan does not perform I/O itself.

```rust
graph.resource_plan("query_subscriptions")
    .from_diff(query_shapes.diff())
    .plan(|diff, plan| {
        for removed in diff.removed() {
            plan.command(Command::CloseQuery(removed.clone()));
        }

        for added in diff.added() {
            plan.command(Command::OpenQuery(added.clone()));
        }
    });
```

The host application decides what `Command::OpenQuery` and `Command::CloseQuery` do.

This keeps graph propagation deterministic and testable.

---

### `MaterializedOutput`

A materialized output is an output surface owned by the graph.

Examples:

- screen rows;
- diagnostics;
- dashboard panels;
- search results;
- command palette entries;
- sync status;
- task progress;
- editor tokens;
- plugin contributions.

Outputs are revisioned. A consumer can tell whether a frame belongs to the current graph state.

```rust
graph.materialized_output("diagnostics")
    .depends_on(diagnostic_set)
    .emit(|ctx, out| {
        out.baseline(Frame::Diagnostics {
            revision: ctx.revision(),
            items: ctx.get(diagnostic_set).to_vec(),
        });
    });
```

Trellis supports output policies such as:

- `baseline`: emit the full current output;
- `delta`: emit structural changes;
- `clear`: clear the output surface;
- `rebaseline`: reset and emit the current truth;
- `omit`: intentionally emit no frame for this transaction.

---

### `Scope`

A scope owns lifetimes.

Examples:

- one screen;
- one tab;
- one document session;
- one dashboard panel;
- one plugin instance;
- one live query handle;
- one background task group.

```rust
let scope = graph.scope("workspace_screen");

scope.attach(sync_shapes);
scope.attach(issue_board_output);
```

Closing the scope tears down resources and outputs owned by that scope.

```rust
let tx = graph.close_scope(scope);

for command in tx.commands() {
    runtime.apply(command);
}

for frame in tx.frames() {
    screen.send(frame);
}
```

A scope is the answer to: “Who owns this live demand?”

---

### `Transaction`

A transaction is one propagation cycle.

A transaction starts with one or more input mutations and ends with planned commands and output frames.

```rust
let tx = graph.transaction(|tx| {
    tx.set(active_workspace, Some(workspace));
    tx.set(route_params, params);
});

runtime.apply_all(tx.commands());
ui.send_all(tx.frames());
```

Transactions make batching explicit.

```text
multiple input changes
 -> one graph propagation
 -> one command batch
 -> one output batch
```

---

### `Revision`

Every committed transaction advances the graph revision.

Revisions allow downstream consumers to identify stale frames, rebaseline boundaries, and coherent snapshots.

```rust
Frame::Baseline {
    revision: ctx.revision(),
    rows,
}
```

A revision is not a wall-clock timestamp. It is a graph-state version.

---

### `FullRecomputeOracle`

A full-recompute oracle is a test hook.

It recomputes the expected current state from canonical inputs and compares it to the incremental graph state.

```rust
graph.assert_incremental_equals_full_recompute();
```

This is useful for property tests:

```rust
proptest! {
    #[test]
    fn incremental_matches_full_recompute(actions in arbitrary_action_sequence()) {
        let mut graph = test_graph();
        let mut oracle = FullRecomputeModel::new();

        for action in actions {
            graph.apply(action.clone());
            oracle.apply(action);

            graph.assert_matches(&oracle);
        }
    }
}
```

The point is to test the graph contract, not just individual callbacks.

---

## Execution model

Trellis is designed around deterministic phase ordering.

A typical transaction has these phases:

```text
1. accept input mutations
2. mark dependent nodes dirty
3. recompute derived values
4. compute collection diffs
5. build resource plans
6. materialize output frames
7. commit revision
8. return command/frame batch to host
```

The graph does not perform external I/O during phases 1-7.

The host runtime applies returned commands after propagation.

```text
Trellis: compute what should happen.
Host: make it happen.
```

This makes propagation:

- deterministic;
- inspectable;
- replayable;
- property-testable;
- independent of async runtime choice.

### Single-writer assumption

Trellis works best when one actor owns the graph.

External systems send events to that actor:

```text
network event -> actor mailbox -> graph input mutation
file event    -> actor mailbox -> graph input mutation
timer event   -> actor mailbox -> graph input mutation
UI action     -> actor mailbox -> graph input mutation
```

The actor applies one transaction at a time.

This avoids hidden interleavings and makes graph revisions meaningful.

### Async boundary

Trellis does not need to own async execution.

A command returned by the graph might start an async task:

```rust
match command {
    Command::OpenQuery(shape) => runtime.spawn_query(shape),
    Command::CloseQuery(id) => runtime.cancel_query(id),
}
```

Results from async tasks come back as new canonical inputs or events:

```rust
GraphEvent::QueryRowsArrived { shape_id, rows }
GraphEvent::QueryClosed { shape_id }
GraphEvent::TaskFailed { task_id, error }
```

The graph remains the place where state transitions are reconciled.

---

## Resource plans

Resource plans are the core difference between Trellis and ordinary value reactivity.

A resource plan is a deterministic description of external lifecycle changes.

Examples:

```rust
enum Command {
    OpenQuery(QueryShape),
    CloseQuery(QueryShape),
    StartFileWatcher(PathBuf),
    StopFileWatcher(PathBuf),
    SpawnTask(TaskSpec),
    CancelTask(TaskId),
    SubscribeTopic(Topic),
    UnsubscribeTopic(Topic),
    LoadAsset(AssetId),
    ReleaseAsset(AssetId),
}
```

Trellis does not know these commands. The application defines them.

### Plans are not effects

A Trellis plan should be pure or near-pure. It should not perform network I/O, file I/O, database writes, or spawn tasks directly.

Instead:

```rust
plan.command(Command::SubscribeTopic(topic));
```

The host actor applies the command later.

### Why plans are better than arbitrary effect closures

Plans are:

- loggable;
- deduplicatable;
- testable;
- replayable;
- inspectable;
- batchable;
- ordered by the graph transaction;
- easier to audit for teardown bugs.

Effect closures hide too much:

```text
What did this closure open?
Who closes it?
Did it run before or after the output cleared?
What happens if it runs twice?
What happens if the dependency becomes empty?
```

Plans make those questions explicit.

### Plan ordering

A resource plan can impose an ordering policy.

Common policies:

```text
CloseThenOpen
OpenThenClose
ReplaceAtomically
CancelThenSpawn
ClearThenBaseline
```

Example:

```rust
graph.resource_plan("device_topics")
    .from_diff(topic_set.diff())
    .ordering(OrderingPolicy::CloseThenOpen)
    .plan(|diff, plan| {
        for topic in diff.removed() {
            plan.command(Command::UnsubscribeTopic(topic.clone()));
        }

        for topic in diff.added() {
            plan.command(Command::SubscribeTopic(topic.clone()));
        }
    });
```

### Empty-source behavior

Every collection that drives resource demand should define empty behavior.

```rust
let topics = graph.collection::<Topic>("topics")
    .depends_on(visible_devices)
    .derive(derive_topics)
    .empty_means_empty();
```

This is a safety feature.

The graph should distinguish:

```text
empty source -> no demand
```

from:

```text
missing source -> not ready / invalid / error
```

and should never silently interpret empty as broad demand.

---

## Scopes and teardown

Scopes are how Trellis prevents resource leaks.

A scope is a lifetime owner.

```rust
let tab = graph.scope("tab:123");
```

Resources and outputs can be attached to the scope:

```rust
tab.attach(query_shapes);
tab.attach(search_results_output);
```

When the scope closes:

```rust
let tx = graph.close_scope(tab);
```

Trellis computes the teardown plan:

```text
CloseQuery(shape:a)
CloseQuery(shape:b)
CancelTask(task:c)
ClearOutput(search_results)
Commit revision
```

### Shared resources

Two scopes may require the same resource.

```text
panel A needs topic /devices/42
panel B needs topic /devices/42
```

Trellis can model this as reference ownership:

```text
Open topic when first owner appears.
Keep topic open while at least one owner remains.
Close topic when last owner disappears.
```

This avoids duplicate subscriptions and premature teardown.

### Hierarchical scopes

Scopes can be hierarchical.

```text
workspace screen
  issue board panel
  activity panel
  search panel
```

Closing the parent closes children.

```rust
let workspace = graph.scope("workspace");
let board = workspace.child("board");
let search = workspace.child("search");

let tx = graph.close_scope(workspace);
```

The teardown plan covers all child-owned resources.

### Scope identity

Scope identity should be stable and domain-meaningful.

Good:

```text
workspace_screen:acme
editor_document:/src/lib.rs
plugin:spellcheck
panel:telemetry:site-12
```

Poor:

```text
scope_1
scope_2
thing
```

Inspectability is a design goal.

---

## Materialized output

A materialized output is the graph-owned representation of what an external consumer should see.

Examples:

```text
issue board rows
diagnostics for an open file
visible devices in a dashboard
search result page
plugin command registry
asset loading status
sync status
```

Outputs are not side effects. They are part of the transaction result.

```rust
let tx = graph.transaction(|tx| {
    tx.set(active_workspace, Some(workspace));
});

for frame in tx.frames() {
    output_port.send(frame);
}
```

### Baselines

A baseline is the complete current output for a surface.

```rust
Frame::Baseline {
    revision: Revision(42),
    rows: vec![...],
}
```

Baselines are useful when:

- a scope opens;
- a source changes shape;
- a consumer reconnects;
- the graph revalidates after an error;
- a delta sequence was missed.

### Deltas

A delta is a structural change from the previous revision.

```rust
Frame::Delta {
    revision: Revision(43),
    inserted: vec![...],
    updated: vec![...],
    removed: vec![...],
}
```

Deltas are useful when output surfaces are large and changes are small.

### Clears

A clear tells the consumer that previous output is no longer valid.

```rust
Frame::Clear {
    revision: Revision(44),
    output: OutputId::new("issue_board"),
}
```

A clear is not an error. It is a coherent state transition.

### Rebaseline

A rebaseline combines clear + baseline semantics.

```text
old output is invalid;
here is the current truth at revision R.
```

Rebaseline is useful after:

- route changes;
- permission changes;
- schema changes;
- missed deltas;
- recovery from an external error;
- consumer reconnect.

### Omission

Sometimes the correct output for a transaction is no output.

Example:

```text
local cache warmed but visible rows unchanged
```

Trellis should allow explicit omission so “no frame” is not confused with “forgot to emit.”

---

## Collection diffs

Many resource problems are set/map problems.

```text
old set: {A, B, C}
new set: {B, C, D}

diff:
  removed: {A}
  added:   {D}
  retained:{B, C}
```

A scalar invalidation only says:

```text
something changed
```

A collection diff says:

```text
close resource for A
open resource for D
leave B and C alone
```

This is the difference between noisy reactivity and precise reconciliation.

### Set diffs

```rust
pub struct SetDiff<K> {
    pub added: BTreeSet<K>,
    pub removed: BTreeSet<K>,
    pub retained: BTreeSet<K>,
}
```

### Map diffs

```rust
pub struct MapDiff<K, V> {
    pub added: BTreeMap<K, V>,
    pub removed: BTreeMap<K, V>,
    pub updated: BTreeMap<K, ValueChange<V>>,
    pub retained: BTreeMap<K, V>,
}
```

### Diff policies

Different collections may need different equality policies.

```rust
let diagnostics = graph.map::<FileId, DiagnosticSet>("diagnostics")
    .equality(DiagnosticSet::semantic_eq);
```

Examples:

- byte-for-byte equality;
- semantic equality;
- version-only equality;
- key-only equality;
- custom diff.

### Derived empty sets

A derived set can be empty for several reasons:

```text
there are no matching items;
permissions allow nothing;
the source is not ready;
the source errored;
the scope is closing.
```

Do not collapse these into one ambiguous state.

Prefer explicit source states:

```rust
enum SourceState<T> {
    Ready(T),
    Empty,
    NotReady,
    Error(SourceError),
    Closing,
}
```

A resource-driving collection should define behavior for each state.

---

## Dynamic dependencies

Some dependencies are known only after reading data.

Examples:

```text
open document -> referenced subdocuments
selected project -> linked repositories
visible dashboard -> panels -> panel queries
plugin manifest -> contributed watchers
route params -> data dependencies
```

Trellis supports dynamic dependencies, but they should remain inspectable.

A dynamic node should be able to report:

```text
current dependency keys
current resource owners
current output surfaces
current empty/error behavior
```

Example:

```rust
let referenced_docs = graph.collection::<DocumentId>("referenced_docs")
    .depends_on(open_document)
    .derive(|ctx| extract_document_refs(ctx.get(open_document)))
    .inspectable();

let doc_sync = graph.resource_plan("document_sync")
    .from_diff(referenced_docs.diff())
    .plan(|diff, plan| {
        for id in diff.removed() {
            plan.command(Command::CloseDocumentSync(id.clone()));
        }

        for id in diff.added() {
            plan.command(Command::OpenDocumentSync(id.clone()));
        }
    });
```

Dynamic dependencies are useful. Hidden dependencies are not.

---

## Error handling

Errors are part of the graph state.

Trellis distinguishes:

```text
empty
not ready
error
stale
closing
ready
```

These states should not be collapsed.

### Resource errors

Resource errors should come back into the graph as events or input updates.

```rust
enum RuntimeEvent {
    QueryOpened(QueryId),
    QueryFailed(QueryId, QueryError),
    QueryRowsArrived(QueryId, Vec<Row>),
    QueryClosed(QueryId),
}
```

The graph can then derive output:

```text
query failed -> sync status output updated
query failed -> dependent output marked stale
query failed -> retry plan emitted if policy allows
```

### Partial failure

Some resources may fail while others remain valid.

Example:

```text
3 dashboard panels
  panel A: ready
  panel B: stale
  panel C: failed
```

Trellis should allow outputs to represent partial states without invalidating the whole graph.

### Retry policy

Retry policy belongs to the application.

Trellis can plan a retry command if the application encodes that policy:

```rust
plan.command(Command::RetryQuery {
    query_id,
    backoff: Backoff::exponential(attempt),
});
```

Trellis should not hide retry behavior in its scheduler.

---

## Testing and verification

Trellis is designed to make lifecycle bugs testable.

### Generic invariants

A Trellis graph should support tests like:

```text
incremental result equals full recompute;
source shrink withdraws resources;
empty source opens no broad demand;
scope close releases owned resources;
shared resources close only after last owner leaves;
output revision is monotonic;
clear/rebaseline emits coherent frames;
no command is emitted outside a transaction;
command ordering follows the resource policy;
all dynamic dependencies are inspectable.
```

### Full recompute

Full recompute is the oracle.

For any sequence of input changes, the graph should end in the same state as rebuilding from canonical inputs.

```rust
#[test]
fn workspace_switch_matches_full_recompute() {
    let mut graph = workspace_graph();
    let mut oracle = WorkspaceOracle::default();

    let actions = vec![
        Action::SelectWorkspace("acme"),
        Action::GrantProject("p1"),
        Action::GrantProject("p2"),
        Action::RevokeProject("p1"),
        Action::SelectWorkspace("globex"),
        Action::ClearWorkspace,
    ];

    for action in actions {
        graph.apply(action.clone());
        oracle.apply(action);

        assert_eq!(graph.visible_state(), oracle.full_recompute());
    }
}
```

### Resource lifecycle tests

```rust
#[test]
fn revoked_project_closes_shape() {
    let mut graph = workspace_graph();

    graph.apply(Action::SelectWorkspace("acme"));
    graph.apply(Action::GrantProject("p1"));

    let tx = graph.apply(Action::RevokeProject("p1"));

    assert!(tx.commands().contains(&Command::CloseShape(shape("p1"))));
    assert!(!graph.active_resources().contains(&shape("p1")));
}
```

### Empty-source tests

```rust
#[test]
fn empty_permission_set_opens_no_demand() {
    let mut graph = workspace_graph();

    let tx = graph.apply(Action::SetPermissions(PermissionSnapshot::empty()));

    assert!(tx.commands().iter().all(|cmd| !cmd.opens_broad_demand()));
    assert!(graph.active_resources().is_empty());
}
```

### Scope teardown tests

```rust
#[test]
fn closing_scope_releases_owned_resources() {
    let mut graph = dashboard_graph();
    let panel = graph.open_scope("panel:temperature");

    graph.apply(Action::PanelNeedsTopic(panel, topic("devices/42/temp")));
    assert!(graph.active_resources().contains(&topic("devices/42/temp")));

    let tx = graph.close_scope(panel);

    assert!(tx.commands().contains(&Command::UnsubscribeTopic(topic("devices/42/temp"))));
    assert!(!graph.active_resources().contains(&topic("devices/42/temp")));
}
```

### Property testing

Trellis is a good fit for property testing because the graph contract is stateful and sequence-sensitive.

Interesting generated actions:

```text
open scope
close scope
change route
grant permission
revoke permission
insert local row
delete local row
external resource succeeds
external resource fails
consumer reconnects
force rebaseline
```

Properties:

```text
no leaked resources;
no stale unauthorized output;
revision is monotonic;
incremental equals full recompute;
empty means empty;
shared resources are reference-counted correctly.
```

---

## Examples of real application shapes

This section describes realistic problem sets Trellis is meant to cover.

### Local-first sync

```text
active workspace
 -> visible records
 -> sync windows
 -> local replica
 -> materialized screen output
```

When the workspace changes:

```text
close old sync windows;
open new sync windows;
replay local cache;
clear stale output;
emit new revision.
```

### Permission-aware data access

```text
current user
 -> role grants
 -> accessible project ids
 -> live queries
 -> visible rows
```

When access is revoked:

```text
close unauthorized queries;
clear unauthorized rows;
fail closed if no access remains;
rebaseline output.
```

### Language-server-like analysis

```text
open files
 -> parsed modules
 -> import graph
 -> diagnostics
 -> editor frames
```

When a file changes:

```text
invalidate affected modules;
cancel obsolete analysis tasks;
update file watchers;
clear diagnostics for deleted files;
emit diagnostics revision.
```

### Telemetry dashboard

```text
selected customer
 -> visible devices
 -> telemetry topics
 -> broker subscriptions
 -> dashboard panels
```

When the filter shrinks:

```text
unsubscribe from removed topics;
keep retained topics;
subscribe to added topics;
clear stale device cards;
emit dashboard revision.
```

### Market-data dashboard

```text
watchlist
 -> entitled symbols
 -> quote/orderbook subscriptions
 -> grid and chart output
```

When a symbol is removed:

```text
unsubscribe if no remaining scope needs it;
clear or mark stale row;
release chart resources;
emit revisioned output.
```

### Collaborative editor

```text
open document
 -> referenced subdocuments
 -> comments
 -> presence rooms
 -> attachment hydration
 -> editor output
```

When the document changes:

```text
open sync for newly referenced subdocuments;
close sync for removed subdocuments;
clear removed blocks;
materialize new editor state.
```

### Plugin runtime

```text
enabled plugins
 -> contributed commands
 -> file watchers
 -> background workers
 -> panels
```

When a plugin is disabled:

```text
stop workers;
close watchers;
remove commands;
clear panels;
release owned resources.
```

### Build/task orchestration

```text
selected target
 -> dependency graph
 -> watched paths
 -> build jobs
 -> artifact output
```

When dependencies change:

```text
update watchers;
cancel obsolete jobs;
spawn new jobs;
clear stale artifacts;
emit build-status revision.
```

### Asset streaming

```text
camera viewport
 -> visible world chunks
 -> required assets
 -> streaming jobs
 -> GPU resources
 -> scene output
```

When the viewport changes:

```text
release far assets;
load near assets;
cancel obsolete jobs;
rebaseline visible scene output.
```

### Search/index application

```text
query params
 -> candidate shards
 -> index readers
 -> ranking jobs
 -> result page
```

When the corpus changes:

```text
close old shard readers;
open new shard readers;
cancel ranking jobs;
clear stale results;
emit search revision.
```

---

## Design principles

### 1. Explicit dependency identity

Dependencies should be inspectable.

You should be able to ask:

```text
What canonical inputs affect this output?
What resources does this scope own?
What outputs can this node clear?
What happens when this source becomes empty?
What is the full recompute equivalent?
```

Automatic dependency discovery can be convenient, but hidden dependency edges are dangerous when they control external resources.

Trellis is explicit-dependency-first.

### 2. Plans over side effects

Graph propagation should produce plans, not perform arbitrary I/O.

```text
derive -> diff -> plan -> return commands
```

The host runtime applies commands.

### 3. Collection diffs over broad invalidation

Resource lifecycles are often collection-shaped. Trellis should know what was added and removed.

```text
removed -> close
added -> open
retained -> leave alone
updated -> replace or patch
```

### 4. Scoped lifecycle

Every live resource should have an owner.

If the owner closes, the resource must close unless another owner still needs it.

### 5. Fail-closed empty sources

A source that derives no targets should open no broad demand.

```text
empty -> nothing
```

not:

```text
empty -> everything
```

### 6. Deterministic phase ordering

Transactions should have stable ordering so output and resource commands are coherent.

### 7. Revisioned output

Consumers should be able to identify current output, stale output, baselines, deltas, clears, and rebaselines.

### 8. Full-recompute verification

Incremental systems drift unless tested. Trellis should make it practical to compare incremental state against a simple full recompute model.

### 9. Runtime independence

Trellis should not require a specific async runtime, UI framework, database, or transport.

### 10. Domain neutrality

Trellis should not know your domain. It should provide graph, diff, plan, scope, transaction, and output primitives.

---

## Architecture

A Trellis application usually has this shape:

```text
+------------------+
| external events  |
| UI actions       |
| runtime results  |
| timers           |
+--------+---------+
         |
         v
+------------------+
| host actor       |
| single writer    |
+--------+---------+
         |
         v
+------------------+
| Trellis graph    |
| inputs           |
| derived nodes    |
| collection diffs |
| resource plans   |
| outputs          |
+--------+---------+
         |
         +------------------+
         |                  |
         v                  v
+------------------+   +------------------+
| command batch    |   | output frames    |
| open/close/cancel|   | baseline/delta   |
+--------+---------+   +--------+---------+
         |                  |
         v                  v
+------------------+   +------------------+
| runtime systems  |   | consumers        |
| network/db/fs/etc|   | UI/API/log/etc   |
+------------------+   +------------------+
```

The graph is not a global scheduler. It is owned by the host actor.

### Suggested crate layout

```text
src/
  lib.rs
  graph.rs
  node.rs
  input.rs
  derive.rs
  collection.rs
  diff.rs
  plan.rs
  scope.rs
  transaction.rs
  output.rs
  revision.rs
  oracle.rs
  inspect.rs
  error.rs
  prelude.rs
```

### Optional integration crates

The core crate should stay small. Runtime-specific integrations can live separately.

```text
trellis-core
trellis-tokio
trellis-async-std
trellis-tracing
trellis-test
trellis-inspect
trellis-serde
```

---

## Runtime integration

Trellis is runtime-neutral.

A typical integration has four parts.

### 1. Convert external events into input mutations

```rust
match event {
    Event::RouteChanged(route) => {
        graph.transaction(|tx| tx.set(route_node, route));
    }
    Event::RowsArrived { shape, rows } => {
        graph.transaction(|tx| tx.update(local_store_node, |store| {
            store.insert_rows(shape, rows);
        }));
    }
}
```

### 2. Apply command plans

```rust
for command in tx.commands() {
    match command {
        Command::OpenQuery(shape) => runtime.open_query(shape),
        Command::CloseQuery(shape) => runtime.close_query(shape),
        Command::StartWatcher(path) => runtime.start_watcher(path),
        Command::StopWatcher(path) => runtime.stop_watcher(path),
    }
}
```

### 3. Deliver output frames

```rust
for frame in tx.frames() {
    output_port.send(frame);
}
```

### 4. Feed runtime results back into the graph

```rust
runtime.on_query_rows(|shape, rows| {
    actor.send(Event::RowsArrived { shape, rows });
});
```

The graph remains deterministic because runtime results are processed as explicit events.

---

## Performance model

Trellis is intended for application kernels, not high-throughput distributed dataflow.

Design priorities:

```text
precise invalidation over broad wakes;
collection diffs over full replacement;
low allocation in hot propagation paths;
stable node identity;
explicit batching;
shared-resource deduplication;
fast no-op transactions;
inspectable dependency graph;
optional tracing rather than mandatory logging.
```

### Expected costs

A transaction cost is roughly:

```text
changed inputs
+ dirty derived nodes
+ changed collection diffs
+ resource plan construction
+ output materialization
```

Trellis should not recompute unrelated subgraphs.

### Equality gating

Derived nodes can avoid downstream propagation when their semantic value did not change.

```rust
let route_context = graph.derive::<RouteContext>("route_context")
    .depends_on(route)
    .equality(RouteContext::semantic_eq)
    .compute(derive_route_context);
```

### Batching

Multiple input mutations can be committed in one transaction.

```rust
let tx = graph.transaction(|tx| {
    tx.set(active_workspace, workspace);
    tx.set(route_params, params);
    tx.set(permission_snapshot, permissions);
});
```

This avoids emitting intermediate frames that no consumer should see.

### Shared resources

Trellis should avoid duplicate resources when multiple scopes require the same key.

```text
scope A requires query X
scope B requires query X
 -> one OpenQuery(X)

scope A closes
 -> query X remains open

scope B closes
 -> CloseQuery(X)
```

---

## API sketch

The exact API is not final. This section shows the intended shape.

### Basic graph

```rust
use trellis::prelude::*;

let mut graph = Graph::<Command, Frame>::new();

let route = graph.input::<Route>("route");
let permissions = graph.input::<Permissions>("permissions");
let local_store = graph.input::<LocalStore>("local_store");
```

### Derived value

```rust
let route_context = graph.derive::<RouteContext>("route_context")
    .depends_on((route, permissions))
    .compute(|ctx| {
        RouteContext::from_route_and_permissions(
            ctx.get(route),
            ctx.get(permissions),
        )
    });
```

### Derived collection

```rust
let visible_records = graph.collection::<RecordId>("visible_records")
    .depends_on(route_context)
    .derive(|ctx| derive_visible_records(ctx.get(route_context)))
    .empty_means_empty();
```

### Resource plan

```rust
graph.resource_plan("record_queries")
    .from_diff(visible_records.diff())
    .plan(|diff, plan| {
        for id in diff.removed() {
            plan.command(Command::CloseRecordQuery(id.clone()));
        }

        for id in diff.added() {
            plan.command(Command::OpenRecordQuery(id.clone()));
        }
    });
```

### Materialized output

```rust
graph.materialized_output("record_list")
    .depends_on((visible_records, local_store))
    .emit(|ctx, out| {
        let rows = materialize_rows(
            ctx.get(visible_records),
            ctx.get(local_store),
        );

        out.baseline(Frame::RecordListBaseline {
            revision: ctx.revision(),
            rows,
        });
    });
```

### Transaction

```rust
let tx = graph.transaction(|tx| {
    tx.set(route, Route::workspace("acme"));
});

runtime.apply_all(tx.commands());
consumer.send_all(tx.frames());
```

### Scope

```rust
let screen = graph.scope("screen:workspace:acme");

screen.attach(visible_records);
screen.attach_output("record_list");

let tx = graph.close_scope(screen);
```

### Inspection

```rust
let report = graph.inspect(scope);

println!("inputs: {:#?}", report.canonical_inputs);
println!("resources: {:#?}", report.owned_resources);
println!("outputs: {:#?}", report.outputs);
```

### Test oracle

```rust
graph.register_oracle("record_list", |inputs| {
    full_recompute_record_list(inputs)
});

graph.assert_incremental_equals_full_recompute();
```

---

## Cargo features

Proposed features:

```toml
[dependencies]
trellis = { version = "0.1", features = ["std"] }
```

Feature flags:

```text
std          enables standard library types
alloc        enables allocation without std
serde        derives serialization for graph inspection types
tracing      emits tracing spans for transactions and plans
test         enables oracle and property-test helpers
inspect      enables graph inspection reports
tokio        optional helpers for Tokio command application
```

The core graph should not require a specific async runtime.

---

## Roadmap

### 0.1

Focus: prove the core model.

- explicit input nodes;
- derived nodes;
- collection nodes;
- set/map diffs;
- resource plans;
- scopes;
- transaction batches;
- revisioned output;
- graph inspection;
- full-recompute test hooks;
- examples for sync, diagnostics, telemetry, and plugins.

### 0.2

Focus: ergonomics and testing.

- better builder API;
- procedural macro experiments for node definitions;
- property-test helpers;
- command-plan assertions;
- snapshot testing for output frames;
- shared-resource ownership policies;
- improved tracing output.

### 0.3

Focus: runtime integration.

- optional Tokio helpers;
- cancellation adapter patterns;
- output-port adapters;
- typed command executors;
- graph event loop examples.

### 0.4

Focus: dynamic dependencies.

- inspectable dynamic dependencies;
- dynamic scope trees;
- dependency report snapshots;
- runtime warnings for hidden broad invalidation.

### 1.0

A 1.0 release should require:

- stable core API;
- documented transaction semantics;
- well-tested teardown behavior;
- no known resource-leak class in scoped ownership;
- production-sized example;
- clear comparison with adjacent libraries;
- fuzz/property tests for graph invariants.

---

## FAQ

### Is Trellis a UI framework?

No.

Trellis can feed UI frameworks, but it does not render UI and does not own component lifecycles.

It is meant for the Rust application kernel below UI.

### Is Trellis a signal library?

Not in the usual sense.

Trellis has input and derived nodes, but its center is not automatic view invalidation. Its center is resource reconciliation and revisioned output.

### Does Trellis run effects?

Trellis builds plans.

Your host runtime applies the resulting commands.

This keeps graph propagation deterministic and independent of I/O scheduling.

### Does Trellis require Tokio?

No.

The core should be runtime-neutral. Optional integration crates may provide helpers for specific runtimes.

### Can Trellis work in WebAssembly?

The core should be portable. Runtime integration depends on the commands you define and how you execute them.

### Does Trellis support dynamic dependencies?

Yes, but the design favors inspectable dynamic dependencies. A graph should be able to report what a node currently depends on and what resources it owns.

### Does Trellis replace a database?

No.

Trellis can decide what database queries should exist and how their results should be materialized, but it is not a database.

### Does Trellis replace a query cache?

No.

A query cache may be one of the resources commanded by Trellis. Trellis decides which query shapes should be active and how their outputs affect the graph.

### Does Trellis replace an actor framework?

No.

Trellis works well inside an actor. The actor owns the graph, receives events, runs transactions, applies commands, and emits frames.

### Why not let effect closures perform I/O directly?

Because resource lifecycle is part of correctness.

Plans are easier to inspect, test, order, deduplicate, log, and replay.

### What does “fail closed” mean?

If a derived source is empty, invalid, or unauthorized, Trellis should not broaden demand accidentally.

For example:

```text
visible device set = empty
 -> subscribe to no device topics
```

not:

```text
visible device set = empty
 -> subscribe to all device topics
```

### What does “revisioned output” mean?

Every output frame is associated with a graph revision.

Consumers can use revisions to reject stale frames, apply deltas safely, or request a rebaseline.

### Can Trellis emit deltas instead of baselines?

Yes. A materialized output can emit baselines, deltas, clears, rebaselines, or no frame.

### What is the recommended architecture?

Use one owner for the graph.

```text
external event -> host actor -> graph transaction -> command batch + output frames
```

Apply commands after the graph transaction.

### Is this production ready?

Not yet.

The initial goal is to prove the abstraction against realistic examples and stabilize the core transaction/resource/output model.

---

## Contributing

Useful contributions:

- realistic example applications;
- critiques of the resource-plan model;
- tests for lifecycle edge cases;
- performance traces;
- API simplification proposals;
- comparison notes with existing Rust crates;
- property-test generators;
- runtime integration experiments;
- documentation for failure modes.

Especially useful examples:

```text
workspace-driven sync;
live database subscriptions;
file watcher -> diagnostics;
telemetry topic subscriptions;
plugin contributions;
asset streaming;
search shard readers;
collaborative document hydration.
```

When proposing features, include:

```text
what canonical input changes;
what derived value changes;
what collection diff is produced;
what resource plan should result;
what output should be emitted;
what happens when the source is empty;
what happens when the scope closes;
how to verify with full recompute.
```

---

## License

TBD.

Suggested options:

```text
MIT OR Apache-2.0
```

---

## Summary

Trellis is for Rust systems where state owns resources.

It gives names and runtime support to a common application-kernel pattern:

```text
input
 -> derivation
 -> collection diff
 -> resource plan
 -> materialized output
 -> revision
 -> verification
```

Use it when recomputing a value is not enough.

Use it when the correct behavior is to reconcile live resources, tear them down safely, and emit coherent output.
