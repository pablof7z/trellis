---
type: noun-entry
slug: window-hash
name: "window_hash"
origin: extracted
source_refs:
  - transcript:1255-1255
  - transcript:2082-2089
---

# window_hash

A sha256 hash of the transcript slice fed to the distill LLM, computed at the host boundary and threaded into both the llm_calls row and the status receipt's changed_summary — the join key that lets `explain event:<30315>` recover the exact LLM inputs behind an activity.
