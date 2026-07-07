---
type: noun-entry
slug: turn-start-audit-turn-check-audit
name: "turn_start_audit / turn_check_audit"
origin: extracted
source_refs:
  - transcript:294-294
  - transcript:299-299
---

# turn_start_audit / turn_check_audit

A hand-rolled receipt function in tenex-edge that builds a structured JSON describing cursors before/after, joined_channels, evaluated ambient_chat and awareness (candidate channels, raw Status rows with busy/activity/title/last_seen/updated_at/expiration), plus the exact output.text emitted — returned alongside the context from turn_start/turn_check RPCs, but an approximation that diverges from the actual render.
