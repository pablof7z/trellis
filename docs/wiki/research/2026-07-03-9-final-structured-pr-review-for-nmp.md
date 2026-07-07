---
type: research-record
date: 2026-07-03
session: c7805f5d-42c5-44b6-8eaa-ecd2453ed822
transcript: /Users/pablofernandez/.claude/projects/-Users-pablofernandez-src-trellis/c7805f5d-42c5-44b6-8eaa-ecd2453ed822.jsonl
source_lines: 1562-1582
agent_attribution: main
has_preregistered_criteria: true
has_method: true
has_structured_report: true
characterization: "Final structured PR review for NMP #2866 — 6 findings with verdicts (PLAUSIBLE/CONFIRMED), 4 refuted non-issues, overall verdict mergeable once finding 1 (kind:10019 relay seeding) is addressed"
captured_at: 2026-07-03T09:42:55Z
---

Final structured PR review for NMP #2866 — 6 findings with verdicts (PLAUSIBLE/CONFIRMED), 4 refuted non-issues, overall verdict mergeable once finding 1 (kind:10019 relay seeding) is addressed

---

Assistant: # Review: nostr-multi-platform#2866 — Reactivate nmp-nip60

The PR does what issue #2865 asked: returns `nmp-nip60` to workspace/CI membership (dropping the standalone `[workspace]` stub and inheriting workspace package fields), deletes the dead `WalletBackend` trait whose `pay_invoice` always failed, renames the kind:17375 relay data to `legacy_relay_hint` with non-authoritative framing throughout, and adds the ownership declaration and release classification the gates require. Eight finder angles and seven adversarial verifiers later: the mechanical work is solid — no serde/wire breakage (the 17375 content is hand-built JSON, tags carry the relays), zero consumers of the deleted trait, no ownership collisions, and every doctrine/manifest/file-size gate passes. Six findings survived verification.

**Findings, most severe first:**

1. `crates/nmp-nip60/src/nip60_wallet/nutzap_send.rs:84` — **`publish_nutzap_info` seeds the *authoritative* kind:10019 relay tags from `legacy_relay_hint`, the exact value the PR's own docs say "must never" drive relay selection** (found independently by 5 angles; PLAUSIBLE — every hop confirmed, but the method currently has zero callers and `build_nutzap_info_event` is a public escape hatch). Failure mode if an app adopts it: a wallet decoded from a stale or relay-less 17375 publishes a 10019 advertising stale/empty relays, senders deliver kind:9321 nutzaps to relays the user never reads, and value is silently lost. The doc comment saying callers "should prefer" a resolved set describes a capability the signature doesn't offer. Suggested fix: give it a `relays: &[String]` parameter (or delete the helper until `nmp-wallet` owns 10019 publication).

2. `crates/nmp-nip60/src/ownership.rs:102` — **the exclusive mechanism claim's `owns` text says the crate owns "the Cashu backend adapter *seam surface* consumed by nmp-wallet" — but this same PR deletes all adapter code, and the design doc explicitly places the seam in `nmp-wallet`, "not nmp-nip60"** (CONFIRMED; doc-accuracy severity — the linker symbol keys only on scope, so no gate fails). The claim even contradicts its own `notes` entry four lines down. Suggested fix: reword to the settled part only ("Cashu proof/DLEQ/P2PK/rollover types; adapter *for* the nmp-wallet seam").

3. `crates/nmp-nip60/src/wallet_event.rs:88` — **the demotion is read-side only: `build_wallet_event` still emits `relay` tags on every newly built kind:17375, and `WalletConfig::generate` still takes the hint parameter** (PLAUSIBLE — the design doc's sentence technically governs read authority, and the path is dormant until a caller exists, but new wallets will keep minting the "legacy residue" instead of letting it age out). Worth deciding now whether new 17375s should carry relay tags at all.

4. `crates/nmp-nip60/src/nip60_wallet.rs:84` — **`legacy_relay_hint` is stored twice** (handle field + inside `config: Arc<Mutex<WalletConfig>>`), with the two live readers split across the copies (`publish_nutzap_info` reads the handle, `build_wallet_event` reads the config), and the renamed public accessor at line 223 has zero callers workspace-wide (PLAUSIBLE — divergence is latent since nothing mutates post-construction). Collapse to one source and drop the dead accessor.

5. `crates/nmp-nip60/Cargo.toml:34` — **`ureq` is unconditional while the crate's siblings (`nmp-nip57`, `nmp-nip05`) gate it behind `native = ["dep:ureq"]`** (PLAUSIBLE — as a consistency matter only; the build-cost argument was refuted since `nmp-blossom` and `nmp-nip11` already compile ureq+TLS unconditionally). Gating the mint client behind the same `native` feature would match convention and keep the codec surface HTTP-free for wasm-ish consumers.

6. Doc prose — the same "legacy, non-authoritative; kind:10019/NIP-65 authoritative; policy lives in nmp-wallet; see design doc" paragraph is restated roughly seven times across `lib.rs`, `mint_announce.rs`, `nip60_wallet.rs` (×3), `wallet_event.rs` (×2), and `nutzap_send.rs` (CONFIRMED by inspection). State it once on the accessor and link the rest — seven copies will drift.

**Verified non-issues worth knowing:** the field rename cannot break persisted wallets (relays live in tags, config JSON is hand-built key-pairs); the `nostr = "0.44"` literal pin is an explicit documented workspace policy boundary, not a missed migration; the kind:38172 exclusive claim without design-doc mention matches workspace norm (nmp-nip57/nmp-blossom do the same); and the release dry-run only runs `cargo package --list` over the new `[[public_crates]]` entry, with all path deps also public.

**Verdict:** mergeable once finding 1 is addressed — it's the one place the PR's new contract and its code point in opposite directions, and it sits on the exact "silent value loss, fails at runtime" path the wallet design doc forbids. Findings 2–6 are small, and 2 is a two-line prose fix worth doing in the same PR. Want me to post these as comments on the PR?
