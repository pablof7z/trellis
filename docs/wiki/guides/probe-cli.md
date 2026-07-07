---
title: Probe CLI Verbs
slug: probe-cli
topic: trellis-adoption
summary: The probe CLI verb family is named 'probe', not 'trellis', to keep the vocabulary boundary honest so the command names describe intent rather than substrate
tags:
  - capture
volatility: warm
confidence: medium
created: 2026-07-04
updated: 2026-07-04
verified: 2026-07-04
compiled-from: conversation
sources:
  - session:065295ad-311d-4965-a3c1-6f749135f2b8
---

# Probe CLI Verbs

## Naming & Visibility

The probe CLI verb family is named 'probe', not 'trellis', to keep the vocabulary boundary honest so the command names describe intent rather than substrate. The probe CLI is invoked as `tenex-edge probe <verb>` with `--json` as a first-class contract, hidden like the debug command, treated as a machine contract, backed by daemon-resident RPC methods on the existing dispatch table. Every probe verb emits a versioned envelope with schema, verb, ok, data, and evidence fields carrying stable addressable handles.

<!-- citations: [^06529-a4735] [^06529-d2108] -->
## Oracle

Probe oracle runs assert_incremental_equals_full() against live daemon-held graphs — the graph is cloned under lock and the check runs off-lock — sampled per-N-commits, logged to an oracle_checks ledger, with red rows surfaced as one-line alerts in the fabric snapshot agents already read each turn. Probe oracle output prints `oracle: green` / `surface-correctness: NOT proven (oracle checks the graph's bookkeeping, not host effects)` and lists uncovered mutator names — oracle-green does not equal surface-correct. The honest render text is locked in by the test oracle_render_is_honest_about_correctness.

<!-- citations: [^06529-b176b] [^06529-95f37] -->
## Stats

Probe stats reports per-surface commit counts from the trellis_commits ledger: effectful commits, no-ops, suppressed publishes, command counts, output frame counts, duration microseconds, and graph node counts. The probe stats CLI verb takes --surface (status|subscriptions|hook_context) and --since (unix-millis), omitting --surface for all surfaces, and renders a per-surface table with commits, effectful, noop, cmds, frames, dur_us, and nodes columns. Per-surface counters include status commits/published/suppressed split by trigger, refresh-vs-replace ratio, subscription opens/closes/coalesced-joins/live-now/max-live, hook renders/frames-emitted/unchanged, and per-commit duration histograms with graph node counts. The value numbers in probe stats (e.g. 'avoided publishes') are modeled against a reconstructed baseline, not measured — the old five-trigger path is deleted — and must be labeled as modeled in the output.

<!-- citations: [^06529-d3e6c] [^06529-0788b] -->
## Replay

Probe replay rebuilds a fresh reconciler from a recorded input capsule and asserts trace equality via trellis-testing's TrellisHarness::replay_data with DataTransactionScript<InputFact>, giving assert_replay_matches, ResourceLedger (no orphan/no dup-close/forbidden-key), and OutputLedger (revision-monotonic) invariants. Probe replay --export-trace writes a SerializedScenario JSON that loads directly into the existing Trellis Flight Recorder workstation — no second viewer is built.

<!-- citations: [^06529-6b2b9] [^06529-53f53] -->
## Diff

Probe diff replays a capsule twice — once verbatim, once with a single drive-arg mutated — and reports the per-step decision diff (commands emitted/suppressed, frame kinds, receipt deltas) as a ± table or JSON, leveraging that Trellis returns plans as data.

<!-- citations: [^06529-f8227] [^06529-4cc10] -->
## Acid (Counterfactual Verification)

Probe acid takes an artifact handle, produces its why-chain from receipts and live audit, then verifies the explanation counterfactually: removing each claimed cause input must change the artifact, and mutating a non-cause must leave it byte-identical — output is CONFIRMED or FALSIFIED.

<!-- citations: [^06529-cb55d] [^06529-dbf97] -->
## Simulate

tx.preview() runs the full commit pipeline on the working clone and returns the TransactionResult without writing back to the real graph — the clone is already paid at begin_transaction, making dry-run nearly free. tx.preview() is implemented in trellis-core by extracting the shared pre-swap pipeline into a private run_pipeline method; preview calls run_pipeline and returns the result without the swap or scope reclaim. tx.preview() is merged into pablof7z/trellis master (squash 60e759d, PR #164) with CI green (format/lint/test/docs/bench + proptest/fuzz). Probe simulate stages a candidate InputFact, calls tx.preview(), and returns the plan/frames/receipt as JSON without touching the live relay — a Terraform-plan for a status or subscription change. A test proves the graph is untouched after the preview. The probe simulate --now parameter expects unix seconds (not millis) because the TTL re-arm bucket is computed as now / refresh_secs; passing millis would fabricate a spurious TTL-refresh in the previewed plan.

<!-- citations: [^06529-b640a] [^06529-db359] -->
## Implementation Status

The probe surface is committed as PR #241 on pablof7z/tenex-edge and includes probe stats, oracle, simulate, why, and state verbs with 458 lib tests passing. Probe oracle runs assert_incremental_equals_full() against the live daemon-held graphs (cloned under lock, checked off-lock). Probe simulate dry-runs a fact via tx.preview() with a test proving the graph is untouched after the preview. An end-to-end rpc_probe daemon smoke test (rpc_probe_reflects_driven_state_for_every_verb) exercises all five probe verbs through a constructed DaemonState with live reconcilers and the store, verifying oracle, stats, simulate, why, and state responses. The Fable validation verdict on the probe surface is: SOLID — real, live-wired, and honestly rendered, with all three fakery checks clean (preview non-mutation proven adversarially, tx.preview() genuinely merged and skips the swap, wired to the same live reconcilers the daemon drives).

<!-- citations: [^06529-df3d8] [^06529-c1222] -->
## Why

Probe why queries the live in-process audit API (why_resource_command, why_output_frame, dependency_path) for the latest-per-key change to a surface handle. It prints that it answers latest-only and that history requires probe replay. <!-- [^06529-15ca4] -->

## Seams

Probe seams renders the authority-frontier table showing each surface's mode (authoritative/advisory/imperative) with live counters and uncovered mutator names. <!-- [^06529-36978] -->
