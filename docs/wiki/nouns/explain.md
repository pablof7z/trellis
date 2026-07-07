---
type: noun-entry
slug: explain
name: "explain"
origin: extracted
source_refs:
  - transcript:2802-2813
---

# explain

A query engine (tenex-edge explain <handle>) that points at an artifact (event:, hook:, llm:, session:, txn:, sub:) and returns what produced it — joining the receipts and llm_calls ledgers, threaded by window_hash from a published 30315 back to the exact LLM inputs (system prompt, transcript slice, model, raw response).
