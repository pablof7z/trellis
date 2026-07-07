---
type: noun-entry
slug: trellis-commits-all-commit-ledger
name: "trellis_commits (all-commit ledger)"
origin: extracted
source_refs:
  - transcript:4631-4646
---

# trellis_commits (all-commit ledger)

A SQLite table recording every reconciler transaction including no-ops (unlike receipts which only capture effectful commits), with surface, trigger_kind, changed inputs/derived/collections as JSON, command/output counts, noop flag, duration_us, and graph_nodes — the value meter's data source.
