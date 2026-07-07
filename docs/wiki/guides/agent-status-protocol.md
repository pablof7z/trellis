---
title: "Agent Status Protocol (kind:30315)"
slug: agent-status-protocol
topic: nostr-protocol
summary: "kind:30315 is a NIP-33 replaceable event addressed by (pubkey, `d`=session_id), used for publishing agent status"
tags:
  - capture
volatility: warm
confidence: medium
created: 2026-07-03
updated: 2026-07-03
verified: 2026-07-03
compiled-from: conversation
sources:
  - session:065295ad-311d-4965-a3c1-6f749135f2b8
---

# Agent Status Protocol (kind:30315)

## Nostr Event Kind

kind:30315 is a NIP-33 replaceable event addressed by (pubkey, `d`=session_id), used for publishing agent status. The event encodes tags for `d` (session_id), `title`, `status` (busy|idle), `host`, repeated `h` for each channel, optional `slug`, `rel-cwd`, and `expiration` (NIP-40). The event content carries the live activity text, which is empty when idle. The `STATUS_TTL_SECS` constant is 90 seconds, so published status events age off via NIP-40 expiration set to now + 90s, re-armed every `HEARTBEAT_SECS` (30 seconds) up to 90s after the last publish. No NIP-09 kind:5 deletion events are published anywhere in tenex-edge; status (kind:30315) is never actively deleted and ages off solely via NIP-40 expiration.

`distill::distill_session()` produces the live activity text and stable title for a session by calling an LLM on a recent transcript slice. It yields two labeled outputs: `TITLE:` (nudged-to-keep across calls) and `NOW:` (regenerated every call).

<!-- citations: [^06529-7307c] [^06529-fddbc] [^06529-8276b] [^06529-033ae] [^06529-42e19] -->
## Publish Paths

tenex-edge has a single derived kind:30315 publish node that collapses all five existing publish triggers (startup, per-engine heartbeat, daemon-wide heartbeat re-armer, distill completion, turn-end) and the two independent ~30s timers into one node that emits only on change.

<!-- citations: [^06529-4ba1f] [^06529-02c2b] [^06529-8105d] [^06529-6d8ce] -->
## Redundancy and Collisions

A single derived node emits a kind:30315 only when title, activity, or busy changes, so there is no double-publishing from uncoordinated timers and no unconditional republish on every heartbeat tick, turn-end, startup, or distill-applied event when the values are byte-identical to the last publish.

<!-- citations: [^06529-f5974] [^06529-87756] [^06529-9c2d0] [^06529-6d8ce] -->
## Death and Channel-Exit Handling

When a session dies, `mark_dead()` only flips `alive=0, working=0` locally; it does not publish a final, blank, or expired kind:30315, so remote peers continue to see the dead session as live for up to 90s (`STATUS_TTL_SECS`) after it ended. Because kind:30315 is a NIP-33 replaceable event addressed by (pubkey, d=session_id) and not per-channel, when a session leaves a channel the previously published status event under the old `h` tag is never retracted and ages off via its own expiration up to 90s later.

<!-- citations: [^06529-442f2] [^06529-fa1f2] [^06529-f2cc5] -->
## Traceability Gaps

A single derived node collapses the five previously uncoordinated triggers (startup, per-engine heartbeat, daemon-wide heartbeat re-armer, distill completion, turn-end) into one emit-only-on-change node, so there is no longer a need to guess which of five trigger paths produced a 30315 publish.

<!-- citations: [^06529-fbad5] [^06529-3a2d8] [^06529-9ec10] [^06529-6d8ce] -->
## Distill Pipeline

The distill→status pipeline re-reads the transcript fresh each time via `read_recent(path, 14, 2500)`, tailing the last 96KB, last 14 user/assistant messages, capped at 2500 chars. The distill pipeline spawns a background tokio task with a 20s timeout for each `distill_session` call.

Distill resolution order is `$TENEX_EDGE_DISTILL_CMD` override, then the edge-distillation role from `llms.json`/`providers.json`, dispatched to `claude-cli` binary or native rig (openrouter/ollama), with a nudge-to-keep fallback that retains the current title and blanks activity.

On distill completion, the result is written to `sessions.title`/`activity` via `set_session_distill`. The single derived node then emits a 30315 only if title, activity, or busy actually changed since the last publish.

<!-- citations: [^06529-9523b] [^06529-770f4] [^06529-6d8ce] -->
