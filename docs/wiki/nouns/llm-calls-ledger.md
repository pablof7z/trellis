---
type: noun-entry
slug: llm-calls-ledger
name: "llm_calls ledger"
origin: extracted
source_refs:
  - transcript:646-646
---

# llm_calls ledger

A persisted SQLite table capturing the exact transcript slice, system prompt, model/provider, and raw response of every distill call, linked by provenance to the input write it produced. Enables answering 'what was fed to the LLM?' for any given distilled activity.
