---
title: tenex-edge Explain Command
slug: tenex-edge-explain
topic: trellis-adoption
summary: "The `tenex-edge explain <handle>` CLI verb queries artifact handles â `event:<id>`, `llm:<id>`, `session:<id>[@ts]`, `hook:<id>[@ts]`, `txn:<surface>:<id>`, a"
tags:
  - capture
volatility: warm
confidence: medium
created: 2026-07-03
updated: 2026-07-04
verified: 2026-07-03
compiled-from: conversation
sources:
  - session:065295ad-311d-4965-a3c1-6f749135f2b8
---

# tenex-edge Explain Command

## Command Overview

The `tenex-edge explain <handle>` CLI verb queries artifact handles — `event:<id>`, `llm:<id>`, `session:<id>[@ts]`, `hook:<id>[@ts]`, `txn:<surface>:<id>`, and `sub:<channel>` — backed by a persisted `receipts` table and an `llm_calls` ledger. The CLI forwards to a daemon explain RPC over the Unix socket, exactly like the `who` command.

The `--json` flag emits raw joined record output. The `--redact` flag replaces prompt, transcript, and response bodies with `sha256:<hash> (<n> bytes)`.

The `llm_calls` ledger table stores `session_id`, `window_hash`, `provider`, `model`, `system_prompt`, `transcript_slice`, `raw_response`, `parsed_title`, `parsed_activity`, and `created_at` for each distill LLM call.

The `receipts` table stores `surface`, `transaction_id`, `revision`, `changed_summary` (JSON), `commands` (JSON array of `{kind, key, reason}`), `artifact_ref`, and `created_at` for each reconciler transaction. `changed_summary` currently persists graph-local numeric NodeIds (e.g. `{"inputs":[2,3]}`) rather than human-readable labels, which are opaque without the live graph.

The `llm_calls` and `receipts` tables are created via `CREATE TABLE IF NOT EXISTS` and do not require a `SCHEMA_VERSION` bump because the version gate exists to catch breaking/destructive changes, not additive ones.

The status→LLM join is threaded by `window_hash`: a published 30315's receipt carries `window_hash` in `changed_summary`, which joins to `llm_calls` by `window_hash` to recover the exact system prompt, transcript slice, model, and raw response. `explain event:<30315>` surfaces the reconciler receipt and the exact LLM inputs behind a distilled activity; for non-distill publishes it prints `LLM inputs: (none — this publish was not distill-driven)`.

Audit explanations in trellis-core are latest-per-key and ephemeral: `why_changed` and `why_resource_command` answer the most recent change per key with no historical why. Historical why must come from persisted commits and replay.

<!-- citations: [^06529-d9c9e] [^06529-707cc] [^06529-45e48] [^06529-6b7c1] [^06529-90a3c] [^06529-b4541] -->
