---
type: noun-entry
slug: tx-preview
name: "tx.preview()"
origin: extracted
source_refs:
  - transcript:3825-3844
---

# tx.preview()

A dry-run of a Trellis transaction: runs the full commit pipeline (recompute, structural diffs, resource plans, output frames, audit) on the already-cloned working copy and returns the TransactionResult without the final std::mem::swap that writes back to the real graph — nearly free because the working clone is already paid at begin_transaction. The keystone for 'an agent never gambles on what it will do.'
