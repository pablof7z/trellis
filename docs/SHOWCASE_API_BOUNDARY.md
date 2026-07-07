# Showcase API Boundary

Status: normative draft for showcase and adapter examples.

This document defines how showcase applications expose Trellis-backed behavior
without making Trellis the application programming model. It is the boundary
that flagship showcase issues should use when deciding whether an API is clean
enough to be product-facing.

## Rule

Trellis stays behind an application-owned wrapper.

The wrapper owns the `Graph`, node handles, scopes, output keys, resource
plans, transaction results, and trace plumbing. Public callers send domain
inputs and receive domain effects, domain frames, opaque handles, and optional
diagnostic receipts.

```text
host event / product fact
 -> app-owned wrapper method
 -> private Trellis transaction
 -> typed host effects and typed output frames
 -> optional trace or receipt for diagnostics
```

The host may apply returned domain effects. It must not inspect Trellis
resource commands, output frame kinds, node ids, scope ids, dependency paths, or
dirty sets to decide product behavior. Those values are evidence for tooling
and tests, not the public app contract.

## Public Surface

A clean wrapper exposes:

- opaque handles for long-lived sessions, dashboards, workspaces, or panels;
- domain parameters and domain events as inputs;
- typed effect commands for the host executor;
- typed output frames for UI, protocol, or app state consumers;
- explicit `close` or teardown operations for scoped lifetimes;
- optional debug accessors that return traces, labels, or receipts without
  feeding them back into product decisions.

The wrapper may translate private Trellis `ResourceCommand`s into typed effects
and private output frames into typed domain frames. That translation belongs in
the wrapper, not in arbitrary host code.

## Forbidden Leaks

A showcase API is not clean if public callers must:

- hold `Graph`, `InputNode`, `NodeId`, `ScopeId`, `OutputKey`, or
  `ResourceKey` values;
- match on `ResourceCommand` or `OutputFrameKind`;
- read dirty-node lists to decide which product rows to change;
- interpret audit explanations as the source of product truth;
- decide teardown by comparing previous and next domain state outside the
  wrapper;
- manually clear output after scope close because Trellis emitted a clear
  frame.

Diagnostics are the exception. Observatory views, tests, and debug tools may
inspect traces, resource plans, dependency paths, and audit receipts. They must
not become required application inputs.

## Reference Pattern

`crates/trellis-examples/src/protocol_subscription` is the current reference
wrapper. `ArticleFeedApp` keeps `Graph`, input nodes, sessions, output keys,
and transaction results private. Its public API uses:

- `ArticleFeedHandle` for an opaque session handle;
- `ArticleFeedParams` to open a feed from application identifiers;
- `set_route_sources` and `replace_source_rows` for domain input changes;
- `request_replay` and `close` for explicit lifecycle operations;
- `drain_subscription_effects` for typed host effects;
- `poll_output` for typed `ArticleFeedFrame`s.

The host receives `SubscriptionEffect::Open`, `Replace`, or `Close` with
domain subscription shapes. It does not see Trellis resource commands. The host
receives feed baselines, deltas, replays, or clear frames as article-feed
frames. It does not see output keys or frame internals.

## Workspace Sync Board

The flagship workspace board should expose a wrapper shaped like:

```text
open_workspace_board(params) -> WorkspaceBoardHandle
apply_user_event(handle, WorkspaceBoardEvent) -> WorkspaceBoardUpdate
apply_host_status(handle, WorkspaceHostStatus) -> WorkspaceBoardUpdate
drain_output(handle) -> Vec<BoardFrame>
close(handle) -> WorkspaceBoardUpdate
```

Domain inputs include active workspace, permissions, route, visible columns,
project metadata, and host-reported sync status. Domain effects include opening
or closing sync windows. Domain frames include board baselines, row deltas,
status frames, and clears.

The host should not decide which sync windows to close by diffing project sets
itself. It applies the typed effects returned by the wrapper.

## Mini Language Server Workbench

The language-server showcase should expose a wrapper shaped like:

```text
open_workspace(root) -> LanguageWorkspaceHandle
did_open(handle, file, contents) -> LanguageUpdate
did_change(handle, file, edit) -> LanguageUpdate
did_delete(handle, file) -> LanguageUpdate
apply_host_status(handle, LanguageHostStatus) -> LanguageUpdate
drain_diagnostics(handle) -> Vec<DiagnosticFrame>
drain_semantic_tokens(handle) -> Vec<SemanticTokenFrame>
close(handle) -> LanguageUpdate
```

Domain inputs include file contents, import edges, config, editor visibility,
and host-reported watcher or analysis status. Domain effects include watcher
opens/closes and analysis-job demand. Domain frames include diagnostics,
semantic tokens, index status, and clears.

The host should not use Trellis dirty-node lists to decide which diagnostics
to invalidate. It drains typed output frames.

## FleetPulse Telemetry Dashboard

The telemetry dashboard showcase should expose a wrapper shaped like:

```text
open_fleet_dashboard(params) -> FleetDashboardHandle
apply_filter_change(handle, FleetFilterChange) -> FleetUpdate
apply_permission_change(handle, FleetPermissionChange) -> FleetUpdate
apply_host_status(handle, FleetHostStatus) -> FleetUpdate
drain_output(handle) -> Vec<FleetFrame>
close(handle) -> FleetUpdate
```

Domain inputs include customer, site, device filters, permissions, alert
selection, and host-reported topic status. Domain effects include opening,
replacing, or closing topic subscriptions. Domain frames include live cards,
alert-panel frames, status frames, and clears.

The host should not interpret resource keys to decide which cards are still
authorized. Permission and empty-source behavior must fail closed inside the
wrapper.

## Observatory Boundary

The Observatory is allowed to inspect Trellis internals because it is a
diagnostic surface. It is not part of the app-facing API for any showcase.

That means an app can expose a trace export, label registry, replay capsule, or
audit receipt for Observatory consumption, but normal host execution must still
use typed domain effects and frames.

## Acceptance Checklist

Before a showcase API is treated as product-facing, verify:

- public method names use domain vocabulary, not Trellis vocabulary;
- Trellis handles and ids remain private to the wrapper;
- resource commands are translated into typed host effects in one place;
- output frames are translated into typed domain frames in one place;
- close/teardown is explicit and scoped;
- host statuses re-enter as canonical domain inputs;
- tests or capsules can inspect receipts without making receipts product state;
- the full-recompute oracle and ledgers validate the lifecycle behavior.
