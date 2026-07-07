---
title: Fabric Context Snapshot Rendering
slug: fabric-context-snapshot
topic: fabric-context
summary: "The fabric context snapshot's full-vs-delta shape is gated by `cursor == 0`: full `<members>` and `<subchannels>` render only when cursor is 0 (practically the"
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

# Fabric Context Snapshot Rendering

## Full vs Delta Snapshot Gating

The fabric context snapshot's full-vs-delta shape is gated by `cursor == 0`: full `<members>` and `<subchannels>` render only when cursor is 0 (practically the first turn). `<recent-presence>` only appears on later turns, for entries whose status changed since the last cursor. <!-- [^06529-1f491] -->


Full `<members>` and `<subchannels>` render only when `cursor == 0` (practically the first turn of a session). `<recent-presence>` appears only on later turns for entries whose status changed since the last cursor. <!-- [^06529-57803] -->
## Unjoined Channels

The `unjoined_channels` list in tenex-edge's fabric context is silently truncated with `.take(12)` with no '+N more' indicator. It is ordered by `updated_at DESC` (most-recently-metadata-updated first, not most-recently-chatted). <!-- [^06529-7fd31] -->


The `unjoined_channels` list in tenex-edge's fabric context is ordered by SQL `ORDER BY updated_at DESC` (most-recently-metadata-updated first, not most-recently-chatted), silently truncated with `.take(12)` with no '+N more' indicator. `last_active` is rendered as `relative_time(c.updated_at, now)`. <!-- [^06529-7fd31] -->
## Member Ordering

Member pubkeys in the fabric context are gathered into a `BTreeSet<String>` and iterated in raw hex pubkey lexicographic order, not by name, slug, or join-time. <!-- [^06529-7d255] -->


Member pubkeys in the fabric context are gathered into a `BTreeSet<String>` and iterated sorted lexicographically by raw hex pubkey, not by slug, name, or join-time. Channel ordering is likewise hash-lexicographic via `channels.sort().dedup()` on the channel hash string. <!-- [^06529-7d255] -->
## Fabric Context Snapshot Rendering

The fabric context snapshot is assembled and rendered by tenex-edge from live SQLite data on every hook call — there is no cache layer. HookContextReconciler models the fabric snapshot as six declared inputs (cursor, now, channel-meta, members, presence, messages) → a derived FabricView node → a materialized output frame. The build assembles a FabricView from four live data sources: project/channel metadata, members, presence/status deltas, and chat/mentions, plus invitable agents. The seen_cursor high-water mark mutates the shape of the next render (full snapshot vs. delta). Captures for the hook-context graph are now/cursor-independent supersets so that liveness windows, chat windows, the full-vs-delta shape decision, and relative_time strings are pure functions of cursor and now inputs inside assemble_view. An undeclared read in the hook-context graph is a Trellis error, making the old awareness/render scope mismatch impossible by construction.

<!-- citations: [^06529-8502c] [^06529-4d22a] -->
## Hook Events and Cursor Lifecycle

Two Claude Code hook events feed fabric context assembly. `UserPromptSubmit` maps to the `turn_start` RPC (`assemble_turn_start_context`, `cursor = seen_cursor` which is 0 on first turn for a full render, advancing to `now` after rendering). `PostToolUse` maps to the `turn_check` RPC (`assemble_turn_check_context`, `cursor = delta_since` from a CAS-advanced `seen_cursor`, rendering nothing if the CAS lost a race). Fabric context is emitted as plain text on stdout for `UserPromptSubmit` and as a `hookSpecificOutput.additionalContext` JSON envelope for `PostToolUse`. <!-- [^06529-ceddf] -->

## Member Composition

A member in fabric context is the union of formal NIP-29 channel membership, anyone with a live status row for that channel (presence without formal membership), and self. Backends are filtered out via the `relay_profiles.is_backend` flag. <!-- [^06529-424a4] -->

## Status Rendering and TTL

The status text in fabric context shows the activity (or title if activity is empty) when `busy=true`, falling back to the literal string 'working'. When `busy=false`, it shows the title, falling back to 'idle'.

A status row counts as live only while `now <= expiration`, with `expiration = now + STATUS_TTL_SECS` (90s), re-armed every `HEARTBEAT_SECS` (30s). A stale (>90s silent) agent disappears with no explicit offline marker beyond the default fallback string 'offline' when no status row exists. <!-- [^06529-0ec8c] -->

## Message Rendering and Clustering

Fabric context message rendering uses a 4-hour window floor for full renders and fetches up to 10,000 rows. On full renders, it keeps only the most recent contiguous burst up to 30 rows (`MAX_CLUSTER_ROWS`) with a 20-minute gap boundary (`MAX_CLUSTER_GAP_SECS`), reporting the rest as an omitted count rendered as `<omitted count='N' window='last 4h' />`. Delta renders show everything since the cursor, uncapped by cluster logic. <!-- [^06529-64251] -->

## Audit and Logging

The drifting turn_start_audit and turn_check_audit are deleted, replaced by a receipt sourced from why_output_frame/why_changed that cannot drift from the render. (Previously: tenex-edge had a hand-rolled partial receipt via turn_start_audit() and turn_check_audit() that built structured JSON with cursors, joined_channels, evaluated ambient_chat and awareness, plus the exact output.text emitted; that audit was an approximation that diverged from the actual render, with only output.text being byte-exact.)

<!-- citations: [^06529-978cc] [^06529-d2c94] -->
## Non-Determinism and Replayability

Time-relative strings in fabric context (`relative_time` buckets: 'just now' <60s, 'N min ago' <1h, 'N hour(s) ago' <24h, 'yesterday' <48h, 'N days ago') make re-rendering the same state a few seconds later silently change the output, so renders are not diffable or replayable without embedding wall-clock `now`. Additionally, two parallel `PostToolUse` hooks can nondeterministically produce different injected context depending on which wins the compare-and-swap race on `seen_cursor`, because only the first wins the CAS and the rest get `delta_since=None` and emit nothing. <!-- [^06529-b7c77] -->

## Reconciler Lifecycle

The hook-context reconciler is instantiated fresh per render and discarded — there is no long-lived hook-context graph in the daemon to inspect. <!-- [^06529-2c245] -->
