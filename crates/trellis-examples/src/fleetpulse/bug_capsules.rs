use crate::seeded_bugs::{self, SeededBugCapsule, SeededBugReport, SeededBugRun};

use super::bug_capsule_paths::{
    run_empty_device_set, run_filter_shrink, run_late_status, run_shared_topic,
};

struct CapsuleSpec {
    name: &'static str,
    title: &'static str,
    invariant: &'static str,
    expected_failure_ids: &'static [&'static str],
    run: fn(&'static str) -> (SeededBugRun, SeededBugRun),
}

/// Returns FleetPulse seeded-bug capsule metadata.
pub fn available_bug_capsules() -> Vec<SeededBugCapsule> {
    capsule_specs().iter().map(capsule_for).collect()
}

/// Runs all FleetPulse seeded-bug capsules.
pub fn run_all_bug_capsules() -> Vec<SeededBugReport> {
    capsule_specs().iter().map(run_spec).collect()
}

/// Runs one FleetPulse seeded-bug capsule by stable name.
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
            name: "fleet-filter-shrink-unsubscribes-topic",
            title: "Filter shrink unsubscribes removed topic",
            invariant: "filter shrink withdraws telemetry subscriptions",
            expected_failure_ids: &["fleet-filter-shrink-unsubscribes-topic"],
            run: run_filter_shrink,
        },
        CapsuleSpec {
            name: "fleet-late-closed-topic-status",
            title: "Late closed-topic status is audit-only",
            invariant: "late host status cannot reopen closed topic demand",
            expected_failure_ids: &["fleet-late-status-audit-only"],
            run: run_late_status,
        },
        CapsuleSpec {
            name: "fleet-shared-topic-keeps-last-owner",
            title: "Shared topic stays open until last owner leaves",
            invariant: "shared resource closes only after the last owner leaves",
            expected_failure_ids: &["fleet-shared-topic-keeps-last-owner"],
            run: run_shared_topic,
        },
        CapsuleSpec {
            name: "fleet-empty-device-set-opens-no-wildcard",
            title: "Empty device set opens no wildcard subscription",
            invariant: "empty device demand remains empty",
            expected_failure_ids: &["fleet-empty-device-set-opens-no-wildcard"],
            run: run_empty_device_set,
        },
    ]
}
