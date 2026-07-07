use crate::seeded_bugs::{self, SeededBugCapsule, SeededBugReport, SeededBugRun};

use super::bug_capsule_paths::{
    run_empty_workspace, run_permission_revoke, run_removed_issue_delta, run_workspace_switch,
};

struct CapsuleSpec {
    name: &'static str,
    title: &'static str,
    invariant: &'static str,
    expected_failure_ids: &'static [&'static str],
    run: fn(&'static str) -> (SeededBugRun, SeededBugRun),
}

/// Returns Workspace Sync Board seeded-bug capsule metadata.
pub fn available_bug_capsules() -> Vec<SeededBugCapsule> {
    capsule_specs().iter().map(capsule_for).collect()
}

/// Runs all Workspace Sync Board seeded-bug capsules.
pub fn run_all_bug_capsules() -> Vec<SeededBugReport> {
    capsule_specs().iter().map(run_spec).collect()
}

/// Runs one Workspace Sync Board seeded-bug capsule by stable name.
pub fn run_bug_capsule(name: &str) -> Option<SeededBugReport> {
    capsule_specs()
        .iter()
        .find(|spec| spec.name == name)
        .map(run_spec)
}

fn run_spec(spec: &CapsuleSpec) -> SeededBugReport {
    let capsule = capsule_for(spec);
    let (success_path, seeded_bug_path) = (spec.run)(spec.invariant);
    seeded_bugs::report(capsule, success_path, seeded_bug_path)
}

fn capsule_for(spec: &CapsuleSpec) -> SeededBugCapsule {
    seeded_bugs::capsule(
        spec.name,
        spec.title,
        spec.invariant,
        spec.expected_failure_ids,
    )
}

fn capsule_specs() -> [CapsuleSpec; 4] {
    [
        CapsuleSpec {
            name: "workspace-switch-closes-old-windows",
            title: "Workspace switch closes old sync windows",
            invariant: "workspace switch withdraws previous workspace sync demand",
            expected_failure_ids: &["old-workspace-sync-window-closed"],
            run: run_workspace_switch,
        },
        CapsuleSpec {
            name: "workspace-revoke-clears-project-rows",
            title: "Permission revoke clears revoked project rows",
            invariant: "revoked project rows disappear from the output ledger",
            expected_failure_ids: &["revoked-project-rows-cleared"],
            run: run_permission_revoke,
        },
        CapsuleSpec {
            name: "workspace-empty-opens-no-broad-sync",
            title: "Empty workspace opens no broad sync window",
            invariant: "empty workspace demand remains empty",
            expected_failure_ids: &["empty-workspace-opens-no-broad-sync"],
            run: run_empty_workspace,
        },
        CapsuleSpec {
            name: "workspace-removed-issue-delta-recomputes",
            title: "Removed issue delta matches full recompute",
            invariant: "issue deltas match the full recompute oracle",
            expected_failure_ids: &["workspace-delta-matches-full-recompute"],
            run: run_removed_issue_delta,
        },
    ]
}
