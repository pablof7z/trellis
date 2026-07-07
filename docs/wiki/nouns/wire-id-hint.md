---
type: noun-entry
slug: wire-id-hint
name: "wire_id_hint"
origin: extracted
source_refs:
  - transcript:69-69
  - transcript:299-309
---

# wire_id_hint

A diagnostic hint derived from canonical_filter_hash on the raw pre-compile child shape, intended as a short-term join key to correlate feed-session diagnostic receipts with relay/socket wire subscriptions; differs from the real wire sub id because it hashes the shape before author-partitioning and lattice-merge mutate it.
