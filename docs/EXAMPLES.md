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
```

The JSON includes the showcase name, script name, reproduction command,
deterministic replay status, seeded-bug status reserved for
[#93](https://github.com/pablof7z/trellis/issues/93), and named transaction
steps. Each step contains the payload-neutral `TransactionTrace`, host-status
metadata, resource commands, output frames, scope events, audit receipts,
phase trace, and a full-recompute invariant result.

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
