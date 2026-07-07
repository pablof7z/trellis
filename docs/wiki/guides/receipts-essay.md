---
title: Receipts Essay Modal
slug: receipts-essay
topic: ui-components
summary: The essay 'Receipts.' lives at /receipts/ on f7z.io
tags:
  - capture
volatility: warm
confidence: medium
created: 2026-07-04
updated: 2026-07-04
verified: 2026-07-04
compiled-from: conversation
sources:
  - session:065295ad-311d-4965-a3c1-6f749135f2b8
---

# Receipts Essay Modal

## Essay Location and Entry Point

The essay 'Receipts.' lives at /receipts/ on f7z.io. It opens as a centered popup modal (not a sidebar sheet) when someone clicks the Trellis card on /stuff. <!-- [^06529-a45f9] -->

## Modal Behavior

The essay modal handles the click in the capture phase and stops the event before any sidebar handler can run. It uses inline essay content instead of an iframe. The modal closes via Esc, ×, or backdrop. <!-- [^06529-833ea] -->

## Trellis Card Copy

The Trellis card copy on /stuff reads: 'Never guess what your code did. Never gamble on what it'll do.' followed by a two-liner naming the forgotten-subscription pain before the turn. <!-- [^06529-26853] -->
