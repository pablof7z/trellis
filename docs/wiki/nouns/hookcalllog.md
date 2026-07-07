---
type: noun-entry
slug: hookcalllog
name: "HookCallLog"
origin: extracted
source_refs:
  - transcript:295-296
  - transcript:534-535
---

# HookCallLog

A per-session JSONL forensic log at <edge_home>/sessions/<session_id>/hook-calls.jsonl recording every hook invocation's raw stdin, process/parent-chain snapshot, redacted env, and a context-audit note with the exact injected text — a hand-rolled receipt for hook context injection.
