use crate::engine::{dispatch_action, initial_app_state};
use crate::types::{AppState, InvariantCheck, ObservableState, ReplayResult};

pub fn replay_current_trace(state: &AppState) -> ReplayResult {
    let mut replayed = initial_app_state();
    replayed.mode = state.mode.clone();
    replayed.bug_policy = state.bug_policy.clone();
    for action in &state.action_log {
        replayed = dispatch_action(replayed, action.clone());
    }
    let checks = vec![
        replay_check(
            "same transaction trace",
            replayed.traces.len() == state.traces.len(),
        ),
        replay_check(
            "same resource plans",
            resource_signature(&replayed) == resource_signature(state),
        ),
        replay_check(
            "same output frames",
            output_signature(&replayed) == output_signature(state),
        ),
        replay_check(
            "same final output state",
            observable(&replayed) == observable(state),
        ),
        replay_check(
            "same invariant results",
            invariant_signature(&replayed) == invariant_signature(state),
        ),
    ];
    ReplayResult {
        status: if checks.iter().all(|check| check.status == "pass") {
            "pass"
        } else {
            "fail"
        }
        .to_owned(),
        trace_length: state.traces.len(),
        final_observable_matches: observable(&replayed) == observable(state),
        checks,
    }
}

fn observable(state: &AppState) -> ObservableState {
    let mut resources = state.resource_ledger.values().cloned().collect::<Vec<_>>();
    resources.sort_by(|a, b| a.key.cmp(&b.key));
    ObservableState {
        diagnostics_by_file: state.output_ledger.diagnostics_by_file.clone(),
        links_by_file: state.output_ledger.links_by_file.clone(),
        tokens_by_file: state.output_ledger.tokens_by_file.clone(),
        watchers: resources
            .iter()
            .filter(|r| r.state == "open" && r.key.contains("Watch"))
            .map(|r| r.key.clone())
            .collect(),
        active_jobs: resources
            .iter()
            .filter(|r| r.state == "open" && r.key.starts_with("AnalysisJob("))
            .map(|r| r.key.clone())
            .collect(),
        resources,
    }
}

fn replay_check(label: &str, ok: bool) -> InvariantCheck {
    InvariantCheck {
        id: label.replace(' ', "-"),
        label: label.to_owned(),
        status: if ok { "pass" } else { "fail" }.to_owned(),
        details: if ok {
            String::new()
        } else {
            "Replay diverged".to_owned()
        },
    }
}

fn resource_signature(state: &AppState) -> Vec<String> {
    state
        .traces
        .iter()
        .flat_map(|trace| {
            trace
                .resource_commands
                .iter()
                .map(|cmd| format!("{}:{}", cmd.op, cmd.key))
        })
        .collect()
}

fn output_signature(state: &AppState) -> Vec<String> {
    state
        .traces
        .iter()
        .flat_map(|trace| {
            trace
                .output_frames
                .iter()
                .map(|frame| format!("{}:{}", frame.kind, frame.output_key))
        })
        .collect()
}

fn invariant_signature(state: &AppState) -> Vec<String> {
    state
        .traces
        .iter()
        .flat_map(|trace| {
            trace
                .invariant_checks
                .iter()
                .map(|check| format!("{}:{}", check.id, check.status))
        })
        .collect()
}
