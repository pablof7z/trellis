---
type: research-record
date: 2026-07-03
session: c7805f5d-42c5-44b6-8eaa-ecd2453ed822
transcript: /Users/pablofernandez/.claude/projects/-Users-pablofernandez-src-trellis/c7805f5d-42c5-44b6-8eaa-ecd2453ed822.jsonl
source_lines: 1531-1556
agent_attribution: verify-7
has_preregistered_criteria: true
has_method: true
has_structured_report: true
characterization: "Verify-7 verdict PLAUSIBLE on ureq gating inconsistency — confirmed ureq unconditional vs siblings' optional native feature, but build-cost rationale refuted since other workspace members already compile ureq+TLS"
captured_at: 2026-07-03T09:42:55Z
---

Verify-7 verdict PLAUSIBLE on ureq gating inconsistency — confirmed ureq unconditional vs siblings' optional native feature, but build-cost rationale refuted since other workspace members already compile ureq+TLS

---

User: Another Claude session sent a message:
<agent-message from="verify-7">
VERDICT: PLAUSIBLE — the gating-inconsistency fact is real, but the "makes every workspace/CI build compile ureq" build-cost rationale is materially wrong.

(a) ureq IS unconditional in nmp-nip60/Cargo.toml. It has NO [features] section at all, and:
    `ureq = { version = "2", default-features = false, features = ["tls", "json"] }`  (no `optional = true`). CONFIRMED.

(b) Siblings DO gate ureq behind an optional `native` feature. CONFIRMED:
  nmp-nip57/Cargo.toml:
    `[features]` / `native = ["dep:ureq"]`
    `ureq = { version = "2", optional = true, default-features = false, features = ["tls", "json"] }`
  nmp-nip05/Cargo.toml:
    `native = ["dep:ureq"]`
    `ureq = { version = "2", optional = true, default-features = false, features = ["tls", "json"] }`

(c) BUT ureq+TLS is ALREADY compiled unconditionally in every workspace/CI build via other always-built workspace members — this is the flaw in the finding's marginal-cost argument:
  nmp-blossom/Cargo.toml:26 (workspace member, Cargo.toml:8):
    `ureq = { version = "2", default-features = false, features = ["tls"] }`  (unconditional, no optional)
  nmp-nip11/Cargo.toml:19 (workspace member, Cargo.toml:32):
    `ureq = { version = "2", default-features = false, features = ["tls"] }`  (unconditional, no optional)
  ureq appears once in Cargo.lock. So the workspace build already links ureq+TLS regardless of nmp-nip60; adding it does NOT newly introduce the TLS stack. The only marginal delta is ureq's `json` feature (minor serde glue) — not the "TLS HTTP client" cost the finding implies.

(d) The ~400-line Cashu mint client is unconditional today: `crates/nmp-nip60/src/lib.rs:60` declares `pub mod cashu;` with no cfg gate, and `src/cashu/client.rs` contains only `#[cfg(test)]` (line 394) — no feature/cfg guarding the client itself.

Net: gating ureq behind a `native` feature to match nip57/nip05 is a legitimate consistency improvement (b + a + d hold), so the finding isn't wrong to flag it. But its stated justification — that re-adding nmp-nip60 forces the workspace to compile ureq+TLS — is refuted by (c): two other always-built members (nmp-blossom, nmp-nip11) already do. Hence PLAUSIBLE, not CONFIRMED. Also note the finding's paraphrase of the crate description ("NIP mechanics only — event codecs, types, pure shape validation") is trimmed; the actual manifest description explicitly covers "Cashu mint API calls" and only says backend selection/journal live in nmp-wallet, so it doesn't claim to be HTTP-free.
</agent-message>
