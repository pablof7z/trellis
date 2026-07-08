use crate::seeded_bugs::{self, SeededBugCapsule, SeededBugReport};

/// Returns ControlPlane Lite seeded-bug capsule metadata.
pub fn available_bug_capsules() -> Vec<SeededBugCapsule> {
    vec![capsule()]
}

/// Runs all ControlPlane Lite seeded-bug capsules.
pub fn run_all_bug_capsules() -> Vec<SeededBugReport> {
    vec![run_bug_capsule("control-resource-failure-opens-retry").unwrap()]
}

/// Runs one ControlPlane Lite seeded-bug capsule by stable name.
pub fn run_bug_capsule(name: &str) -> Option<SeededBugReport> {
    (name == "control-resource-failure-opens-retry").then(run_failure_capsule)
}

fn capsule() -> SeededBugCapsule {
    seeded_bugs::capsule(
        "control-resource-failure-opens-retry",
        "Resource failure opens retry demand",
        "failed desired resources must create retry demand and degraded status",
        &[
            "control-failed-worker-left-no-retry",
            "control-degraded-condition-missing",
            "control-close-left-worker-open",
        ],
    )
}

fn run_failure_capsule() -> SeededBugReport {
    let invariant = "failed desired resources must create retry demand and degraded status";
    let success_path = seeded_bugs::run("trellis", "resource-failed", 1, Vec::new());
    let seeded_bug_path = seeded_bugs::run(
        "seeded-bug",
        "resource-failed",
        1,
        vec![
            seeded_bugs::failure(
                "control-failed-worker-left-no-retry",
                "Retry demand was not opened",
                "resource ledger",
                invariant,
                "checkout worker failure did not create retry job demand",
            ),
            seeded_bugs::failure(
                "control-degraded-condition-missing",
                "Degraded status was missing",
                "output ledger",
                invariant,
                "status output did not report degraded after worker failure",
            ),
            seeded_bugs::failure(
                "control-close-left-worker-open",
                "Worker remained open after close",
                "resource ledger",
                invariant,
                "controller close left a worker resource open",
            ),
        ],
    );
    seeded_bugs::report(capsule(), success_path, seeded_bug_path)
}
