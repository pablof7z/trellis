---
type: noun-entry
slug: hook-context-fabric-snapshot
name: "hook context (fabric snapshot)"
origin: extracted
source_refs:
  - transcript:230-236
  - transcript:376-376
  - transcript:222-228
---

# hook context (fabric snapshot)

The injected agent context — a <tenex-edge> XML block built purely from live SQLite store reads (channel metadata, members, presence deltas, chat/mentions, invitable agents), assembled by build_view and rendered by render_view, recomputed every hook call with a cursor-driven full-vs-delta shape.
