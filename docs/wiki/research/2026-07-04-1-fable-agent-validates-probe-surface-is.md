---
type: research-record
date: 2026-07-04
session: 065295ad-311d-4965-a3c1-6f749135f2b8
transcript: /Users/pablofernandez/.claude/projects/-Users-pablofernandez-src-trellis/065295ad-311d-4965-a3c1-6f749135f2b8.jsonl
source_lines: 5113-5141
agent_attribution: a7c57653cbca398f6
has_preregistered_criteria: true
has_method: true
has_structured_report: true
characterization: "Fable agent validates probe surface is real/useful/complete via independent fakery checks, test runs, and source verification; verdict: SOLID with honest gaps and three must-fixes"
captured_at: 2026-07-04T18:01:13Z
---

Fable agent validates probe surface is real/useful/complete via independent fakery checks, test runs, and source verification; verdict: SOLID with honest gaps and three must-fixes

---

User: [Agent task result: Agent "Fable validation: is the probe surface real?" finished]
# Verdict: SOLID, ships with honest gaps — real, live-wired, and honestly rendered; not yet committed, and the daemon-glue last inch is untested

## What I verified (not trusted)

**Compiles and passes.** `cargo build --lib` exit 0. `cargo test --lib`: **456 passed, 0 failed**. Probe-filtered: **25 passed** (reconciler probes, daemon verb impls, CLI renders, ledger store).

**REAL — the three fakery checks all came back clean:**

1. **The keystone test is genuinely strong, not a tautology.** `reconcile::status::probe::tests::preview_applies_nothing_to_the_live_graph` (`src/reconcile/status/probe.rs:195`) proves non-mutation *three independent ways*: (a) revision unchanged, (b) oracle still green, (c) — the clever one — it previews the *original* activity afterwards and asserts it dedups to an empty plan, which is impossible if the first preview had leaked into the graph. Then it lands a *real* `on_distill` and asserts exactly `rev0 + 1` and one publish. That's a semantic proof, not a mock.
2. **`tx.preview()` is genuinely merged upstream and genuinely skips the swap.** The Cargo.toml pins trellis rev `60e759d` = "Add Transaction::preview(): dry-run a transaction without committing (#164)". I read the checkout: `preview(mut self)` runs the shared `run_pipeline()` on `self.working` and returns without `std::mem::swap`. `preview_stage` in tenex-edge ends in `tx.preview()`, not `commit()` (`status/probe.rs:95`).
3. **It's wired to the LIVE graphs, not rebuilt fakes.** The engine that drives real status publishes receives `state.status.clone()` (`daemon/server/engine_lifecycle.rs:40-43`) — the *same* `Arc<Mutex<StatusReconciler>>` that `probe oracle/simulate/why/state` lock (`daemon/server/probe/*.rs`). All runtime status triggers go through `status_seam::drive` (`runtime.rs:206,247,282,302,308`), and all sub mutations go through `sync_subscriptions` (`daemon/server/subscriptions.rs:49`; `session_start.rs` only reads). The ledger is recorded at all three real seams: `status_seam.rs:57`, `subscriptions.rs:65`, and both hook render paths (`turn_context/start.rs:235`, `check.rs:107`) — including no-ops. Labels are recorded at node creation on every surface. `trellis_commits` exists in `schema.rs:251` with an index.

**Renders are honest.** `render_oracle` prints `surface-correctness: NOT proven (oracle checks the graph's bookkeeping, not host effects)` plus `uncovered: turn_lifecycle, cursor, session_start, outbox`, and hook_context as "not a live graph (advisory)" — and there's a test locking that in (`oracle_render_is_honest_about_correctness`). Simulate prints "(nothing is applied)" and "NO CHANGE (deduped — no publish)"; why prints the latest-per-key footer; subscriptions-simulate returns `implemented: false` with a v2 message rather than silently missing. I found no overclaims in the output surfaces.

**USEFUL — yes, at 2am these are the right verbs.** "Is my live state provably self-consistent?" → `oracle`. "Why is this REQ still open?" → `why sub:<chan>` with owners+refcount from the live scope inventory. "What would this distill do — will it publish or dedup?" → `simulate`. "What has Trellis been suppressing?" → `stats` noop column. The wall you'd hit: no history (`why` is latest-per-key only, which it admits), no `probe why` for an *event id*, nothing for the four imperative surfaces (which it names).

## The sharpest things against

1. **It's uncommitted.** 25 modified/untracked files sitting in the `trellis-probe` worktree working tree; `master..HEAD` contains none of this. It cannot be called shipped until it's at least committed.
2. **The daemon glue is only compile-verified.** No test drives `rpc_probe` → `oracle_value(state)`/`why_value`/`simulate_value` through an actual `DaemonState`. The oracle test builds its own reconcilers and mirrors `check()` — reasonable, and the reconcilers are really driven, but the lock/param/dispatch inch (and the whole CLI→daemon RPC path) has never executed under test or, in this validation, against a running daemon.
3. **Real doc bug:** `probe simulate --now` is documented "Wall-clock millis" (`cli/probe/mod.rs:61`) but the reconciler expects unix *seconds* (`mutate`/`preview_stage` compute `now / refresh_secs`). Passing millis would fabricate a spurious TTL-refresh in the previewed plan. Also minor: `oracle --now` is an accepted no-op flag, and `stats --surface bogus` returns a zeroed row instead of erroring.

**Must-fix before "real":** commit the work; fix the `--now` units doc; ideally one smoke test through `rpc_probe` with a constructed `DaemonState`.

## The blog claim

*"Point the check that proves the engine honest at the running system, ask any event why, and simulate a change before it happens"* — **two-thirds fully earned, one-third needs a word changed.** "Check the engine honest at the running system": earned — the oracle runs `assert_incremental_equals_full` on the actual daemon-held graphs, and (crucially) the tool itself disclaims what that doesn't prove. "Simulate before it happens": earned — real `tx.preview()` upstream, non-mutation proven adversarially. "Ask **any event** why": overclaimed — you can ask any live *sub/status handle* why its *latest* change happened; not arbitrary events, not history, not hook_context, not the four imperative surfaces. Write "ask any live surface why" and the claim is fully honest — which would match the tool, because the tool already is.

