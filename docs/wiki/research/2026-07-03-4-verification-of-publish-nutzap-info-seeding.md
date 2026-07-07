---
type: research-record
date: 2026-07-03
session: c7805f5d-42c5-44b6-8eaa-ecd2453ed822
transcript: /Users/pablofernandez/.claude/projects/-Users-pablofernandez-src-trellis/c7805f5d-42c5-44b6-8eaa-ecd2453ed822.jsonl
source_lines: 1408-1416
agent_attribution: verify-1
has_preregistered_criteria: true
has_method: true
has_structured_report: true
characterization: "Verification of publish_nutzap_info seeding authoritative kind:10019 from legacy hint; verdict: PLAUSIBLE (every hop confirmed, but zero callers and escape-hatch API exist)"
captured_at: 2026-07-03T09:30:55Z
---

Verification of publish_nutzap_info seeding authoritative kind:10019 from legacy hint; verdict: PLAUSIBLE (every hop confirmed, but zero callers and escape-hatch API exist)

---

<agent-message from="verify-1">
VERDICT: PLAUSIBLE — candidate at crates/nmp-nip60/src/nip60_wallet/nutzap_send.rs:84 `relays: self.legacy_relay_hint.clone(),`

The full mechanism is real and every hop is confirmed in code:
(a) NutZapInfo.relays DOES become kind:10019 `relay` tags — build_nutzap_info_event (nutzap.rs:49-51) emits one `Tag::custom("relay", [relay])` per entry.
(c) from_wallet_event → decode_wallet_event populates legacy_relay_hint from the kind:17375's own `relay` tags (wallet_event.rs:132-137, "Extract the legacy relay compatibility hint (non-authoritative)"). So a 17375 with stale/empty relay tags yields a legacy_relay_hint with stale/empty relays, which publish_nutzap_info then stamps onto the 10019.

Why PLAUSIBLE not CONFIRMED — the (b) claim ("no alternative API lets a caller supply resolved relays") is not fully accurate, and the trigger is undemonstrated: publish_nutzap_info(&self) indeed takes no relay parameter, BUT the crate publicly re-exports build_nutzap_info_event (lib.rs:78), which accepts an arbitrary NutZapInfo — a caller with a resolved NIP-65/10019 set can build+publish the 10019 through the kernel without ever calling publish_nutzap_info. That is exactly the escape hatch the new doc comment points to ("callers ... should prefer publishing that set instead"). Also, publish_nutzap_info has ZERO callers anywhere in the repo/crates, so nothing currently exercises the bad default. The harm (senders publish 9321 nutzaps to stale/no relays → silently lost) only fires IF an app adopts publish_nutzap_info as its standard 10019-publish path AND feeds it 17375s lacking good relay tags. WHAT WOULD CONFIRM: an actual app/kernel caller invoking publish_nutzap_info as the routine way to publish the user's 10019 rather than the resolved-relay builder.
</agent-message>
