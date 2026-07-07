---
type: research-record
date: 2026-07-03
session: c7805f5d-42c5-44b6-8eaa-ecd2453ed822
transcript: /Users/pablofernandez/.claude/projects/-Users-pablofernandez-src-trellis/c7805f5d-42c5-44b6-8eaa-ecd2453ed822.jsonl
source_lines: 428-470
agent_attribution: fable-reviewer
has_preregistered_criteria: true
has_method: true
has_structured_report: true
characterization: "Code-verification review of X-Ray epic plan against Trellis source: 17 findings (4 blockers, 8 should-fix, 5 consider) with overall verdict that phase ordering is sound but Phase E over-relies on trace foundations"
captured_at: 2026-07-03T08:38:21Z
---

Code-verification review of X-Ray epic plan against Trellis source: 17 findings (4 blockers, 8 should-fix, 5 consider) with overall verdict that phase ordering is sound but Phase E over-relies on trace foundations

---

# Review: X-Ray diagnostic epic (NMP#2858)

I verified the epic's claims against the actual Trellis code (`trace.rs`, `serialized.rs`, `scenario.rs`, `resource_key.rs`, `transaction_types.rs`). The structural claims hold: `TransactionTrace` is a payload-free, serde-capable projection with ordered resource commands, coalescences, output frames, scope events, audit log, and phase trace; `SerializedScenario` carries `TRACE_FORMAT_VERSION`; redaction hooks exist. But three of the foundations are weaker than the epic's Phase E text assumes, and one phase dependency runs backwards.

## Findings

**1. [Blocker — Phase E] The Trellis redaction hook leaks resource keys through the audit log.** `redact_trace` (crates/trellis-testing/src/scenario.rs:280-292) redacts `resource_commands`, `resource_coalescences`, and invariant names — but not `trace.audit_log`, and `AuditEvent::ResourceOpenCoalesced` carries a full `ResourceKey` (crates/trellis-core/src/transaction_types.rs:87-94). The epic cites these hooks as the redaction foundation; as implemented, a "shareable" capsule would ship un-redacted keys. Suggested change: add a Phase E checklist item — "upstream trellis fix: `redact_trace` must cover `audit_log` (and any future key-carrying fields); add a test that serializes a redacted capsule and asserts no original key material survives anywhere in the JSON." The "all prerequisites closed" claim is true of the listed issues, but this gap is new.

**2. [Blocker — Phase E] "Replay from capsule alone" conflates scrubbing with re-execution.** `TransactionTrace` records what happened, payload-free; you cannot re-execute from traces. Re-execution requires the `DataTransactionScript` (operations *with payloads* — follow lists, filters), plus the same NMP version constructing the identical graph topology, because NodeIds are positional. Recording the script blows up the privacy surface far beyond the payload-free trace. Suggested change: define capsule v1 explicitly as **trace + receipts, scrub/reconstruction only**; document `replay_session(trace)` as reconstruction, not re-execution; list re-executable capsules (script capture) as an explicit non-goal or later phase with its own redaction story.

**3. [Blocker — Phase E, affects B] Traces are keyed by opaque NodeId/ScopeId — the capsule needs a symbolication manifest.** `changed_inputs`, `dirty_roots`, recompute lists, and most `AuditEvent`s are bare `NodeId`s. Without a NodeId/ScopeId → NMP-semantic-label registry captured at record time, a capsule replayed by an agent that never saw the session is unreadable, and cause chains can't be reconstructed. Suggested change: Phase E item — "capsule embeds a node/scope registry captured at record time; registry labels pass through the same local/shareable redaction modes."

**4. [Blocker — Phase A/D sequencing] The lint exemption can't be finalized before the Phase D transport is decided.** Phase D leaves "declared projection **or** devtools channel" open. If receipts ride the normal FlatBuffer projection pipeline, the receipt schema becomes part of the (dev-build) app surface and the Phase A lint/ratchet wording must account for that; if it's a side channel, the exemption is purely dependency-graph-shaped. Suggested change: move the transport decision into Phase A / #2809 as a decision-gate item.

**5. [Should-fix — Phase A] "Compiled out of release builds" needs a mechanical definition and CI enforcement.** `cfg(debug_assertions)` is the wrong tool (release-with-debug builds exist; per-platform build configs differ). Suggested wording: nmp-devtools is a separate crate behind an off-by-default cargo feature; CI asserts each platform's release artifact dependency graph (`cargo tree`) and/or symbol table contains no nmp-devtools; the ratchet rule is dependency-graph-based — an allowlist file of crates permitted to depend on `trellis-*` (existing private adapters + nmp-devtools), where adding a line is the ratchet violation.

**6. [Should-fix — Phase A/B] Type lints can't catch string leakage, and Phase B's "reason string" is exactly that hole.** Debug-formatting a `ResourcePlan` or `ResourceKey` into a reason string leaks Trellis vocabulary invisibly to any type-based gate. Suggested change: replace "reason string" with structured reason codes (NMP enum + typed params, rendered to text at the edge), plus a snapshot/regex test that rendered receipts contain no `trellis`/`ResourcePlan`/`NodeId(`-pattern substrings.

