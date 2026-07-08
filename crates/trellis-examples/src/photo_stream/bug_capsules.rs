use crate::seeded_bugs::{self, SeededBugCapsule, SeededBugReport};

/// Returns PhotoStream seeded-bug capsule metadata.
pub fn available_bug_capsules() -> Vec<SeededBugCapsule> {
    vec![capsule()]
}

/// Runs all PhotoStream seeded-bug capsules.
pub fn run_all_bug_capsules() -> Vec<SeededBugReport> {
    vec![run_bug_capsule("photo-storage-pressure-drops-optional-work").unwrap()]
}

/// Runs one PhotoStream seeded-bug capsule by stable name.
pub fn run_bug_capsule(name: &str) -> Option<SeededBugReport> {
    (name == "photo-storage-pressure-drops-optional-work").then(run_pressure_capsule)
}

fn capsule() -> SeededBugCapsule {
    seeded_bugs::capsule(
        "photo-storage-pressure-drops-optional-work",
        "Storage pressure drops optional photo work",
        "storage pressure must close optional high-res and cloud jobs without stale grid rows",
        &[
            "photo-pressure-left-highres-open",
            "photo-pressure-left-cloud-open",
            "photo-rule-change-left-stale-tile",
        ],
    )
}

fn run_pressure_capsule() -> SeededBugReport {
    let invariant =
        "storage pressure must close optional high-res and cloud jobs without stale grid rows";
    let success_path = seeded_bugs::run("trellis", "storage-pressure", 1, Vec::new());
    let seeded_bug_path = seeded_bugs::run(
        "seeded-bug",
        "storage-pressure",
        1,
        vec![
            seeded_bugs::failure(
                "photo-pressure-left-highres-open",
                "High-res preview remained open",
                "resource ledger",
                invariant,
                "asset-001 kept a high-resolution preview while storage was constrained",
            ),
            seeded_bugs::failure(
                "photo-pressure-left-cloud-open",
                "Cloud download remained open",
                "resource ledger",
                invariant,
                "asset-006 kept an optional cloud download while storage was constrained",
            ),
            seeded_bugs::failure(
                "photo-rule-change-left-stale-tile",
                "Grid kept a stale tile",
                "output ledger",
                invariant,
                "a removed smart-album asset still appeared in the bounded grid output",
            ),
        ],
    );
    seeded_bugs::report(capsule(), success_path, seeded_bug_path)
}
