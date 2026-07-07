---
type: research-record
date: 2026-07-03
session: c7805f5d-42c5-44b6-8eaa-ecd2453ed822
transcript: /Users/pablofernandez/.claude/projects/-Users-pablofernandez-src-trellis/c7805f5d-42c5-44b6-8eaa-ecd2453ed822.jsonl
source_lines: 1367-1377
agent_attribution: verify-2
has_preregistered_criteria: true
has_method: true
has_structured_report: true
characterization: "Verification of build_wallet_event relay-tag write-path candidate; verdict: PLAUSIBLE (mechanism confirmed but dormant, design doc governs read authority only)"
captured_at: 2026-07-03T09:30:55Z
---

Verification of build_wallet_event relay-tag write-path candidate; verdict: PLAUSIBLE (mechanism confirmed but dormant, design doc governs read authority only)

---

<agent-message from="verify-2">
VERDICT: PLAUSIBLE — candidate crates/nmp-nip60/src/wallet_event.rs:88 (build_wallet_event still emits legacy 17375 relay tags)

Factual claims are all confirmed:
- `build_wallet_event` still writes relay tags: lines 87-90 `let mut tags = Vec::new(); for relay in &config.legacy_relay_hint { tags.push(Tag::custom(TagKind::custom("relay"), [relay.as_str()])); }`
- `generate` still takes the param: line 40 `pub fn generate(mint_urls: Vec<String>, legacy_relay_hint: Vec<String>) -> Self` (only renamed from `relays`, behavior unchanged).
- The write path is live via `Nip60WalletHandle::create_new` (nip60_wallet.rs:96-101), which passes its `relays` arg straight into `generate` → emitted as tags on every brand-new wallet.
- Confirmed there are no non-test callers of `create_new`/`generate` in-repo (nmp-wallet doesn't exist yet), so the write path is currently dormant.

But the "contradicts PR goal" framing overreaches, so not CONFIRMED. The design doc's governing sentence targets READ authority only: "Activation should remove that as the source of truth, or parse it only as a non-authoritative compatibility hint that cannot override kind:10019 or NIP-65 fallback." It offers two options; the PR picked option 2 (parse as hint) and delivered on it — renamed the field, demoted it in docs everywhere, and nothing reads it as authoritative. The doc says nothing prohibiting *emission*. Decode→rebuild preserving tags is legitimate round-trip fidelity, and for new wallets the caller controls what relays get written. So this is a real design tension (new wallets don't age the legacy surface out) but a defensible judgment call rather than a defect against a stated requirement — PLAUSIBLE.
</agent-message>
