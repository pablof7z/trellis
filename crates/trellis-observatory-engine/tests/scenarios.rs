use trellis_observatory_engine::types::Action;
use trellis_observatory_engine::{dispatch_action, initial_app_state, replay_current_trace};

#[test]
fn delete_file_trellis_passes_and_clears_outputs() {
    let state = dispatch_action(
        initial_app_state(),
        Action::DeleteFile {
            path: "src/legacy_user.tl".to_owned(),
        },
    );
    let last = state.traces.last().unwrap();
    assert!(
        last.invariant_checks
            .iter()
            .all(|check| check.status == "pass")
    );
    assert!(
        !state
            .output_ledger
            .diagnostics_by_file
            .contains_key("src/legacy_user.tl")
    );
    assert!(
        last.resource_commands
            .iter()
            .any(|cmd| cmd.op == "Close" && cmd.key == "WatchFile(src/legacy_user.tl)")
    );
}

#[test]
fn delete_file_trace_is_backed_by_trellis_core() {
    let state = dispatch_action(
        initial_app_state(),
        Action::DeleteFile {
            path: "src/legacy_user.tl".to_owned(),
        },
    );
    let last = state.traces.last().unwrap();
    assert!(last.core_backed);
    assert!(last.core_transaction_id.is_some());
    assert!(last.core_revision.is_some());
    assert!(
        last.audit_edges
            .iter()
            .any(|edge| edge.starts_with("trellis-core::ProduceResourcePlans"))
    );
    assert!(
        last.resource_commands
            .iter()
            .all(|command| command.cause.input_key == "trellis-core transaction")
    );
}

#[test]
fn delete_file_naive_fails_with_stale_diagnostics_and_watcher() {
    let mut state = initial_app_state();
    state.mode = "naive".to_owned();
    let state = dispatch_action(
        state,
        Action::DeleteFile {
            path: "src/legacy_user.tl".to_owned(),
        },
    );
    let last = state.traces.last().unwrap();
    assert!(
        last.invariant_checks
            .iter()
            .any(|check| check.status == "fail")
    );
}

#[test]
fn branch_switch_clears_legacy_diagnostics() {
    let state = dispatch_action(
        initial_app_state(),
        Action::SwitchBranch {
            branch: "feature/schema-v2".to_owned(),
        },
    );
    assert!(state.output_ledger.diagnostics_by_file.is_empty());
}

#[test]
fn late_analysis_result_is_ignored_in_trellis_mode() {
    let state = dispatch_action(initial_app_state(), Action::StartSlowAnalysis);
    let state = dispatch_action(state, Action::FixApp);
    let state = dispatch_action(state, Action::InjectStaleAnalysisResult);
    let event = &state.traces.last().unwrap().host_status_events[0];
    assert_eq!(event.classification, "stale_command_revision");
    assert!(
        state
            .traces
            .last()
            .unwrap()
            .invariant_checks
            .iter()
            .all(|check| check.status == "pass")
    );
}

#[test]
fn replay_matches_trace_and_final_output() {
    let state = dispatch_action(
        initial_app_state(),
        Action::DeleteFile {
            path: "src/legacy_user.tl".to_owned(),
        },
    );
    let replay = replay_current_trace(&state);
    assert_eq!(replay.status, "pass");
}
