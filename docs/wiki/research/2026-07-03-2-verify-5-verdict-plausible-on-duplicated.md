---
type: research-record
date: 2026-07-03
session: c7805f5d-42c5-44b6-8eaa-ecd2453ed822
transcript: /Users/pablofernandez/.claude/projects/-Users-pablofernandez-src-trellis/c7805f5d-42c5-44b6-8eaa-ecd2453ed822.jsonl
source_lines: 1384-1401
agent_attribution: verify-5
has_preregistered_criteria: true
has_method: true
has_structured_report: true
characterization: "Verify-5 verdict PLAUSIBLE on duplicated legacy_relay_hint state — confirmed redundant field and dead accessor, but divergence is latent not live since no mutation path exists"
captured_at: 2026-07-03T09:42:55Z
---

Verify-5 verdict PLAUSIBLE on duplicated legacy_relay_hint state — confirmed redundant field and dead accessor, but divergence is latent not live since no mutation path exists

---

<agent-message from="verify-5">
VERDICT: PLAUSIBLE (combined candidate; the dead-accessor half is definitively true, the divergence-risk half is only hypothetical).

(a) Both copies exist and are populated in both constructors — CONFIRMED.
- Handle field: `nip60_wallet.rs:84  legacy_relay_hint: Vec<String>,`
- create_new: `nip60_wallet.rs:112  legacy_relay_hint: relays,` while config gets the same vec via `WalletConfig::generate(vec![mint_url...], relays.clone())` (line 101).
- from_wallet_event: `nip60_wallet.rs:130  let legacy_relay_hint = config.legacy_relay_hint.clone();` then `:136  legacy_relay_hint,`. Both places seeded from the identical source.

(b) Mutation path after construction — NONE EXISTS. The only `config.lock()` sites are `nutzap_send.rs:81` and `nutzap_receive.rs:28`, and both immediately `.clone()` for read-only use; neither mutates `config.legacy_relay_hint`. `build_wallet_event` is called only in `create_new` (construction) and one test — there is no update_wallet / event-rebuild flow. So divergence between the two copies is purely hypothetical today. Per your own criterion this pushes to PLAUSIBLE, not CONFIRMED.

(c) Zero callers of the accessor — CONFIRMED. `grep "legacy_relay_hint()"` returns no call sites anywhere in the workspace; every `Self::legacy_relay_hint` hit (lib.rs:26, mint_announce.rs:15, nutzap_send.rs:75) is a doc-comment rustdoc link, not a call. The two real reads use the field directly, not the accessor.

(d) The two divergent read sites — quoted:
- publish_nutzap_info reads the HANDLE copy: `nip60_wallet/nutzap_send.rs:84  relays: self.legacy_relay_hint.clone(),`
- build_wallet_event reads the CONFIG copy: `wallet_event.rs:88  for relay in &config.legacy_relay_hint {`

Summary: The redundant field and the two independent read sites are real, so the simplification (collapse to a single source, likely reading through `config`) is legitimate. But because no code path ever mutates `config.legacy_relay_hint` post-construction, the two copies cannot actually diverge today — the bug is latent, not live. The unused `legacy_relay_hint()` accessor (line 223) is genuinely dead code and safe to drop. Net: a valid cleanup, not a live bug → PLAUSIBLE.
</agent-message>
