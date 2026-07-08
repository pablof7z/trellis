# Examples

Examples live outside `trellis-core` in `crates/trellis-examples`.

They are normative design pressure: if an example requires domain vocabulary in
the core crate, the abstraction is wrong or the example belongs outside Trellis.
Product-facing showcase APIs should also follow the
[Showcase API Boundary](SHOWCASE_API_BOUNDARY.md): Trellis remains private to
an app-owned wrapper, while hosts send domain events and receive typed domain
effects and frames.

## Workspace Sync Board

File: `crates/trellis-examples/src/workspace_sync_board/`

This is the flagship local-first showcase and supersedes the compact
`workspace_sync.rs` proof module for product-facing API decisions. It exposes
`WorkspaceBoardApp` with `open_workspace_board`, `apply_user_event`,
`drain_sync_effects`, `drain_output`, and `close`; Trellis graph handles,
resource commands, scopes, and output keys remain private.

Shape:

```text
active user
 -> selected org/workspace or personal view
 -> permission set
 -> visible project set
 -> project/comment/profile sync windows
 -> materialized issue board frames
```

Covered behavior:

- workspace switch closes old sync windows;
- permission revoke clears forbidden rows;
- empty workspace opens no windows;
- assigned-to-me view derives projects from cached issue assignees;
- visible-column filter changes emit a board rebaseline;
- incremental result is checked against full recompute.

Compact proof: `crates/trellis-examples/src/workspace_sync.rs` keeps the
smallest graph shape for invariant tests.

## Mini Language Server

File: `crates/trellis-examples/src/mini_language_server.rs`

Shape:

```text
file contents
 -> module graph
 -> affected file set
 -> watcher resource plan
 -> diagnostics output
```

Covered behavior:

- deleting the open/root file clears diagnostics and closes watchers;
- import edge changes move affected files, watcher demand, and diagnostics;
- incremental result is checked against full recompute.

## FleetPulse Telemetry Dashboard

File: `crates/trellis-examples/src/fleetpulse/`

This is the flagship telemetry showcase and supersedes the compact
`telemetry_dashboard.rs` proof module for product-facing API decisions. It
exposes `FleetPulseApp` with `open_fleet_dashboard`,
`apply_filter_change`, `apply_permission_change`, `apply_host_status`,
`drain_effects`, `drain_output`, and `close`; Trellis graph handles,
resource keys, scope ids, and output keys remain private.

Shape:

```text
current user
 -> permission set
 -> selected customer/site/filter
 -> visible device set
 -> topic and alert-stream subscription sets
 -> resource plans
 -> telemetry card, alert, and status frames
```

Covered behavior:

- filter shrink unsubscribes removed topics;
- permission revoke clears unauthorized cards and alert streams;
- empty customer/device set subscribes to nothing and opens no wildcard;
- shared topic remains live while another panel needs it;
- late host status for a closed topic is classified and ignored;
- incremental result is checked against full recompute.

Compact proof: `crates/trellis-examples/src/telemetry_dashboard.rs` keeps the
smallest topic-subscription graph shape for invariant tests.

## Headless Showcase Traces

The flagship proof graphs expose deterministic headless scripts before any
interactive showcase UI. Each command prints pretty JSON using the shared
`trellis.showcase.trace` contract:

```sh
cargo run -p trellis-examples --example workspace_sync_board -- --script switch-workspace
cargo run -p trellis-examples --example mini_language_server -- --script delete-file
cargo run -p trellis-examples --example fleetpulse -- --script revoke-permission
cargo run -p trellis-examples --example plugin_host -- --script capability-lifecycle
cargo run -p trellis-examples --example market_desk -- --script market-lifecycle
cargo run -p trellis-examples --example photo_stream -- --script smart-album-lifecycle
cargo run -p trellis-examples --example search_ops -- --script search-lifecycle
```

