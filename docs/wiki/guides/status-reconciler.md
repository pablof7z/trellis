---
title: Status Reconciler
slug: status-reconciler
topic: trellis-adoption
summary: "The StatusReconciler is the single authority deciding when kind:30315 status is published, replacing five uncoordinated publish paths with change-only publish,"
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

# Status Reconciler

## Overview

The StatusReconciler is the single authority deciding when kind:30315 status is published, replacing five uncoordinated publish paths with change-only publish, TTL refresh, and deterministic expiry.

<!-- citations: [^06529-4242f] [^06529-0eb52] [^06529-02cfe] -->
## Trigger Consolidation

The spawn_status_heartbeat_publisher function and the direct set_status publish seam are deleted, collapsing five triggers and two timers into one change-only path through the outbox executor.

<!-- citations: [^06529-6758b] [^06529-9e083] -->
## Command Kinds

The status reconciler emits distinct command kinds: Open for new sessions, Replace for content changes, Refresh for TTL-only re-arms, and Close (Expire) for session teardown. <!-- [^06529-9f828] -->

## Dedup and TTL Refresh

The status reconciler models expiration as a bucketed arm value (now / refresh_secs) deliberately outside StatusContent so that an idle heartbeat TTL re-arm is never mistaken for a content change — this is what enables real dedup.

Status publishes only when content changes (Replace), only when TTL re-arm is due (Refresh), or on session end (Expire) — identical state commits produce no publish (dedup).

<!-- citations: [^06529-57af4] [^06529-31204] [^06529-7d4e2] -->
## Causal Attribution

The status reconciler's why_command receipt attributes a publish to its cause — e.g., a distill-driven publish's input_causes contains the activity input node. <!-- [^06529-152ed] -->

## Session-End Teardown

Session-end teardown publishes a final expiring 30315 with busy=false, empty activity, and last-known h-tags retained before marking the session dead.

<!-- citations: [^06529-2e610] [^06529-571bb] -->
## Channel Membership Correction

The status reconciler's on_channels_changed input corrects the derived h-tag set on join/leave, fixing stale h-tags after channel membership changes. <!-- [^06529-66ab9] -->

## Single-Driver Observation Loop

Turn edges, channel changes, and distill results are observed by the per-session runtime loop (5s obs tick) and drive the status reconciler, rather than being pushed directly from RPC handlers — this keeps a single driver and avoids split-brain, at the cost of up to one obs-interval of latency. <!-- [^06529-5b974] -->
