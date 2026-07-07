---
type: research-record
date: 2026-07-03
session: c7805f5d-42c5-44b6-8eaa-ecd2453ed822
transcript: /Users/pablofernandez/.claude/projects/-Users-pablofernandez-src-trellis/c7805f5d-42c5-44b6-8eaa-ecd2453ed822.jsonl
source_lines: 1500-1512
agent_attribution: verify-3
has_preregistered_criteria: true
has_method: true
has_structured_report: true
characterization: "Verification of ownership claim prose describing deleted backend adapter seam; verdict: CONFIRMED (doc-accuracy severity — prose misdescribes deleted code, but linker symbol keys on scope only)"
captured_at: 2026-07-03T09:30:55Z
---

Verification of ownership claim prose describing deleted backend adapter seam; verdict: CONFIRMED (doc-accuracy severity — prose misdescribes deleted code, but linker symbol keys on scope only)

---

<agent-message from="verify-3">
VERDICT: CONFIRMED — crates/nmp-nip60/src/ownership.rs:97-108

The `owns` text on the exclusive `mechanism` claim `nip60.cashu_backend_adapter` — "Cashu proof/DLEQ/P2PK/rollover types **and the Cashu backend adapter seam surface consumed by nmp-wallet**" — is inconsistent with both the code state and the design doc, and all three sub-issues verify:

(1) DELETED code: `git diff master...pr2866` shows `crates/nmp-nip60/src/backend.rs` removed (82 deletions: the `WalletBackend` trait, `PayResult`, `WalletError`), plus `pub mod backend;` and `pub use backend::{PayResult, WalletBackend, WalletError};` removed from lib.rs. `ls backend.rs` = No such file. So the "seam surface consumed by nmp-wallet" phrase describes code that no longer exists in this crate after the PR.

(2) adapter-vs-seam conflation: design doc is explicit — line 101 "`WalletBackend` lives in `nmp-wallet`, not `nmp-core` and not `nmp-nip60`"; line 75 (Existing NIP Crates) nmp-nip60 owns "a Cashu backend adapter **for** the `nmp-wallet::WalletBackend` seam." The doc cleanly separates *adapter* (nip60's) from *seam* (nmp-wallet's). The claim's phrase "the Cashu backend adapter **seam surface** consumed by nmp-wallet" fuses the two and reads as claiming the seam — which the crate's own note[0] (ownership.rs:112-115) then contradicts: "the unified WalletBackend seam ... are owned by nmp-wallet ... not this crate." Internal tension between the `owns` text and its own note.

(3) bundling: one exclusive id bundles settled crypto-type ownership (Cashu proof/DLEQ/P2PK/rollover — legitimate, matches doc line 73) with the contested/nonexistent "seam surface" wording in a single `owns` string.

CAVEAT on severity / enforcement: this is a documentation-accuracy finding, not a functional or linker bug. Per crates/nmp-ownership/src/macros.rs:15 the exclusive linker symbol is `concat!("__nmp_own__", claim_type, "__", scope_kind, "__", scope_value, "__", context)` = `__nmp_own__mechanism__namespace__nmp-nip60__`. It keys ONLY on claim_type + scope kind/value/context — NOT on the `id` and NOT on the `owns` text (lib.rs:59-63 confirms collision is scope-symbol based). So `exclusive: true` here merely reserves the (mechanism, namespace=nmp-nip60) scope against another crate; the "seam surface" wording is purely descriptive prose and causes no collision. The finding is real (the prose misdescribes deleted code and conflates adapter/seam against the doc), but its impact is claim-text accuracy, not runtime/build behavior.
</agent-message>
