use crate::seeded_bugs::{self, SeededBugCapsule, SeededBugReport};

/// Returns CollabCanvas seeded-bug capsule metadata.
pub fn available_bug_capsules() -> Vec<SeededBugCapsule> {
    vec![capsule()]
}

/// Runs all CollabCanvas seeded-bug capsules.
pub fn run_all_bug_capsules() -> Vec<SeededBugReport> {
    vec![run_bug_capsule("collab-hidden-attachment-cancels-hydration").unwrap()]
}

/// Runs one CollabCanvas seeded-bug capsule by stable name.
pub fn run_bug_capsule(name: &str) -> Option<SeededBugReport> {
    (name == "collab-hidden-attachment-cancels-hydration").then(run_hidden_attachment_capsule)
}

fn capsule() -> SeededBugCapsule {
    seeded_bugs::capsule(
        "collab-hidden-attachment-cancels-hydration",
        "Hidden attachment cancels hydration",
        "hidden attachments must close hydration jobs and disappear from editor output",
        &[
            "collab-hidden-attachment-left-hydration-open",
            "collab-hidden-attachment-left-output-visible",
        ],
    )
}

fn run_hidden_attachment_capsule() -> SeededBugReport {
    let invariant = "hidden attachments must close hydration jobs and disappear from editor output";
    let success_path = seeded_bugs::run("trellis", "hide-attachment", 1, Vec::new());
    let seeded_bug_path = seeded_bugs::run(
        "seeded-bug",
        "hide-attachment",
        1,
        vec![
            seeded_bugs::failure(
                "collab-hidden-attachment-left-hydration-open",
                "Hydration job remained open",
                "resource ledger",
                invariant,
                "attachment hero.png was no longer visible but its hydration resource stayed open",
            ),
            seeded_bugs::failure(
                "collab-hidden-attachment-left-output-visible",
                "Editor output kept hidden attachment",
                "output ledger",
                invariant,
                "editor output still listed hero.png after the visibility set became empty",
            ),
        ],
    );
    seeded_bugs::report(capsule(), success_path, seeded_bug_path)
}
