//! Shared seeded-bug capsule report types for showcase examples.

/// Discoverable seeded-bug capsule metadata.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeededBugCapsule {
    /// Stable capsule name accepted by `--capsule`.
    pub name: String,
    /// Human-readable capsule title.
    pub title: String,
    /// Lifecycle invariant the capsule proves.
    pub lifecycle_invariant: String,
    /// Failure ids expected from the seeded bug path.
    pub expected_failure_ids: Vec<String>,
}

/// Complete report for one seeded-bug capsule run.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeededBugReport {
    /// Stable capsule name.
    pub name: String,
    /// Human-readable capsule title.
    pub title: String,
    /// Lifecycle invariant the capsule proves.
    pub lifecycle_invariant: String,
    /// Failure ids expected from the seeded bug path.
    pub expected_failure_ids: Vec<String>,
    /// Whether all expected failures appeared in the seeded bug path.
    pub expected_failures_detected: bool,
    /// Overall capsule status: `pass` or `fail`.
    pub status: String,
    /// Trellis-backed success path.
    pub success_path: SeededBugRun,
    /// Seeded broken host or consumer path.
    pub seeded_bug_path: SeededBugRun,
}

/// One path inside a seeded-bug capsule.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeededBugRun {
    /// Path mode, usually `trellis` or `naive`.
    pub mode: String,
    /// Whether the path produced no invariant failures.
    pub passed: bool,
    /// Stable trace label for the exercised scenario.
    pub trace_label: String,
    /// Number of committed Trellis transactions inspected by the path.
    pub transaction_count: usize,
    /// Invariant failures detected for this path.
    pub failed_checks: Vec<SeededBugFailure>,
}

/// One invariant failure from a seeded-bug path.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeededBugFailure {
    /// Stable failure id.
    pub id: String,
    /// Human-readable failure label.
    pub label: String,
    /// Ledger, audit, or oracle that detected the failure.
    pub source: String,
    /// Deterministic failure detail.
    pub details: String,
    /// Full failure text intended for CLI output.
    pub failure_text: String,
}

pub(crate) fn capsule(
    name: &str,
    title: &str,
    lifecycle_invariant: &str,
    expected_failure_ids: &[&str],
) -> SeededBugCapsule {
    SeededBugCapsule {
        name: name.to_owned(),
        title: title.to_owned(),
        lifecycle_invariant: lifecycle_invariant.to_owned(),
        expected_failure_ids: expected_failure_ids
            .iter()
            .map(|id| (*id).to_owned())
            .collect(),
    }
}

pub(crate) fn report(
    capsule: SeededBugCapsule,
    success_path: SeededBugRun,
    seeded_bug_path: SeededBugRun,
) -> SeededBugReport {
    let expected_failures_detected = capsule.expected_failure_ids.iter().all(|id| {
        seeded_bug_path
            .failed_checks
            .iter()
            .any(|failure| failure.id == *id)
    });
    let status = if success_path.passed && !seeded_bug_path.passed && expected_failures_detected {
        "pass"
    } else {
        "fail"
    };

    SeededBugReport {
        name: capsule.name,
        title: capsule.title,
        lifecycle_invariant: capsule.lifecycle_invariant,
        expected_failure_ids: capsule.expected_failure_ids,
        expected_failures_detected,
        status: status.to_owned(),
        success_path,
        seeded_bug_path,
    }
}

pub(crate) fn run(
    mode: &str,
    trace_label: &str,
    transaction_count: usize,
    failed_checks: Vec<SeededBugFailure>,
) -> SeededBugRun {
    SeededBugRun {
        mode: mode.to_owned(),
        passed: failed_checks.is_empty(),
        trace_label: trace_label.to_owned(),
        transaction_count,
        failed_checks,
    }
}

pub(crate) fn failure(
    id: &str,
    label: &str,
    source: &str,
    lifecycle_invariant: &str,
    details: impl Into<String>,
) -> SeededBugFailure {
    let details = details.into();
    SeededBugFailure {
        id: id.to_owned(),
        label: label.to_owned(),
        source: source.to_owned(),
        failure_text: format!("{source} violated {lifecycle_invariant}: {details}"),
        details,
    }
}
