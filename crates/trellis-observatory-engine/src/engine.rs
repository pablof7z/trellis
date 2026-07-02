use std::collections::BTreeMap;

use crate::actions::{mutate_inputs, set_bug};
use crate::bugs::{apply_naive_bugs, host_status_events};
use crate::compute::{diff_vec, full_recompute, input_change, output_frames};
use crate::invariants::invariants;
use crate::ledger::resource_commands;
use crate::ledger::{apply_output_frames, apply_resource_commands, empty_output_ledger};
use crate::seed::initial_inputs;
use crate::types::{
    Action, AppState, ChangedNode, CollectionDiff, NaiveBugPolicy, ScopeEvent, TransactionTrace,
};

pub fn initial_app_state() -> AppState {
    let inputs = initial_inputs();
    let analysis_revisions = inputs
        .files
        .keys()
        .map(|path| (path.clone(), inputs.scenario_revision))
        .collect::<BTreeMap<_, _>>();
    let full = full_recompute(&inputs, &analysis_revisions);
    let mut state = AppState {
        mode: "trellis".to_owned(),
        bug_policy: NaiveBugPolicy {
            skip_clear_diagnostics_for_deleted_file: true,
            skip_document_link_rebaseline: true,
            skip_watcher_close: true,
            accept_stale_analysis_results: true,
            skip_scope_close_output_clear: false,
        },
        inputs,
        full: empty_full(),
        resource_ledger: BTreeMap::new(),
        output_ledger: empty_output_ledger(),
        traces: Vec::new(),
        action_log: Vec::new(),
        analysis_revisions,
        closed_scopes: Vec::new(),
        selected_why: None,
        replay_result: None,
    };
    let mut bootstrap = TransactionTrace {
        tx_id: 1,
        revision: 1,
        label: "Open main branch".to_owned(),
        input_changes: vec![input_change("activeBranch", "none", "main")],
        changed_nodes: changed_nodes(),
        collection_diffs: vec![diff_vec("sourceFiles", &[], &full.source_files)],
        resource_commands: resource_commands(&[], &full.desired_resources, 1, "Open main branch"),
        output_frames: output_frames(&empty_full(), &full, 1, "Open main branch"),
        scope_events: full
            .source_files
            .iter()
            .map(|path| ScopeEvent {
                op: "Open".to_owned(),
                scope: format!("FileScope({path})"),
                reason: "source file entered graph".to_owned(),
            })
            .collect(),
        host_status_events: Vec::new(),
        invariant_checks: Vec::new(),
        audit_edges: audit_edges(),
    };
    state.full = full;
    apply_resource_commands(&mut state.resource_ledger, &bootstrap.resource_commands, 1);
    apply_output_frames(&mut state.output_ledger, &bootstrap.output_frames);
    bootstrap.invariant_checks = invariants(
        &state.full,
        &state.resource_ledger,
        &state.output_ledger,
        &state.closed_scopes,
        false,
    );
    state.traces.push(bootstrap);
    state
}

pub fn dispatch_action(mut state: AppState, action: Action) -> AppState {
    match action {
        Action::Reset => initial_app_state(),
        Action::SetMode { mode } => {
            state.mode = mode;
            state.replay_result = None;
            state
        }
        Action::SetBug { key, value } => {
            set_bug(&mut state.bug_policy, &key, value);
            state
        }
        Action::SelectWhy { id } => {
            state.selected_why = id;
            state
        }
        other => apply_transaction(state, other),
    }
}

fn apply_transaction(mut state: AppState, action: Action) -> AppState {
    let before_inputs = state.inputs.clone();
    let before_full = state.full.clone();
    let label = mutate_inputs(&mut state, &action);
    state.inputs.scenario_revision += 1;
    let revision = state.inputs.scenario_revision;
    let tx_id = state.traces.len() as u32 + 1;
    let after_full = full_recompute(&state.inputs, &state.analysis_revisions);
    let mut commands = resource_commands(
        &before_full.desired_resources,
        &after_full.desired_resources,
        revision,
        &label,
    );
    let mut frames = output_frames(&before_full, &after_full, revision, &label);
    let mut host_events = host_status_events(&state, &action, revision);
    let stale_mutated = apply_naive_bugs(
        &mut commands,
        &mut frames,
        &state,
        &action,
        &mut host_events,
    );
    let source_diff = diff_vec(
        "sourceFiles",
        &before_full.source_files,
        &after_full.source_files,
    );
    for removed in &source_diff.removed {
        let scope = format!("FileScope({removed})");
        if !state.closed_scopes.contains(&scope) {
            state.closed_scopes.push(scope);
        }
    }
    apply_resource_commands(&mut state.resource_ledger, &commands, tx_id);
    apply_output_frames(&mut state.output_ledger, &frames);
    state.full = after_full.clone();
    let mut trace = TransactionTrace {
        tx_id,
        revision,
        label: label.clone(),
        input_changes: input_changes(&before_inputs, &state.inputs, &action),
        changed_nodes: changed_nodes(),
        collection_diffs: collection_diffs(&before_full, &after_full),
        resource_commands: commands,
        output_frames: frames,
        scope_events: scope_events(&source_diff),
        host_status_events: host_events,
        invariant_checks: Vec::new(),
        audit_edges: audit_edges(),
    };
    trace.invariant_checks = invariants(
        &state.full,
        &state.resource_ledger,
        &state.output_ledger,
        &state.closed_scopes,
        stale_mutated,
    );
    state.traces.push(trace);
    state.action_log.push(action);
    state.replay_result = None;
    state
}

