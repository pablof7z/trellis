use trellis_core::{OutputFrameKindTrace, ScopeLifecycleKind};
use trellis_examples::{
    collab_canvas::document_lifecycle_showcase_trace,
    fleetpulse::revoke_permission_showcase_trace,
    mini_language_server::delete_file_showcase_trace,
    plugin_host::capability_lifecycle_showcase_trace,
    showcase_trace::{SHOWCASE_TRACE_CONTRACT, SHOWCASE_TRACE_FORMAT_VERSION, ShowcaseTrace},
    workspace_sync_board::switch_workspace_showcase_trace,
};

#[test]
fn workspace_sync_script_emits_contract_trace() {
    let trace = switch_workspace_showcase_trace();
    assert_common_contract(&trace, "workspace-sync-board", "switch-workspace");
    assert_eq!(trace.steps[0].name, "switch-workspace");
    assert!(trace.steps[0].trace.resource_commands.len() >= 3);
    assert!(trace.steps.iter().all(|step| step.host_statuses.is_empty()));
    assert_has_material_output(&trace);
    assert_has_closed_scope(&trace);
    assert_json_round_trips(&trace);
}

#[test]
fn mini_language_server_script_emits_contract_trace() {
    let trace = delete_file_showcase_trace();
    assert_common_contract(&trace, "mini-language-server", "delete-file");
    assert_eq!(trace.steps[0].name, "delete-file");
    assert!(!trace.steps[0].trace.resource_commands.is_empty());
    assert!(trace.steps.iter().all(|step| step.host_statuses.is_empty()));
    assert_has_material_output(&trace);
    assert_has_closed_scope(&trace);
    assert_json_round_trips(&trace);
}

#[test]
fn fleetpulse_script_emits_contract_trace() {
    let trace = revoke_permission_showcase_trace();
    assert_common_contract(&trace, "fleetpulse", "revoke-permission");
    assert_eq!(trace.steps[0].name, "revoke-permission");
    assert_eq!(trace.steps[1].name, "late-closed-topic-status");
    assert!(!trace.steps[1].host_statuses.is_empty());
    assert!(!trace.steps[0].trace.resource_commands.is_empty());
    assert_has_material_output(&trace);
    assert_has_closed_scope(&trace);
    assert_json_round_trips(&trace);
}

#[test]
fn collab_canvas_script_emits_contract_trace() {
    let trace = document_lifecycle_showcase_trace();
    assert_common_contract(&trace, "collab-canvas", "document-lifecycle");
    assert_eq!(trace.steps[0].name, "show-attachment");
    assert_eq!(trace.steps[2].name, "hide-attachment");
    assert!(!trace.steps[0].trace.resource_commands.is_empty());
    assert_has_material_output(&trace);
    assert_has_closed_scope(&trace);
    assert_json_round_trips(&trace);
}

#[test]
fn plugin_host_script_emits_contract_trace() {
    let trace = capability_lifecycle_showcase_trace();
    assert_common_contract(&trace, "plugin-host", "capability-lifecycle");
    assert_eq!(trace.steps[0].name, "manifest-change");
    assert_eq!(trace.steps[5].name, "disable-plugin");
    assert!(!trace.steps[0].trace.resource_commands.is_empty());
    assert_has_material_output(&trace);
    assert_has_closed_scope(&trace);
    assert_json_round_trips(&trace);
}

fn assert_common_contract(trace: &ShowcaseTrace, showcase: &str, script: &str) {
    assert_eq!(trace.contract, SHOWCASE_TRACE_CONTRACT);
    assert_eq!(trace.format_version, SHOWCASE_TRACE_FORMAT_VERSION);
    assert_eq!(trace.showcase, showcase);
    assert_eq!(trace.script, script);
    assert_eq!(trace.replay.status, "passed");
    assert_eq!(trace.replay.compared_runs, 2);
    assert_eq!(trace.seeded_bug.status, "not_included");
    assert_eq!(trace.seeded_bug.issue, "#93");
    assert!(trace.steps.len() >= 2);
    assert!(
        trace
            .command
            .ends_with(&["--script".to_owned(), script.to_owned()])
    );

    for step in &trace.steps {
        assert!(
            step.trace
                .invariant_results
                .iter()
                .any(
                    |invariant| invariant.name == "incremental_equals_full_recompute"
                        && invariant.passed
                )
        );
    }
}

fn assert_has_material_output(trace: &ShowcaseTrace) {
    assert!(trace.steps.iter().any(|step| {
        step.trace.output_frames.iter().any(|frame| {
            matches!(
                frame.kind,
                OutputFrameKindTrace::Baseline
                    | OutputFrameKindTrace::Delta
                    | OutputFrameKindTrace::Rebaseline(_)
            )
        })
    }));
}

fn assert_has_closed_scope(trace: &ShowcaseTrace) {
    assert!(trace.steps.iter().any(|step| {
        step.trace
            .scope_events
            .iter()
            .any(|event| event.kind == ScopeLifecycleKind::Closed)
    }));
}

fn assert_json_round_trips(trace: &ShowcaseTrace) {
    let json = serde_json::to_string_pretty(trace).unwrap();
    let decoded = serde_json::from_str::<ShowcaseTrace>(&json).unwrap();
    assert_eq!(&decoded, trace);
}
