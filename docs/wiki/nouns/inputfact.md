---
type: noun-entry
slug: inputfact
name: "InputFact"
origin: extracted
source_refs:
  - transcript:1117-1132
---

# InputFact

An enum of canonical input events that the Trellis reconciler ingests (SessionStarted, TurnStarted, TranscriptWindowCaptured, DistillCompleted, TurnEnded, RelayEventObserved, RelayPublishAccepted, ProcessExited, ClockTick), each designed to replace a specific existing tenex-edge state writer, carrying only pointers/hashes/summaries rather than bulky payloads.
