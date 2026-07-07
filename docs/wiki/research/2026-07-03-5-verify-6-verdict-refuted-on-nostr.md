---
type: research-record
date: 2026-07-03
session: c7805f5d-42c5-44b6-8eaa-ecd2453ed822
transcript: /Users/pablofernandez/.claude/projects/-Users-pablofernandez-src-trellis/c7805f5d-42c5-44b6-8eaa-ecd2453ed822.jsonl
source_lines: 1440-1472
agent_attribution: verify-6
has_preregistered_criteria: true
has_method: true
has_structured_report: true
characterization: "Verify-6 verdict REFUTED on nostr literal pin being a half-migration defect — explicit workspace policy boundary documented in Cargo.toml, ~10 crates share the same pin pattern"
captured_at: 2026-07-03T09:42:55Z
---

Verify-6 verdict REFUTED on nostr literal pin being a half-migration defect — explicit workspace policy boundary documented in Cargo.toml, ~10 crates share the same pin pattern

---

<agent-message from="verify-6">
VERDICT: REFUTED — crates/nmp-nip60/Cargo.toml leaving `nostr` as a literal pin is the documented, intended state, not a half-migration defect.

(a) Root Cargo.toml [workspace.dependencies] DOES contain nostr, but as a bare pin, not a migration target:
```
nostr = "0.44"
```
Above it, a deliberate policy comment (Cargo.toml lines ~117-142):
```
# TODO: `nostr` is left as a bare entry because its feature posture is split
#   across the workspace — most crates want defaults ON, `nmp-marmot` /
#   `nmp-nostr-lmdb` want `default-features = false, features = ["std"]`.
#   Switching the workspace entry ... deferred to keep this commit scoped.
...
# Mechanical workspace-owned dependency migration is complete for serde,
# serde_json, zeroize, rustls, tungstenite, and nmp-ownership; the
# doctrine_lint_smoke manifest ratchet rejects member manifests that
# reintroduce explicit version/path declarations for those deps. `nostr`
# remains the policy boundary above.
```
So nostr is EXPLICITLY out of scope for this PR; the manifest ratchet enforces workspace inheritance only for serde/serde_json/zeroize/rustls/tungstenite/nmp-ownership — NOT nostr.

(b) Literal nostr pins are common, not an nip60-only outlier. Across crates using nostr: ~16 inherit (`workspace = true`) — blossom, browser-runtime, content-fixtures, core, marmot, nip02, nip17, native-runtime, nip05, nip29, nip57, nip77, nip89, store, signers, wot — and ~10 still literally pin `version = "0.44"` — nip60, nip46, nip46-runtime, nip47, nip51, nip59, nip59-decrypt-only, nwc, testing, example-login-timeline. nip60 sits with roughly a third of the crates, e.g.:
```
crates/nmp-nip47/Cargo.toml:16:  nostr = { version = "0.44", features = ["std"] }
crates/nmp-nip46/Cargo.toml:15:  nostr = { version = "0.44", features = ["std", "nip04", "nip44"] }
crates/nmp-nip59/Cargo.toml:16:  nostr = { version = "0.44", default-features = false, features = ["nip44", "nip59"] }
```

(c) Feature composition: yes, `{ workspace = true, features = ["std","nip44","nip60"] }` WOULD compose correctly (Cargo features are additive, and nip60 wants defaults-on, matching the bare workspace posture). So it's technically migratable — but the maintainers chose not to, workspace-wide, and documented why. The "will drift on the next workspace nostr bump" concern applies equally to ~10 crates and is an accepted, tracked deferral, not a defect introduced by this PR.

The candidate's factual premise (serde/serde_json migrated to `workspace = true` while nostr stays pinned in nip60) is correct — confirmed at nmp-nip60/Cargo.toml lines 16-18 — but the "half-migration bug" framing is wrong: it's consistent with siblings and with an explicit in-repo policy boundary.
</agent-message>
