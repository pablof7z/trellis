---
title: NIP-60/61 Wallet Integration
slug: nip60-wallet
topic: nostr-protocol
summary: "NMP #2864 is the epic tracking the NIP-60/61 wallet milestone with six phases: reactivate nmp-nip60, nmp-wallet composition crate (WalletBackend seam plus opera"
tags:
  - capture
volatility: warm
confidence: medium
created: 2026-07-03
updated: 2026-07-03
verified: 2026-07-03
compiled-from: conversation
sources:
  - session:c7805f5d-42c5-44b6-8eaa-ecd2453ed822
---

# NIP-60/61 Wallet Integration

## Overview

NMP #2864 is the epic tracking the NIP-60/61 wallet milestone with six phases: reactivate nmp-nip60, nmp-wallet composition crate (WalletBackend seam plus operation journal), NWC consolidation behind the seam, NIP-60 Cashu wallet, NIP-61 nutzap send/receive, and Chirp UX.

<!-- citations: [^c7805-70d1f] [^c7805-50924] [^c7805-61ec3] -->
## nmp-wallet Composition Crate

nmp-wallet is a planned composition crate that owns the WalletBackend seam (not nmp-core, not nmp-nip60) and an operation journal, consolidating NWC and NIP-60 Cashu wallets behind a unified interface. Relay acquisition policy lives in nmp-wallet, using kind:10019 as the authoritative relay source with NIP-65 fallback; kind:17375 relay tags are a non-authoritative compatibility hint that must never drive relay selection.

<!-- citations: [^c7805-61ec3] [^c7805-536ce] [^c7805-dd258] [^c7805-2fc49] -->
## Design Invariants

The NIP-60/61 wallet milestone enforces invariants: fail-closed as state (never a runtime error), no secrets in projections, and thin shells for app/platform code.

A NIP-60 wallet app on NMP dispatches typed actions (nmp.wallet.cashu.create, cashu.deposit_quote, nutzap.send, nutzap.redeem, pay_invoice) and binds a single 'wallet' projection; the app never touches keys, proofs, mint HTTP, or relay decisions directly. The projection never contains keys, Cashu proofs, or secrets; capability flags drive the UI so a feature the backend lacks is a missing button, never a runtime error dialog.

Every value-moving wallet operation is recorded first in a durable operation journal before execution, so a process crash can never double-spend — on restart the kernel reconciles against the mint.

<!-- citations: [^c7805-61ec3] [^c7805-50924] [^c7805-ad91f] -->
## Phase 0 — NMP #2865

NMP #2865 is the Phase 0 subissue for the NIP-60/61 wallet milestone: reactivate crates/nmp-nip60 into workspace and CI, capability-gate the false pay_invoice stubs, and demote the legacy kind:17375 relay-tag authority. It is the recommended first coding-agent brief because it has zero open architecture decisions. The nmp-nip60 crate scope is NIP mechanics only — event codecs, types, pure shape validation — with all networking and the WalletBackend seam living in the not-yet-existent nmp-wallet crate, not in nmp-nip60 and not in nmp-core.

The kind:17375 relay tags are renamed to legacy_relay_hint and documented as a non-authoritative compatibility hint that must never override kind:10019 or NIP-65 as the relay-selection source of truth. The WalletConfig field rename from relays to legacy_relay_hint cannot break persisted NIP-60 wallets because the kind:17375 content is hand-built JSON key-pair arrays and relays live in event tags, not in serde-derived content. The no-relay-vesting hint paragraph is restated roughly seven times across lib.rs, mint_announce.rs, nip60_wallet.rs (module doc, field doc, accessor doc), wallet_event.rs (module doc, field doc), and nutzap_send.rs; it should be stated once on the accessor with the rest linking to it.

Three current-code issues block the stated goal of dropping legacy relay-tag authority. First, publish_nutzap_info seeds the authoritative kind:10019 NutZap-info relay tags from self.legacy_relay_hint with no parameter to pass a resolved relay set, contradicting the PR's own doctrine that legacy_relay_hint must never drive relay selection; this is filed as NMP #2870 as a must-fix before Phase 1 gives publish_nutzap_info its first caller, since it is dormant now and cheap to fix but would cause silent value loss if adopted. Second, build_wallet_event still emits relay tags on every newly built kind:17375 and WalletConfig::generate still takes the legacy_relay_hint parameter, so the demotion is read-side only and new wallets keep minting the legacy residue rather than letting it age out — the design doc governs read authority only, so this is a design tension to decide before nmp-wallet lands. Third, the legacy_relay_hint field is stored twice — as a handle field and inside config: Arc<Mutex<WalletConfig>> — with publish_nutzap_info reading the handle copy and build_wallet_event reading the config copy, and the public accessor has zero callers workspace-wide and is dead code.

Sibling crates nmp-nip57 and nmp-nip05 gate ureq behind an optional `native` feature (native = ["dep:ureq"]), while nmp-nip60 makes ureq an unconditional dependency; gating the mint client behind the same `native` feature would match convention and keep the codec surface HTTP-free for wasm consumers.

The nmp-nip60 ownership descriptor's exclusive mechanism claim over the Cashu backend adapter seam surface describes code the same PR deleted and a seam the design doc assigns to nmp-wallet, creating an internal contradiction with the descriptor's own note. Separately, the kind:38172 (NIP-88 mint announce) exclusive ownership claim has no design-doc mention, which matches the workspace norm where nmp-nip57 claims 9734/9735 and nmp-blossom claims 24242 exclusively with no design-doc backing.

<!-- citations: [^c7805-13f04] [^c7805-5f091] [^c7805-e2ed6] [^c7805-8ea4e] [^c7805-6c287] [^c7805-f1c9e] -->
## Phase 0 Review — NMP #2870

NMP #2870 tracks the six review findings from PR #2866 with verdicts, file:line references, failure scenarios, suggested fixes, and a verified non-issues section covering serde compat, deleted-trait consumers, ownership collisions, the nostr pin, and mechanical gates. The must-fix finding that publish_nutzap_info seeds authoritative kind:10019 relay tags from the non-authoritative legacy_relay_hint is tracked here and must land before Phase 1 gives the method its first caller.

<!-- citations: [^c7805-035f7] [^c7805-d4801] -->

## Builder Doc — NMP #2872

NMP #2872 tracks a builder doc explaining how to make a NIP-60 wallet app using NMP; it is a checklist item under Phase 1 of epic #2864. The builder doc must never mention Trellis vocabulary (per ADR-0075) and must never show secrets in examples; capability-gating is the canonical UI pattern. <!-- [^c7805-02955] -->
