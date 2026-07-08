use crate::seeded_bugs::{self, SeededBugCapsule, SeededBugReport};

/// Returns MarketDesk seeded-bug capsule metadata.
pub fn available_bug_capsules() -> Vec<SeededBugCapsule> {
    vec![capsule()]
}

/// Runs all MarketDesk seeded-bug capsules.
pub fn run_all_bug_capsules() -> Vec<SeededBugReport> {
    vec![run_bug_capsule("market-entitlement-revoke-closes-feeds").unwrap()]
}

/// Runs one MarketDesk seeded-bug capsule by stable name.
pub fn run_bug_capsule(name: &str) -> Option<SeededBugReport> {
    (name == "market-entitlement-revoke-closes-feeds").then(run_revoke_capsule)
}

fn capsule() -> SeededBugCapsule {
    seeded_bugs::capsule(
        "market-entitlement-revoke-closes-feeds",
        "Entitlement revoke closes market feeds",
        "revoked market entitlements must close feeds and remove terminal rows",
        &[
            "market-revoked-symbol-left-quote-open",
            "market-revoked-symbol-left-trade-open",
            "market-revoked-symbol-left-row-visible",
        ],
    )
}

fn run_revoke_capsule() -> SeededBugReport {
    let invariant = "revoked market entitlements must close feeds and remove terminal rows";
    let success_path = seeded_bugs::run("trellis", "revoke-entitlement", 1, Vec::new());
    let seeded_bug_path = seeded_bugs::run(
        "seeded-bug",
        "revoke-entitlement",
        1,
        vec![
            seeded_bugs::failure(
                "market-revoked-symbol-left-quote-open",
                "Quote feed remained open",
                "resource ledger",
                invariant,
                "NVDA quote feed was still open after entitlement revoke",
            ),
            seeded_bugs::failure(
                "market-revoked-symbol-left-trade-open",
                "Trade feed remained open",
                "resource ledger",
                invariant,
                "NVDA trade feed was still open after entitlement revoke",
            ),
            seeded_bugs::failure(
                "market-revoked-symbol-left-row-visible",
                "Quote grid kept revoked row",
                "output ledger",
                invariant,
                "NVDA still appeared in the terminal grid after entitlement revoke",
            ),
        ],
    );
    seeded_bugs::report(capsule(), success_path, seeded_bug_path)
}
