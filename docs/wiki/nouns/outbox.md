---
type: noun-entry
slug: outbox
name: "outbox"
origin: extracted
source_refs:
  - transcript:346-346
  - transcript:389-390
  - transcript:425-425
---

# outbox

A durable signed-event publish queue table in tenex-edge — the existing effect-plan + receipt proof-of-concept scoped to status events only, where the runtime parks signed JSON and a drainer later publishes with mark_published/mark_failed/retries.
