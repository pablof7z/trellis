---
type: noun-entry
slug: capsule-xraycapsule
name: "capsule (XrayCapsule)"
origin: extracted
source_refs:
  - transcript:436-436
  - transcript:450-450
  - transcript:784-784
---

# capsule (XrayCapsule)

A recordable/replayable debugging artifact. Capsule v1 is defined as trace + receipts, scrub/reconstruction only (not re-execution). It embeds an envelope with its own version, TRACE_FORMAT_VERSION, NMP receipt schema version, producer metadata, redaction mode, and a symbolication manifest (NodeId/ScopeId → semantic label registry).
