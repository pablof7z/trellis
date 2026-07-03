use serde::Serialize;

use crate::engine::{dispatch_action, initial_app_state};
use crate::types::{Action, AppState, InvariantCheck};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BugCapsule {
    pub name: String,
    pub title: String,
    pub lifecycle_invariant: String,
    pub expected_failure_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BugCapsuleReport {
    pub name: String,
    pub title: String,
    pub lifecycle_invariant: String,
    pub expected_failure_ids: Vec<String>,
    pub expected_failures_detected: bool,
    pub status: String,
    pub success_path: CapsuleRun,
    pub seeded_bug_path: CapsuleRun,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapsuleRun {
    pub mode: String,
    pub passed: bool,
    pub trace_label: String,
    pub transaction_count: usize,
    pub failed_checks: Vec<CapsuleFailure>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapsuleFailure {
    pub id: String,
    pub label: String,
    pub source: String,
    pub details: String,
    pub failure_text: String,
}

struct CapsuleSpec {
    name: &'static str,
    title: &'static str,
    lifecycle_invariant: &'static str,
    expected_failure_ids: &'static [&'static str],
    enabled_bug_keys: &'static [&'static str],
    actions: fn() -> Vec<Action>,
}

pub fn available_bug_capsules() -> Vec<BugCapsule> {
    capsule_specs()
        .iter()
        .map(|spec| BugCapsule {
            name: spec.name.to_owned(),
            title: spec.title.to_owned(),
            lifecycle_invariant: spec.lifecycle_invariant.to_owned(),
            expected_failure_ids: spec
                .expected_failure_ids
                .iter()
                .map(|id| (*id).to_owned())
                .collect(),
        })
        .collect()
}

pub fn run_all_bug_capsules() -> Vec<BugCapsuleReport> {
    capsule_specs().iter().map(run_spec).collect()
}

pub fn run_bug_capsule(name: &str) -> Option<BugCapsuleReport> {
    capsule_specs()
        .iter()
        .find(|spec| spec.name == name)
        .map(run_spec)
}

fn run_spec(spec: &CapsuleSpec) -> BugCapsuleReport {
    let success_path = run_path("trellis", &[], (spec.actions)());
    let seeded_bug_path = run_path("naive", spec.enabled_bug_keys, (spec.actions)());
    let expected_failures_detected = spec.expected_failure_ids.iter().all(|id| {
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

    BugCapsuleReport {
        name: spec.name.to_owned(),
        title: spec.title.to_owned(),
        lifecycle_invariant: spec.lifecycle_invariant.to_owned(),
        expected_failure_ids: spec
            .expected_failure_ids
            .iter()
            .map(|id| (*id).to_owned())
            .collect(),
        expected_failures_detected,
        status: status.to_owned(),
        success_path,
        seeded_bug_path,
    }
}

fn run_path(mode: &str, enabled_bug_keys: &[&str], actions: Vec<Action>) -> CapsuleRun {
    let mut state = dispatch_action(initial_app_state(), set_mode(mode));
    if mode == "naive" {
        for key in ALL_BUG_KEYS {
            state = dispatch_action(state, set_bug(key, false));
        }
        for key in enabled_bug_keys {
            state = dispatch_action(state, set_bug(key, true));
        }
    }
    for action in actions {
        state = dispatch_action(state, action);
    }
    capsule_run(mode, &state)
}

fn capsule_run(mode: &str, state: &AppState) -> CapsuleRun {
    let trace = state.traces.last().expect("capsule action emits a trace");
    let failed_checks = trace
        .invariant_checks
        .iter()
        .filter(|check| check.status != "pass")
        .map(capsule_failure)
        .collect::<Vec<_>>();

    CapsuleRun {
        mode: mode.to_owned(),
        passed: failed_checks.is_empty(),
        trace_label: trace.label.clone(),
        transaction_count: state.traces.len(),
        failed_checks,
    }
}

fn capsule_failure(check: &InvariantCheck) -> CapsuleFailure {
    let source = failure_source(&check.id);
    let details = if check.details.is_empty() {
        "No details supplied by invariant check.".to_owned()
    } else {
        check.details.clone()
    };
    CapsuleFailure {
        id: check.id.clone(),
        label: check.label.clone(),
        source: source.to_owned(),
        failure_text: format!("{} violated {}: {}", source, check.label, details),
        details,
    }
}

fn failure_source(id: &str) -> &'static str {
    match id {
        "no-watcher-for-removed-files"
        | "no-analysis-job-for-removed-files"
        | "closed-scope-owns-no-resources" => "ResourceLedger",
        "no-diagnostics-for-removed-files"
        | "no-document-links-for-removed-files"
        | "no-semantic-tokens-for-removed-files" => "OutputLedger",
        "incremental-equals-full-recompute" => "FullRecomputeOracle",
        "stale-host-status-no-output" => "HostStatusAudit",
        _ => "InvariantCheck",
    }
}

fn capsule_specs() -> [CapsuleSpec; 3] {
    [
        CapsuleSpec {
            name: "delete-file-lifecycle",
            title: "Delete file clears outputs and closes owned resources",
            lifecycle_invariant: "Removed source files leave no diagnostics, watchers, jobs, or scope-owned resources.",
            expected_failure_ids: &[
                "no-diagnostics-for-removed-files",
                "no-watcher-for-removed-files",
                "closed-scope-owns-no-resources",
                "incremental-equals-full-recompute",
            ],
            enabled_bug_keys: &["skipClearDiagnosticsForDeletedFile", "skipWatcherClose"],
            actions: delete_file_actions,
        },
        CapsuleSpec {
            name: "rename-schema-output-rebaseline",
            title: "Import rename rebaselines document links",
            lifecycle_invariant: "Output ledgers must match the full recompute oracle after import graph changes.",
            expected_failure_ids: &["incremental-equals-full-recompute"],
            enabled_bug_keys: &["skipDocumentLinkRebaseline"],
            actions: rename_schema_actions,
        },
        CapsuleSpec {
            name: "stale-analysis-status",
            title: "Stale host status cannot mutate output",
            lifecycle_invariant: "A host result from an obsolete command revision is audit-only and emits no output frame.",
            expected_failure_ids: &[
                "stale-host-status-no-output",
                "incremental-equals-full-recompute",
            ],
            enabled_bug_keys: &["acceptStaleAnalysisResults"],
            actions: stale_analysis_actions,
        },
    ]
}

const ALL_BUG_KEYS: &[&str] = &[
    "skipClearDiagnosticsForDeletedFile",
    "skipDocumentLinkRebaseline",
    "skipWatcherClose",
    "acceptStaleAnalysisResults",
    "skipScopeCloseOutputClear",
];

fn set_mode(mode: &str) -> Action {
    Action::SetMode {
        mode: mode.to_owned(),
    }
}

fn set_bug(key: &str, value: bool) -> Action {
    Action::SetBug {
        key: key.to_owned(),
        value,
    }
}

fn delete_file_actions() -> Vec<Action> {
    vec![Action::DeleteFile {
        path: "src/legacy_user.tl".to_owned(),
    }]
}

fn rename_schema_actions() -> Vec<Action> {
    vec![Action::RenameSchema]
}

fn stale_analysis_actions() -> Vec<Action> {
    vec![
        Action::StartSlowAnalysis,
        Action::FixApp,
        Action::InjectStaleAnalysisResult,
    ]
}
