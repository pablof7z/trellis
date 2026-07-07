---
type: noun-entry
slug: walletbackend-deleted
name: "WalletBackend (deleted)"
origin: extracted
source_refs:
  - transcript:940-940
---

# WalletBackend (deleted)

A dead unified trait in nmp-nip60/backend.rs whose pay_invoice always returned Err(Unsupported) with zero real callers — a false product surface. Deleted in PR #2866; the real WalletBackend seam belongs to the future nmp-wallet composition crate.
