---
title: Hook Context Surface
slug: hook-context-surface
topic: trellis-adoption
summary: The HookCallLog is a JSONL forensic log in tenex-edge appended on every hook invocation
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

# Hook Context Surface

## HookCallLog

The HookCallLog is a JSONL forensic log in tenex-edge appended on every hook invocation. Each entry records raw stdin, a process/parent-chain snapshot, a redacted environment snapshot, and a context-audit note containing the exact injected text. <!-- [^06529-8a5cd] -->
