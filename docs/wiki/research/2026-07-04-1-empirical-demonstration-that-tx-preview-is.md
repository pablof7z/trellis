---
type: research-record
date: 2026-07-04
session: 065295ad-311d-4965-a3c1-6f749135f2b8
transcript: /Users/pablofernandez/.claude/projects/-Users-pablofernandez-src-trellis/065295ad-311d-4965-a3c1-6f749135f2b8.jsonl
source_lines: 4424-4457
agent_attribution: main
has_preregistered_criteria: true
has_method: true
has_structured_report: true
characterization: "Empirical demonstration that tx.preview() is a true dry-run on a subscription graph: preview returns same plan as commit, graph untouched, verdict PROVEN"
captured_at: 2026-07-04T12:49:23Z
---

Empirical demonstration that tx.preview() is a true dry-run on a subscription graph: preview returns same plan as commit, graph untouched, verdict PROVEN

---

Assistant: **Proven.** The dry-run semantics hold end-to-end on a realistic subscription-coverage graph (tenex-edge's actual use case), not just scalar nodes:

- `preview(drop random)` → returns `[Close(sub:random)]` — the exact plan a commit would produce.
- **Graph untouched by preview**: `random` still OPEN, revision still `Revision(1)`, input still reads `{general,random}`.
- A *different* candidate from the same live state — `preview(add ops)` → `[Open(sub:ops)]`, and `ops` is not opened, revision still 1. Previews are independent and mutate nothing.
- `commit(drop random)` → plan **equals** the earlier preview, and now `random` is actually CLOSED with the revision advanced.

That's the "what would committing this fact produce?" simulation working exactly as designed. Let me clean up the throwaway demo and the merged worktree, and sync master.

User: [Tool result]: chpwd:5: command not found: _iterm2_set_profile
demo removed
chpwd:5: command not found: _iterm2_set_profile
Deleted branch tx-preview (was 7a7fdc1).
   3c34fae..60e759d  master     -> origin/master
 crates/trellis-core/tests/transactions.rs | 88 +++++++++++++++++++++++++++++++
 2 files changed, 133 insertions(+), 5 deletions(-)
=== trellis master tip ===
60e759d Add Transaction::preview(): dry-run a transaction without committing (#164)
3c34fae Fix trace audit log redaction (#162)

Assistant: Done — merged, and verified it actually works.

**Merged:** `tx.preview()` is in `pablof7z/trellis` master (squash `60e759d`, PR #164), issue #163 auto-closed, CI was green (format/lint/test/docs/bench + proptest/fuzz). The main Trellis checkout is fast-forwarded to include it.

**Tested (beyond the shipped unit tests):** I wrote a fresh demonstration on a **resource-owning graph modeled on tenex-edge's real use case** — a set of channels driving one relay-subscription Open/Close each — and ran it. It proved all four properties of a true dry-run:

| Step | Result |
|---|---|
| `preview(drop "random")` | returns `[Close(sub:random)]` — the plan a commit *would* emit |
| graph after preview | **unchanged**: `random` still open, `Revision(1)` unchanged, input still `{general, random}` |
| `preview(add "ops")` from the *same* state | a *different* plan `[Open(sub:ops)]`; `ops` not opened, revision still 1 — previews are independent and apply nothing |
| `commit(drop "random")` | plan **equals** the earlier preview; `random` now actually CLOSED; revision advanced |

So `preview()` is genuinely the "what would committing this fact produce?" simulation — the plan you'd get, with zero mutation to the live graph — which is exactly what Phase 1's `probe simulate` and the safety invariant are built on. The throwaway demo and the merged worktree/branch are cleaned up.
