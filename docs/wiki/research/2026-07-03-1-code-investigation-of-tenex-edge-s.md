---
type: research-record
date: 2026-07-03
session: 065295ad-311d-4965-a3c1-6f749135f2b8
transcript: /Users/pablofernandez/.claude/projects/-Users-pablofernandez-src-trellis/065295ad-311d-4965-a3c1-6f749135f2b8.jsonl
source_lines: 431-491
agent_attribution: main
has_preregistered_criteria: false
has_method: true
has_structured_report: true
characterization: "Code investigation of tenex-edge's live resource lifecycle (subscriptions, sessions, presence, refcounting) with verdicts on teardown gaps and drift; finds subscriptions structurally outlive state, refcounting absent"
captured_at: 2026-07-03T10:27:39Z
---

Code investigation of tenex-edge's live resource lifecycle (subscriptions, sessions, presence, refcounting) with verdicts on teardown gaps and drift; finds subscriptions structurally outlive state, refcounting absent

---

User: [Agent task result: Agent "tenex-edge resource lifecycle &amp; leaks" finished]
# tenex-edge Live Resource Lifecycle — Investigation Report

Repo: `~/src/tenex-edge`. Spot-verified against source (not just agent-reported).

## 1. SUBSCRIPTIONS — incremental-add only; no teardown path exists

- **Join/switch**: `ensure_subscription` (`src/daemon/server/subscriptions.rs:3-36`) computes only narrow deltas via `state.subscriptions.lock().unwrap().add_channel(project)` (`:18`). Comment at `:13-18`: *"Incremental add: plan only the NARROW deltas for this newly-tracked channel ... NOT a full aggregate rebuild."*
- **Full recompute exists but only runs once, at startup**: `resubscribe` is called from `lifecycle.rs:207`, inside the one-time startup sequence — never from a leave/end RPC.
- **Leave path never touches subscriptions**: `rpc_channels_leave` / `rpc_channels_switch` (`src/daemon/server/channel_membership_rpc.rs:140-229`) call `remove_member_confirmed` (relay membership) and `leave_session_channel` (local DB row) — neither calls `unsubscribe`/`close_subs`.
- **Teardown machinery is dead code**: `close_subs` (`subscriptions.rs:87-95`, verified) is `#[allow(dead_code)]`, doc'd *"Best-effort per id"*, zero non-test callers. `SubscriptionRegistry::compact` (`src/fabric/subscriptions.rs:243-244`) is likewise `#[allow(dead_code)]`: *"Not yet wired into the daemon — compaction at a quiet boundary is a future optimization."* There is no `remove_channel`.
- **Self-documented history of this exact class of leak**: `src/transport.rs:293-298` — *"random ids leaked an unbounded number of subscriptions into the relay pool ... growing the process to tens of GB over a day"* and `:369-374` — *"The codebase had no unsubscribe path before ... The registry needs this to compact narrow REQs and retire stale subscriptions ... instead of letting the relay-side subscription set grow monotonically."*

**Verdict**: subscriptions structurally outlive the state that justified them for the life of the daemon process. Dedup (deterministic `SubscriptionId`, `transport.rs:290-298`) prevents *duplicate* leaks but there's no shrink/close path at all.

## 2. SESSION LIFECYCLE — DB cleanup ordered/awaited; relay cleanup best-effort

- **Liveness**: `watch_pid` polled each heartbeat tick (default 30s) — `src/runtime.rs:277-278`: `if let Some(pid) = p.watch_pid { if !pid_alive(pid) { break; } }`; `pid_alive` = `nix::sys::signal::kill(pid, None).is_ok()` (`engine_lifecycle.rs:273-275`).
- **Teardown order** (`engine_lifecycle.rs:41-61`, inside a `tokio::spawn`): `remove_session_memberships` → `release_session_signer` → `mark_identity_dead_for_session` → remove from sessions map → `prune_hosted`. Ordered, but the enclosing task itself is fire-and-forget relative to whoever spawned it.
- **Relay-side membership removal is explicitly best-effort and silently retains stale state on failure**: `membership_cleanup.rs:38-61` spawns a detached task per channel: *"membership cleanup: relay removal was not confirmed; local membership row retained"* (`:51-58`).
- **Status (kind:30315) is never actively deleted** — no NIP-09 kind:5 anywhere in `src/`. `session_end.rs:30`: *"Mark the canonical session dead ... Its final published kind:30315 ages off via NIP-40 expiration."* TTL constants: `STATUS_TTL_SECS = 90`, `HEARTBEAT_SECS = 30` (`src/domain.rs:14,17`).
- **Crash safety net**: `reconcile_sessions` (`engine_lifecycle.rs:94-236`) at daemon restart re-derives session truth; `docs/daemon-design.md:333-346` states *"Without this, `who` and routing membership would lie after every daemon restart."* — an implicit admission that the live-path cleanup alone isn't trustworthy.

**Verdict**: local DB state cleanup is deterministic; relay-membership and subscription cleanup are not — best-effort, non-atomic, non-retried beyond a log warning.

## 3. PRESENCE / PEER DIRECTORY — expiry-gated reads, no active pruning of subscriptions/rows

