use trellis_observatory_engine::{available_bug_capsules, run_all_bug_capsules, run_bug_capsule};

#[test]
fn capsules_are_discoverable() {
    let capsules = available_bug_capsules();

    assert_eq!(capsules.len(), 3);
    assert!(capsules.iter().any(|capsule| {
        capsule.name == "delete-file-lifecycle"
            && capsule
                .expected_failure_ids
                .contains(&"no-watcher-for-removed-files".to_owned())
    }));
}

#[test]
fn each_capsule_passes_success_path_and_detects_seeded_bug() {
    let reports = run_all_bug_capsules();

    assert_eq!(reports.len(), 3);
    for report in reports {
        assert_eq!(report.status, "pass", "{report:#?}");
        assert!(report.success_path.passed, "{report:#?}");
        assert!(!report.seeded_bug_path.passed, "{report:#?}");
        assert!(report.expected_failures_detected, "{report:#?}");
        for failure in &report.seeded_bug_path.failed_checks {
            assert!(
                failure.failure_text.contains(&failure.source),
                "{failure:#?}"
            );
        }
    }
}

#[test]
fn named_capsule_runs_independently() {
    let report = run_bug_capsule("stale-analysis-status").unwrap();

    assert_eq!(report.status, "pass");
    assert!(report.seeded_bug_path.failed_checks.iter().any(|failure| {
        failure.id == "stale-host-status-no-output"
            && failure.failure_text.contains("HostStatusAudit")
    }));
    assert!(run_bug_capsule("missing-capsule").is_none());
}
