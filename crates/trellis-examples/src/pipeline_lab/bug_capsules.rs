use crate::seeded_bugs::{self, SeededBugCapsule, SeededBugReport};

/// Returns PipelineLab seeded-bug capsule metadata.
pub fn available_bug_capsules() -> Vec<SeededBugCapsule> {
    vec![capsule()]
}

/// Runs all PipelineLab seeded-bug capsules.
pub fn run_all_bug_capsules() -> Vec<SeededBugReport> {
    vec![run_bug_capsule("pipeline-credential-revoke-clears-previews").unwrap()]
}

/// Runs one PipelineLab seeded-bug capsule by stable name.
pub fn run_bug_capsule(name: &str) -> Option<SeededBugReport> {
    (name == "pipeline-credential-revoke-clears-previews").then(run_revoke_capsule)
}

fn capsule() -> SeededBugCapsule {
    seeded_bugs::capsule(
        "pipeline-credential-revoke-clears-previews",
        "Credential revoke clears pipeline previews",
        "source credential revoke must close pipeline work and clear preview panels",
        &[
            "pipeline-revoked-source-left-connection-open",
            "pipeline-revoked-node-left-compute-job-open",
            "pipeline-revoked-node-left-preview-visible",
        ],
    )
}

fn run_revoke_capsule() -> SeededBugReport {
    let invariant = "source credential revoke must close pipeline work and clear preview panels";
    let success_path = seeded_bugs::run("trellis", "revoke-credential", 1, Vec::new());
    let seeded_bug_path = seeded_bugs::run(
        "seeded-bug",
        "revoke-credential",
        1,
        vec![
            seeded_bugs::failure(
                "pipeline-revoked-source-left-connection-open",
                "Source connection remained open",
                "resource ledger",
                invariant,
                "warehouse connection remained open after the credential was revoked",
            ),
            seeded_bugs::failure(
                "pipeline-revoked-node-left-compute-job-open",
                "Compute job remained open",
                "resource ledger",
                invariant,
                "daily_revenue compute stayed active after its source credential was revoked",
            ),
            seeded_bugs::failure(
                "pipeline-revoked-node-left-preview-visible",
                "Preview panel kept unauthorized rows",
                "output ledger",
                invariant,
                "clean_orders preview rows remained visible after warehouse revoke",
            ),
        ],
    );
    seeded_bugs::report(capsule(), success_path, seeded_bug_path)
}
