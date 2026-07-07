---
type: research-record
date: 2026-07-03
session: c7805f5d-42c5-44b6-8eaa-ecd2453ed822
transcript: /Users/pablofernandez/.claude/projects/-Users-pablofernandez-src-trellis/c7805f5d-42c5-44b6-8eaa-ecd2453ed822.jsonl
source_lines: 1423-1433
agent_attribution: verify-4
has_preregistered_criteria: true
has_method: true
has_structured_report: true
characterization: "Verification of kind:38172 exclusive ownership claim without design-doc basis; verdict: REFUTED (workspace norm allows exclusive claims on implemented codecs without doc backing)"
captured_at: 2026-07-03T09:30:55Z
---

Verification of kind:38172 exclusive ownership claim without design-doc basis; verdict: REFUTED (workspace norm allows exclusive claims on implemented codecs without doc backing)

---

<agent-message from="verify-4">
VERDICT: REFUTED — crates/nmp-nip60/src/ownership.rs:85-95 (kind:38172 exclusive claim)

(a) TRUE that 38172/NIP-88 is absent from the doc: grep of docs/architecture/nip60-nip61-wallet-design.md for "38172|NIP-88|nip88" returns zero hits, and the "Existing NIP Crates" section (lines 71-72) lists only "kind:17375, kind:7375, kind:7376, kind:7374, kind:10019, and kind:9321."

(b) The claim reflects real, shipped code. crates/nmp-nip60/src/mint_announce.rs is a full kind:38172 codec: line 1 "//! NIP-88 mint announcement event (kind:38172)", a `MintAnnouncement` struct, `build_mint_announce_event`, a decoder, and a subscription filter (`.kind(Kind::from(KIND_MINT_ANNOUNCE as u16))`). The exclusive claim guards code that exists.

(c) Doc-basis is NOT the workspace norm — code-implements-it is. nmp-nip60 is the ONLY crate with a design doc enumerating its kind ownership (grep for "Existing NIP Crates|owns reusable" across docs/ returns just this doc + one unrelated content-rendering doc). Sampled peers claim exclusive kinds with NO design-doc backing at all: nmp-nip57/src/ownership.rs claims 9734 and 9735 (both `exclusive: true`), nmp-blossom/src/ownership.rs claims 24242 (`exclusive: true`), and neither crate even has an architecture design doc (grep of docs/architecture/ for these kinds = zero). So an exclusive linker-symbol claim on an implemented kind absent from any design doc is exactly the established convention, not a violation of it.

The candidate's premise — that a doc must sanction the kind for the exclusive claim to be legitimate — is contradicted by the majority of the workspace. This candidate does not describe a defect.
</agent-message>
