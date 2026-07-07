---
type: noun-entry
slug: transactiontrace
name: "TransactionTrace"
origin: extracted
source_refs:
  - transcript:430-430
  - transcript:436-436
---

# TransactionTrace

A payload-free, serde-capable Trellis projection containing ordered resource commands, coalescences, output frames, scope events, audit log, and phase trace; records what happened but cannot re-execute without the corresponding DataTransactionScript (which carries payloads).
