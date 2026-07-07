---
type: noun-entry
slug: commitfacts
name: "CommitFacts"
origin: extracted
source_refs:
  - transcript:4619-4628
---

# CommitFacts

A Trellis-free flattening of a TransactionResult that the drive seams use for persistence — carries transaction_id, revision, changed inputs/derived/collections as labels (not ids), command/output counts, graph node count, and a noop flag. The bridge between the graph's typed result and the trellis_commits ledger.
