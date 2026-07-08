use crate::seeded_bugs::{self, SeededBugCapsule, SeededBugReport};

/// Returns SearchOps seeded-bug capsule metadata.
pub fn available_bug_capsules() -> Vec<SeededBugCapsule> {
    vec![capsule()]
}

/// Runs all SearchOps seeded-bug capsules.
pub fn run_all_bug_capsules() -> Vec<SeededBugReport> {
    vec![run_bug_capsule("search-permission-revoke-clears-results").unwrap()]
}

/// Runs one SearchOps seeded-bug capsule by stable name.
pub fn run_bug_capsule(name: &str) -> Option<SeededBugReport> {
    (name == "search-permission-revoke-clears-results").then(run_revoke_capsule)
}

fn capsule() -> SeededBugCapsule {
    seeded_bugs::capsule(
        "search-permission-revoke-clears-results",
        "Permission revoke clears search results",
        "permission revocation must close unauthorized search work and clear result rows",
        &[
            "search-revoked-doc-left-result-visible",
            "search-stale-ranking-job-left-open",
            "search-revoked-shard-left-cache-window",
        ],
    )
}

fn run_revoke_capsule() -> SeededBugReport {
    let invariant =
        "permission revocation must close unauthorized search work and clear result rows";
    let success_path = seeded_bugs::run("trellis", "revoke-permission", 1, Vec::new());
    let seeded_bug_path = seeded_bugs::run(
        "seeded-bug",
        "revoke-permission",
        1,
        vec![
            seeded_bugs::failure(
                "search-revoked-doc-left-result-visible",
                "Revoked result row remained visible",
                "output ledger",
                invariant,
                "mail-002 still appeared in the visible result window after permission revoke",
            ),
            seeded_bugs::failure(
                "search-stale-ranking-job-left-open",
                "Stale ranking job remained open",
                "resource ledger",
                invariant,
                "a ranking job for the stale query remained active after search inputs changed",
            ),
            seeded_bugs::failure(
                "search-revoked-shard-left-cache-window",
                "Revoked result cache remained open",
                "resource ledger",
                invariant,
                "the result cache still covered a window containing a revoked document",
            ),
        ],
    );
    seeded_bugs::report(capsule(), success_path, seeded_bug_path)
}
