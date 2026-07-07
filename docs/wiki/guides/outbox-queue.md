---
title: Outbox Queue
slug: outbox-queue
topic: data-persistence
summary: "The outbox in tenex-edge is a working effect-plan-plus-receipt proof-of-concept: a durable queue where the runtime parks signed JSON on the outbox table, a drai"
tags:
  - capture
volatility: warm
confidence: medium
created: 2026-07-03
updated: 2026-07-03
verified: 2026-07-03
compiled-from: conversation
sources:
  - session:065295ad-311d-4965-a3c1-6f749135f2b8
---

# Outbox Queue

## Outbox Queue

The outbox in tenex-edge is a working effect-plan-plus-receipt proof-of-concept: a durable queue where the runtime parks signed JSON on the outbox table, a drainer publishes via `publish_event_checked`, and results are recorded as `mark_published` or `mark_failed` with retry count. It is scoped to status events only — chat, subscriptions, tmux spawns, and membership admission bypass it and fire inline.

<!-- citations: [^06529-2eb76] [^06529-d9eaf] [^06529-69a93] -->
