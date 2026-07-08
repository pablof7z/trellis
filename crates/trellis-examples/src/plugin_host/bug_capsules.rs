use crate::seeded_bugs::{self, SeededBugCapsule, SeededBugReport};

/// Returns PluginHost seeded-bug capsule metadata.
pub fn available_bug_capsules() -> Vec<SeededBugCapsule> {
    vec![capsule()]
}

/// Runs all PluginHost seeded-bug capsules.
pub fn run_all_bug_capsules() -> Vec<SeededBugReport> {
    vec![run_bug_capsule("plugin-disable-closes-capabilities").unwrap()]
}

/// Runs one PluginHost seeded-bug capsule by stable name.
pub fn run_bug_capsule(name: &str) -> Option<SeededBugReport> {
    (name == "plugin-disable-closes-capabilities").then(run_disable_capsule)
}

fn capsule() -> SeededBugCapsule {
    seeded_bugs::capsule(
        "plugin-disable-closes-capabilities",
        "Plugin disable closes capabilities",
        "disabling a plugin must close every contributed capability and clear shell output",
        &[
            "plugin-disable-left-watcher-open",
            "plugin-disable-left-worker-open",
            "plugin-disable-left-command-visible",
        ],
    )
}

fn run_disable_capsule() -> SeededBugReport {
    let invariant =
        "disabling a plugin must close every contributed capability and clear shell output";
    let success_path = seeded_bugs::run("trellis", "disable-plugin", 1, Vec::new());
    let seeded_bug_path = seeded_bugs::run(
        "seeded-bug",
        "disable-plugin",
        1,
        vec![
            seeded_bugs::failure(
                "plugin-disable-left-watcher-open",
                "File watcher remained open",
                "resource ledger",
                invariant,
                "src/**/*.rs was still watched after plugin fmt was disabled",
            ),
            seeded_bugs::failure(
                "plugin-disable-left-worker-open",
                "Background worker remained open",
                "resource ledger",
                invariant,
                "rustfmt-daemon continued running after plugin fmt was disabled",
            ),
            seeded_bugs::failure(
                "plugin-disable-left-command-visible",
                "Command palette kept disabled command",
                "output ledger",
                invariant,
                "Format file was still visible after the plugin scope closed",
            ),
        ],
    );
    seeded_bugs::report(capsule(), success_path, seeded_bug_path)
}