- No dedicated peer table gets deleted; liveness = `expiration >= now` read filter (`src/state/status.rs:91-101`, NIP-40). Pruner (`background/pruner.rs:1-11`) ticks every 30s but only maintains an **in-memory** map to emit tail `Leave` events; comment (`:38-42`): *"there is nothing to manually prune — expired rows simply read as not-live."*
- **`relay_status` DB rows themselves are never purged** — `state/retention.rs:20-51` prunes `relay_events`, `inbox`, `outbox` only. Unbounded row growth over time (storage leak, not a correctness leak).
- **Late-status false-prune risk is real and structural**: liveness is `now <= last_publish + 90s`. Outbox drainer retries every 2s with no hard bound (`status_publish.rs:125-170`); if relay connectivity stalls >90s, a genuinely-alive session reads as dead until the next successful publish. Compounded by `status_publish.rs:86-89` skipping idle-and-locally-stale sessions from the heartbeat loop.

## 4. REFCOUNTING — none exists

`grep -rn "refcount\|ref_count" src/` → zero hits (confirmed pattern absent). Membership is tracked only via row-existence: `session_channels` (session→channel rows) and `relay_channel_members` (NIP-29 mirror, per-pubkey). `build_entity_coverage` (`subscriptions.rs:107-158`) is the *only* function that scans all sessions to derive "who still needs this channel," and it's called solely from the startup-only `resubscribe` — never on join/leave. So: shared subscriptions dedupe correctly on open (via deterministic IDs) but there is no shared-ownership counter and no close-on-last-leave — the equivalent of a refcount that only ever increments, never decrements, and is only read once (at boot).

## 5. Admitted drift/leak/race — verbatim

- `transport.rs:293-298` — leaked-subscriptions-to-tens-of-GB historical bug (see §1).
- `subscriptions.rs:87` — `close_subs`: *"Best-effort per id."* (dead code).
- `subscriptions.rs:69-83` — `replay_channel_chat`: *"Best-effort: a replay failure just means the session relies on subsequent live chat."* / *"channel chat replay timed out (best-effort)"*.
- `membership_cleanup.rs:51-58` — *"relay removal was not confirmed; local membership row retained"* (fire-and-forget spawn).
- `lifecycle.rs:113` — *"publishers/subscribers are best-effort and queue"*; `lifecycle.rs:150` — *"Best-effort: failure deferred to next restart."*
- `channels_rpc.rs:15` — *"Best-effort and fail-open — a degraded relay leaves the session..."*
- `session_start.rs:318` — *"so @te_session is never left stale. Best-effort, off the store lock."*
- `engine_lifecycle.rs:52-56` — *"engine exit: failed to mark identity dead; `who` may show a ghost"*; `:113-118,166-175` — *"reconcile GC: failed to mark dead session dead; ghost-alive row may remain"*.
- `tmux/delivery.rs:245` — *"ring_doorbells: clear_tmux_pane failed — stale dead-pane endpoint retained"*.
- **Postmortem** `docs/wiki/episodes/2026-06-29-2-daemon-cleanup-lock-file-deletion-caused.md`: lock-file deletion on shutdown caused two daemons to race and fight over `state.db` — fixed, now `lifecycle.rs:238-244`: *"Do NOT remove the lock file here — deleting it while the flock is still held lets a racing spawner open a new file (different inode) and acquire an independent lock, causing two daemons to overlap."*
- **Postmortem** `docs/wiki/episodes/2026-06-29-1-daemon-inhibit-fail-open-path-produces.md`: fail-open sentinel not consumed safely, produces spurious errors every hook run while inhibited — explicitly left unfixed ("user explicitly deferred").

## Summary table

| Resource | Model | Teardown on last-leave |
|---|---|---|
| Relay subscriptions | Incremental-add; full recompute exists but startup-only | None — dead code (`close_subs`, `compact`) |
| Sessions (local DB) | Event-driven, ordered/awaited | Yes |
| Sessions (relay membership) | Event-driven | Fire-and-forget, silently retains stale row on failure |
| Presence (30315) | Pure TTL expiry at read time | Never actively closed (no kind:5); DB rows never pruned at all |
| Inbox | Imperative state machine | `pending` rows for a dead target_session are never pruned (only completed states, after 7 days) |
| Refcounting | Absent everywhere | Implicit set-membership tables exist but aren't consulted for subscription lifecycle |

**Relevance to your reconciliation-engine idea**: the codebase already contains, verbatim, the exact primitive you're proposing (`resubscribe`/`build_entity_coverage` = derived-set recompute; `compact`/`close_subs` = diff-based children-first teardown) but wired to run only once, at startup, with the incremental live-path deliberately kept separate ("NOT a full aggregate rebuild," `subscriptions.rs:13`) for latency reasons on hook-critical paths. The gap is that nothing periodically (or on scope-close) re-runs that recompute-and-diff to retire subscriptions/membership/pending-inbox rows tied to channels/sessions that no longer exist — which is precisely the drift class documented in `transport.rs:293-298` and structurally still open on the leave/session-end paths today.
