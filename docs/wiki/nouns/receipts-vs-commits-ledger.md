---
type: noun-entry
slug: receipts-vs-commits-ledger
name: "receipts vs commits ledger"
origin: extracted
source_refs:
  - transcript:3157-3171
---

# receipts vs commits ledger

Two distinct persisted stores with different jobs: receipts explain artifacts (the 30315, the hook injection) and exist only for effectful commits; a separate trellis_commits ledger records every transaction including no-ops, unchanged recomputes, and suppressed publishes — the value evidence that receipts deliberately drop.
