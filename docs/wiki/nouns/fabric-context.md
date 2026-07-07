---
type: noun-entry
slug: fabric-context
name: "fabric context"
origin: extracted
source_refs:
  - transcript:230-236
  - transcript:376-376
---

# fabric context

The injected agent context (XML snapshot) built purely from live SQLite store reads via build_view, assembled from project/channel metadata, members, presence/status deltas, chat/mentions, and invitable agents. Recomputed every hook call with no cache layer; shape gated by a seen_cursor high-water mark that mutates full-snapshot vs delta rendering.
