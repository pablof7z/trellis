---
title: tenex-edge SQLite Schema
slug: tenex-edge-sqlite-schema
topic: data-persistence
summary: "tenex-edge is a Rust/Nostr daemon that derives live resources â relay subscriptions, kind:30315 status events, presence, and hook context â from changing st"
tags:
  - capture
volatility: warm
confidence: medium
created: 2026-07-03
updated: 2026-07-04
verified: 2026-07-03
compiled-from: conversation
sources:
  - session:065295ad-311d-4965-a3c1-6f749135f2b8
---

# tenex-edge SQLite Schema

## Schema Tier Model

tenex-edge is a Rust/Nostr daemon that derives live resources — relay subscriptions, kind:30315 status events, presence, and hook context — from changing state. The canonical state the daemon owns is split into non-rebuildable local SQLite tables (sessions, session_channels, session_aliases, identities, inbox, outbox, project_roots) and rebuildable `relay_*` materialized cache tables filled solely by the inbound materializer. The schema is stamped and fail-loud (SCHEMA_VERSION=1) with no migrations; `check_schema_version` bails on mismatch or an unstamped-but-populated DB. Additive `CREATE TABLE IF NOT EXISTS` additions do not require a SCHEMA_VERSION bump per the convention that the version gate catches breaking/destructive changes, not additions.

The `llm_calls` ledger table stores per-LLM-call rows with `id`, `session_id`, `window_hash`, `provider`, `model`, `system_prompt`, `transcript_slice`, `raw_response`, `parsed_title`, `parsed_activity`, and `created_at` (passed in, never clock-read). The `receipts` table stores `id`, `surface` (subscriptions|status|hook_context), `transaction_id`, `revision`, `changed_summary` (JSON), `commands` (JSON array of {kind, key, reason}), `artifact_ref`, and `created_at`. Both are additive `CREATE TABLE IF NOT EXISTS` additions.

The `trellis_commits` table records every transaction including no-ops, with columns `surface`, `transaction_id`, `revision`, `trigger_kind`, `changed_inputs_json`, `changed_derived_json`, `changed_collections_json`, `command_count`, `output_count`, `noop`, `duration_us`, `graph_nodes`, and `created_at`.

<!-- citations: [^06529-a0e21] [^06529-74125] [^06529-0f95c] [^06529-a5fd2] [^06529-8c39e] -->
## Retention and Pruning

`relay_status` DB rows in tenex-edge are never purged; `retention.rs` prunes only `relay_events`, `inbox`, and `outbox`, leading to unbounded `relay_status` row growth over time.

`relay_status` DB rows in tenex-edge are never purged; `retention.rs` prunes only `relay_events`, `inbox`, and `outbox`, leading to unbounded `relay_status` row growth over time. <!-- [^06529-8a482] -->

<!-- citations: [^06529-8a482] -->

## Presence

Presence in tenex-edge is derived on read, never a stored boolean; liveness is computed as NIP-40 expiration >= now over `relay_status` rows. <!-- [^06529-794c8] -->


Presence in tenex-edge is derived on read, never a stored boolean; liveness is computed as NIP-40 expiration >= now over `relay_status` rows. <!-- [^06529-70e32] -->
## Membership Lifecycle

Relay-side membership removal in tenex-edge is explicitly best-effort: `membership_cleanup` spawns a detached task per channel and silently retains the stale local membership row if relay removal is not confirmed. `reconcile_sessions` runs at daemon restart to re-derive session truth because the live-path cleanup alone is not trustworthy; without this, who and routing membership would lie after every daemon restart. No refcounting mechanism exists anywhere in tenex-edge; membership is tracked only via row-existence in `session_channels` and `relay_channel_members`, and `build_entity_coverage` is the only function that scans all sessions to derive who needs a channel but is called solely at startup. <!-- [^06529-cd5df] -->

## Lock File

The tenex-edge lock file must not be removed on shutdown because deleting it while the flock is still held lets a racing spawner open a new file with a different inode and acquire an independent lock, causing two daemons to overlap. <!-- [^06529-c8123] -->

tenex-edge enforces a 500-LOC-per-file hard ceiling and a 300-LOC soft ceiling that new code must respect, requiring new logic to go in new modules. <!-- [^06529-56d0f] -->

## Distill Pipeline

tenex-edge distill scheduling is purely time-based — first distill fires when `now - turn_started_at >= turn_first` (default 30s), with no check of whether the transcript actually grew since the last distill. The pipeline reads the transcript fresh each time via `transcript::read_recent(path, 14, 2500)` (tails last 96KB, last 14 user/assistant messages, capped 2500 chars) and spawns distill as a background task with a 20s timeout. <!-- [^06529-368cf] -->

## Hook Output

tenex-edge hook output is emitted as plain text on stdout for `UserPromptSubmit` and as a `hookSpecificOutput.additionalContext` JSON envelope for `PostToolUse`. <!-- [^06529-d5db4] -->

## Node-Label Registry

A stable node-label registry per surface maps NodeIds to semantic paths (e.g. `status/<session>/activity`, `subscriptions/session/<session>/channels`, `hook/<session>/cursor`), populated at node creation and persisting both id and label. The `NodeLabels` struct maps `NodeId` to semantic string labels, populated at node creation, with `labels_for` returning `node:<n>` for unregistered ids. <!-- [^06529-ac0a3] -->
