---
title: Wire ID Shape Correlation
slug: wire-id-shape-correlation
topic: data-correlation
summary: The `wire_id_hint` must not be trusted as an authoritative join key
tags:
  - capture
volatility: warm
confidence: medium
created: 2026-07-03
updated: 2026-07-03
verified: 2026-07-03
compiled-from: conversation
sources:
  - session:f940bd78-c4e8-413d-82a8-53aa459f690c
---

# Wire ID Shape Correlation

## `wire_id_hint` Trust and Relay Correlation

The `wire_id_hint` must not be trusted as an authoritative join key. It hashes the pre-compile shape, while the wire id hashes the post-compile shape. This divergence causes author partitioning and lattice merge to produce different hashes for multi-author feeds, breaking reliable correlation.

The proper long-term fix for relay correlation is to correlate on `SubShape.originating_interests` (the real `InterestId`s) rather than the shape hash. This requires threading the registry-assigned `InterestId` onto the feed-session receipt.

If `wire_id_hint` is merged as an interim scaffold, it should be relabeled as a best-effort shape fingerprint. Miss outcomes should be changed from `pending`/`missing_wire_row` to an explicit `unknown("uncorrelated")`. Both the producer and correlation paths must be gated behind an experimental-lossy flag. <!-- [^f940b-a0a81] -->