fn input_changes(
    before: &crate::types::CanonicalInputs,
    after: &crate::types::CanonicalInputs,
    action: &Action,
) -> Vec<crate::types::InputChange> {
    match action {
        Action::SwitchBranch { .. } => vec![input_change(
            "activeBranch",
            &before.active_branch,
            &after.active_branch,
        )],
        Action::ChangeConfig { .. } => vec![input_change(
            "compilerConfig",
            format!("{:?}", before.compiler_config),
            format!("{:?}", after.compiler_config),
        )],
        Action::ToggleGenerated => vec![input_change(
            "generatedFilesEnabled",
            before.generated_files_enabled,
            after.generated_files_enabled,
        )],
        Action::InjectStaleAnalysisResult => vec![input_change(
            "hostStatuses",
            before.host_statuses.len(),
            after.host_statuses.len(),
        )],
        Action::CloseAppTab => vec![input_change(
            "openEditors",
            before.open_editors.join(","),
            after.open_editors.join(","),
        )],
        _ => vec![input_change(
            "files",
            format!("{} files", before.files.len()),
            format!("{} files", after.files.len()),
        )],
    }
}

fn collection_diffs(
    before: &crate::types::FullState,
    after: &crate::types::FullState,
) -> Vec<CollectionDiff> {
    vec![
        diff_vec("sourceFiles", &before.source_files, &after.source_files),
        diff_vec("moduleGraph", &before.module_graph, &after.module_graph),
        diff_vec("importEdges", &before.import_edges, &after.import_edges),
        diff_vec(
            "desiredResources",
            &before.desired_resources,
            &after.desired_resources,
        ),
    ]
}

fn scope_events(diff: &CollectionDiff) -> Vec<ScopeEvent> {
    let mut events = Vec::new();
    for path in &diff.removed {
        events.push(ScopeEvent {
            op: "Close".to_owned(),
            scope: format!("FileScope({path})"),
            reason: "source file left graph".to_owned(),
        });
    }
    for path in &diff.added {
        events.push(ScopeEvent {
            op: "Open".to_owned(),
            scope: format!("FileScope({path})"),
            reason: "source file entered graph".to_owned(),
        });
    }
    events
}

fn empty_full() -> crate::types::FullState {
    crate::types::FullState {
        source_files: Vec::new(),
        import_edges: Vec::new(),
        diagnostics_by_file: BTreeMap::new(),
        links_by_file: BTreeMap::new(),
        tokens_by_file: BTreeMap::new(),
        desired_resources: Vec::new(),
        module_graph: Vec::new(),
    }
}

fn changed_nodes() -> Vec<ChangedNode> {
    [
        "sourceFiles",
        "parsedFiles",
        "moduleGraph",
        "watcherDemand",
        "analysisDemand",
        "diagnosticsOutputModel",
        "documentLinksOutputModel",
        "semanticTokensOutputModel",
    ]
    .iter()
    .map(|id| ChangedNode {
        id: (*id).to_owned(),
        summary: "recomputed deterministically".to_owned(),
    })
    .collect()
}

fn audit_edges() -> Vec<String> {
    vec![
        "files -> sourceFiles".to_owned(),
        "sourceFiles -> moduleGraph".to_owned(),
        "moduleGraph -> resourcePlan".to_owned(),
        "moduleGraph -> outputFrames".to_owned(),
        "outputFrames -> outputLedger".to_owned(),
        "resourcePlan -> resourceLedger".to_owned(),
    ]
}