**7. [Should-fix — Phase B] The receipt shape is missing fields its own cause-chain examples require.** The shape bullet (projection key, view/scope label, interest shape, owner key, owner count, relay rows, reason string) omits: transaction id + revision + sequence number (the first Phase B bullet promises them — unify); wall-clock timestamp (`TransactionTrace` has none; NMP must stamp it); command kind (open/replace/refresh/close); outcome/status (a CLOSE that failed at the socket looks identical to one that succeeded); owner-count **transition** (the epic's own example is "refcount 2→1", but the shape only has a current count); parent-scope reference (teardown cascades ordered "children first" need hierarchy); and a structured cause/parent link for `why_subscription_open` to return. Also flag "interest shape" explicitly as the primary privacy-bearing field — it's a Nostr filter containing pubkeys/event ids, and Trellis redaction never sees it.

**8. [Should-fix — Phase E] Redaction is mostly NMP-side work; the epic implies Trellis provides it.** `TraceRedactor` covers only step names, `ResourceKey`s, and invariant names. Pubkeys, relay URLs, and event ids live in NMP receipt fields and in ResourceKey *segments* (NMP taxonomy). Shareable mode also needs **consistent pseudonymization** — same key → same token capsule-wide, preserving the type segment (e.g. keep segments[0], hash the rest) — or refcount/causality analysis breaks after redaction. Suggested change: add "NMP-owned redactor over the receipt shape + ResourceKey segments, with capsule-wide stable pseudonyms" as an explicit deliverable.

**9. [Should-fix — Phase E] Versioning is exact-match, single-layer; the capsule has two layers.** `validate_format_version` (serialized.rs:193-202) hard-fails on anything but equality — "graceful" only in that it's a typed error, not migration. Every Trellis format bump orphans all outstanding capsules attached to issues, and the capsule also contains the NMP receipt/correlation shape, which versions independently. Suggested change: define a capsule **envelope** with its own version plus embedded `TRACE_FORMAT_VERSION`, NMP receipt schema version, and producer metadata (app version, NMP/Trellis versions, platform, recording window, redaction-mode marker); state a compat policy (reader supports N-1, or degrades to raw-JSON view Flight-Recorder-style instead of refusing).

**10. [Should-fix — Phase C] The exit criterion is untestable as written and contradicts Phase B.** "An agent can answer 'why is my feed empty?' from receipts alone" has no failure corpus, no definition of a correct answer, and "real relays" is non-reproducible. Worse, the most common real cause — subscription opened fine, relay returned zero events — is *not* answerable from reconciliation receipts alone; it needs the Phase B relay-diagnostics join. Suggested change: define N seeded failure scenarios against a local relay fixture (empty-source fail-closed; scope teardown closed a subscription it shouldn't; refcount held a subscription alive; subscription open but relay never connected; subscription open, EOSE, zero matching events), each with an expected root cause; CI runs a scripted MCP call sequence (or an agent) and checks root-cause identification. Deliberately include the zero-events case to pin down that "receipts" includes the relay-diagnostic join.

**11. [Should-fix — Phase B] No overhead gating workstream.** `TransactionTrace::from_result` (trace.rs:54-89) deep-clones essentially the whole transaction result; a scrolling feed commits transactions at high frequency, so always-on recording in dev builds distorts the very behavior being debugged. Suggested change: Phase B item — recording is opt-in at runtime (disabled ⇒ zero clones, not "clone and drop"), with a measured per-transaction overhead budget and a benchmark gate.

**12. [Should-fix — Phase C] Hidden dependency: a headless NMP host runner.** "Headless CLI driving a real Chirp session" presumes the Rust kernel + feed-session can run without a platform shell (signer, storage, network capabilities included). That harness isn't itemized anywhere and is plausibly the largest single work item in the phase. Add it explicitly to Phase C.

**13. [Consider — Phase C] `replay_session(trace)` is in the Phase C MCP list but capsules don't exist until Phase E.** Either mark it as a Phase C stub excluded from acceptance, or move it to Phase E's deliverables.

**14. [Consider — Phase D/E] State that the pane scrubs *translated* receipts, not raw capsules.** In Phase E the X-Ray pane scrubs capsules; if it parses Trellis trace JSON directly, the epic's own invariant ("Chirp never parses Trellis keys") breaks. Add one sentence: capsule scrubbing in the pane goes through the same NMP receipt translation as the live stream.

**15. [Consider — Phase E] Residual privacy risk in shareable mode.** Even fully pseudonymized, interest-set sizes, per-relay subscription counts, and transaction timing fingerprint a user (follow count is basically recoverable). Note the residual risk in shareable-mode docs; consider bucketing counts in shareable exports.

**16. [Consider — Phase E] Ring buffer needs a byte bound, not just a count.** One teardown-cascade transaction can be enormous (the trace carries every command, frame, and audit entry). Bound by bytes and count.

**17. [Consider — hygiene] Update chirp#30 to strike v0.** The epic declares shadow X-Ray obsolete post-#2808, but chirp#30 still stages it as "buildable now." Edit the driving issue so the two documents agree on the starting point.

## Verdict

The epic is well-sequenced at the phase level — ADR gate first, NMP-owned receipt shape before any consumer, agent-usable CLI/MCP before UI is the right order, and its claims about what Trellis provides are accurate at the structural level. The weakness is concentrated in Phase E, which leans on "redaction hooks exist" and "graceful versioning" harder than the code supports: the redactor demonstrably leaks resource keys via the audit log, versioning is exact-match single-layer, traces are unreadable without a symbolication manifest the epic never mentions, and "replay from capsule alone" is ambiguous between scrubbing (achievable) and re-execution (not, without recording payloads). Phase A also has a hidden back-edge: the lint exemption can't be worded precisely until the Phase D transport decision is made, so pull that decision forward. None of this requires reordering phases; fix findings 1–4 in the epic text before Phase B coding starts and the plan is sound.
