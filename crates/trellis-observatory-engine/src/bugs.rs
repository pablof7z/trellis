use crate::types::{Action, AppState, Diagnostic, HostStatus, HostStatusEvent};

pub(crate) fn apply_naive_bugs(
    commands: &mut Vec<crate::types::ResourceCommand>,
    frames: &mut Vec<crate::types::OutputFrame>,
    state: &AppState,
    action: &Action,
    host_events: &mut [HostStatusEvent],
) -> bool {
    if state.mode != "naive" {
        return false;
    }
    if state.bug_policy.skip_watcher_close {
        commands.retain(|cmd| !(cmd.op == "Close" && cmd.key.starts_with("WatchFile(")));
    }
    if state.bug_policy.skip_clear_diagnostics_for_deleted_file {
        frames.retain(|frame| frame.kind != "ClearDiagnostics");
    }
    if state.bug_policy.skip_document_link_rebaseline && matches!(action, Action::RenameSchema) {
        frames.retain(|frame| !frame.output_key.contains("DocumentLinks:src/app.tl"));
    }
    if state.bug_policy.accept_stale_analysis_results
        && matches!(action, Action::InjectStaleAnalysisResult)
    {
        for event in host_events {
            event.classification = "accepted_by_naive_callback".to_owned();
            event.effect = "published stale diagnostics".to_owned();
        }
        frames.push(stale_diagnostic_frame(state.inputs.scenario_revision));
        return true;
    }
    false
}

pub(crate) fn host_status_events(
    state: &AppState,
    action: &Action,
    revision: u32,
) -> Vec<HostStatusEvent> {
    if !matches!(action, Action::InjectStaleAnalysisResult) {
        return Vec::new();
    }
    let status = state.inputs.host_statuses.last().cloned().unwrap();
    let current = state
        .analysis_revisions
        .get(&status.path)
        .copied()
        .unwrap_or(revision);
    let classification = if status.command_revision == current {
        "accepted"
    } else {
        "stale_command_revision"
    };
    vec![HostStatusEvent {
        status,
        classification: classification.to_owned(),
        reason: format!("current analysis command revision for src/app.tl is rev{current}"),
        effect: if classification == "accepted" {
            "may publish output"
        } else {
            "no output mutation"
        }
        .to_owned(),
    }]
}

pub(crate) fn stale_status(command_revision: u32, status_revision: u32) -> HostStatus {
    HostStatus {
        kind: "AnalysisFinished".to_owned(),
        path: "src/app.tl".to_owned(),
        command_revision,
        diagnostics: vec![Diagnostic {
            id: "stale:src/app.tl".to_owned(),
            file_path: "src/app.tl".to_owned(),
            line: 7,
            column: 13,
            severity: "error".to_owned(),
            message: "Stale analysis result from cancelled app job".to_owned(),
            source: "typecheck".to_owned(),
        }],
        error: None,
        status_revision,
    }
}

fn stale_diagnostic_frame(revision: u32) -> crate::types::OutputFrame {
    let mut frame = crate::compute::frame(
        "BaselineDiagnostics",
        "src/app.tl",
        revision,
        "naive accepted stale host result",
    );
    frame.diagnostics = stale_status(revision.saturating_sub(1), revision).diagnostics;
    frame
}
