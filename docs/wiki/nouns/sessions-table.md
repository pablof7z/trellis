---
type: noun-entry
slug: sessions-table
name: "sessions table"
origin: extracted
source_refs:
  - transcript:341-341
---

# sessions table

The canonical local process record in tenex-edge's SQLite store. PK session_id; carries agent_pubkey, channel_h, child_pid, alive, last_seen, working, turn_started_at, last_distill_at, title, activity, resume_id. The runtime task is described as 'a thin driver over the local sessions row.'
