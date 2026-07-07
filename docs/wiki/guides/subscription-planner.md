---
title: Subscription Planner
slug: subscription-planner
topic: subscription-lifecycle
summary: The SubscriptionReconciler is the Trellis reconciler that owns relay subscription lifecycle with per-entity refcounted ownership, emitting Open on first owner a
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

# Subscription Planner

## Subscription Planner

The SubscriptionReconciler is the Trellis reconciler that owns relay subscription lifecycle with per-entity refcounted ownership, emitting Open on first owner and real NIP-01 Close only on last-owner departure. It fixes the documented tens-of-GB relay-subscription leak by emitting a real NIP-01 CLOSE on last-owner departure through the channel-leave, channel-switch, and session-end teardown paths.

The old aggregate SubscriptionRegistry with add_channel, compact, seed, and the te-v2-*-all subscription ids is retired, replaced by the reconciler's narrow filter builders. The dead close_subs and SubscriptionRegistry::compact code is deleted.

The reconciler's graph has one daemon-subs scope for durable coverage (subscribed projects, member/admin channels, all #p) and one scope per alive session owning its joined channels — the session scope is the refcount.

Channel-leave, channel-switch, and session-end all emit real NIP-01 CLOSE on last-owner departure through the subscription reconciler.

All subscription writers (leave/switch/session-end/ensure/startup) funnel through a single `sync_subscriptions` path that reads the store as source of truth after each mutation, ensuring no split-brain.

The `build_entity_coverage` function is tenex-edge's pure state-to-coverage-set recompute that derives the full set of relay subscription filters from all alive sessions' joined channels, identities, and subscribed projects, called only at startup.

<!-- citations: [^06529-0ef3d] [^06529-e0a3b] [^06529-eedbb] [^06529-03e28] [^06529-4ef94] [^06529-f624a] -->