The JSON includes the showcase name, script name, reproduction command,
deterministic replay status, seeded-bug status reserved for
[#93](https://github.com/pablof7z/trellis/issues/93), and named transaction
steps. Each step contains the payload-neutral `TransactionTrace`, host-status
metadata, resource commands, output frames, scope events, audit receipts,
phase trace, and a full-recompute invariant result.

## CollabCanvas Document Lifecycle

File: `crates/trellis-examples/src/collab_canvas/`

```sh
cargo run -p trellis-examples --example collab_canvas -- --script document-lifecycle
```

CollabCanvas is a secondary showcase for dynamic dependencies discovered from
document content. An open document derives embedded subdocument rooms, comment
threads, presence rooms, visible attachment hydration jobs, and materialized
editor output. Two document scopes can share a subdocument room; closing one
document does not close the shared room until the last owner leaves.

The script shows attachment visibility opening and closing hydration, embedded
documents opening and closing subdocument rooms, document close clearing editor
output, and full-recompute oracle checks on every step. The example also exposes
a seeded capsule for stale attachment hydration/output invalidation.

## PluginHost Capability Lifecycle

File: `crates/trellis-examples/src/plugin_host/`

```sh
cargo run -p trellis-examples --example plugin_host -- --script capability-lifecycle
```

PluginHost is a secondary showcase for desktop-app plugin runtimes. An enabled
plugin manifest derives command palette entries, shell panels, file watchers,
background workers, IPC channels, and typed shell output. Workspace kind and
permission grants are app-owned inputs; Trellis stays behind `PluginHostApp`
domain operations.

The script shows manifest contribution diffs, permission revocation closing
hidden capabilities, unsupported workspace changes removing all contributions,
supported workspace changes reopening allowed capabilities, plugin disable
closing scoped behavior, output clearing, and full-recompute oracle checks on
every step. The example also exposes a seeded capsule for stale capabilities
left open after plugin disable.

## MarketDesk Live Market-Data Terminal

File: `crates/trellis-examples/src/market_desk/`

```sh
cargo run -p trellis-examples --example market_desk -- --script market-lifecycle
```

MarketDesk is a secondary showcase for market-data terminals. A workspace
watchlist, open chart panels, user entitlements, and host quote metadata derive
quote feeds, trade feeds, order-book depth feeds, candle streams, and
materialized terminal output. Quote feeds can be shared by the grid and chart
scopes while depth and candle resources stay chart-owned.

The script shows symbol rotation closing removed feeds, chart open starting
depth/candle subscriptions, entitlement revoke closing forbidden feeds and
clearing rows, large watchlist churn producing a high-volume resource diff, and
workspace close clearing all scoped streams and output. The example also exposes
a seeded capsule for stale feeds/output after entitlement revoke.

## PhotoStream Smart Album Hydrator

File: `crates/trellis-examples/src/photo_stream/`

```sh
cargo run -p trellis-examples --example photo_stream -- --script smart-album-lifecycle
```

PhotoStream is a secondary showcase for photo-library hydration. A smart album
rule, visible viewport, cloud availability, and storage policy derive CPU
thumbnail decode jobs, disk metadata hydration jobs, cloud downloads,
memory-backed high-resolution previews, and bounded grid output.

The script shows album rule changes canceling removed jobs and starting added
jobs, viewport scroll reconciling offscreen/on-screen high-res work, storage
pressure dropping optional cloud/high-res resources, a large album expansion
producing collection diffs while output stays viewport-bounded, and scope close
clearing all jobs and grid output. The example also exposes a seeded capsule for
stale optional work under storage pressure.

## SearchOps Live Search/Index Dashboard

File: `crates/trellis-examples/src/search_ops/`

```sh
cargo run -p trellis-examples --example search_ops -- --script search-lifecycle
```

SearchOps is a secondary showcase for live search/index dashboards. A selected
corpus, query, filter, visible page window, host catalog, and user permissions
derive allowed shards, shard readers, query ranking jobs, result cache windows,
and materialized bounded result output.

The script shows query changes canceling stale ranking jobs without closing
unchanged shard readers, page-window changes rebaselining visible output and
cache work, permission revoke clearing unauthorized rows, corpus changes
replacing shard readers, and search close clearing all scoped resources and
output. The example also exposes a seeded capsule for stale search work/results
after permission revoke.

## Trellis Observatory Showcase Lab

File: `examples/codebase-observatory/`

The browser Observatory opens on an interactive showcase lab backed by committed
fixtures generated from the three headless scripts above. The lab exposes each
script step as a user action and derives its board/workbench/dashboard panels
from resource commands, output frames, collection diffs, host statuses, replay
metadata, and invariant results in the same trace.

The `Trace viewer` switch opens the structural inspector for the same fixtures.
It can inspect transactions, graph activity, collection diffs, resource plans,
output frames, scope events, host statuses, full-recompute oracle results,
replay metadata, conformance status, and structural cost counts. Clicking a
command, frame, scope event, or host status shows the available structural
cause: transaction id, revision, scope, identity, and the input/diff context
from the same trace.

## Protocol Subscription Wrapper

File: `crates/trellis-examples/src/protocol_subscription.rs`

Shape:

```text
session params
 -> source set
 -> desired interest set
 -> replay selector
 -> live subscription shape
 -> admitted local rows
 -> output frames
 -> scoped teardown
```

The public wrapper API exposes `ArticleFeedHandle`, `ArticleFeedParams`,
`SubscriptionEffect`, and `ArticleFeedFrame`. Trellis graph identities,
resource plans, and output frames remain internal to the example.
This is the current reference implementation of the
[Showcase API Boundary](SHOWCASE_API_BOUNDARY.md).

Covered behavior:

- closing a handle tears down scope-owned subscriptions and clears output;
- source shrink withdraws demand and removes admitted rows;
- an empty source set opens no broad demand;
- replay requests replace subscription shape and emit coherent rebaselines;
- `trellis-testing` ledgers assert lifecycle and output invariants.

## Internal Alpha Prototype

File: `crates/trellis-examples/src/internal_alpha.rs`

This is not a fourth domain proof example. It is the M17 acceptance harness for
one serious prototype path: filtered demand, scoped resource plans,
materialized rows, audit explanations, shared ownership, and full-recompute
checks.

The alpha result and seeded-bug coverage are documented in `docs/ALPHA.md`.

## Observatory Eval Capsules

File: `crates/trellis-observatory-engine/src/eval.rs`

The Observatory engine exposes runnable seeded-bug capsules for the current
Codebase Observatory showcase. Each capsule runs the Trellis-backed success
path and a seeded naive path over the same setup, then reports the
ResourceLedger, OutputLedger, host-status audit, or full-recompute oracle
failure that proves the lifecycle bug was detected.

Run all capsules:

```sh
cargo run -p trellis-observatory-engine --example eval_capsules -- --all
```

Run one capsule by name:

```sh
cargo run -p trellis-observatory-engine --example eval_capsules -- --capsule delete-file-lifecycle
```

The flagship Workspace Sync Board and FleetPulse examples expose their
app-specific seeded-bug capsules through their existing headless runners:

```sh
cargo run -p trellis-examples --example workspace_sync_board -- --capsules
cargo run -p trellis-examples --example workspace_sync_board -- --capsule workspace-switch-closes-old-windows

cargo run -p trellis-examples --example fleetpulse -- --capsules
cargo run -p trellis-examples --example fleetpulse -- --capsule fleet-late-closed-topic-status
```
