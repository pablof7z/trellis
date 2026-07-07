use crate::showcase_trace::{ShowcaseStep, ShowcaseTrace, build_showcase_trace};

use super::WorkspaceBoardApp;
use super::engine::org_workspace_params;
use super::types::{WorkspaceBoardEvent, WorkspaceDataset};

/// Runs the headless `switch-workspace` showcase script.
pub fn switch_workspace_showcase_trace() -> ShowcaseTrace {
    build_showcase_trace(
        "workspace-sync-board",
        "switch-workspace",
        &[
            "cargo",
            "run",
            "-p",
            "trellis-examples",
            "--example",
            "workspace_sync_board",
            "--",
            "--script",
            "switch-workspace",
        ],
        || {
            let mut app = WorkspaceBoardApp::new(WorkspaceDataset::sample());
            let handle = app.open_workspace_board(org_workspace_params("org-a", "workspace-a"));
            app.drain_sync_effects();
            app.drain_output(handle);
            app.drain_diagnostic_traces();

            app.apply_user_event(
                handle,
                WorkspaceBoardEvent::SwitchView(org_workspace_params("org-b", "workspace-b")),
            );
            let switch_trace = app
                .drain_diagnostic_traces()
                .pop()
                .expect("switch emits one trace");

            app.close(handle);
            let close_trace = app
                .drain_diagnostic_traces()
                .pop()
                .expect("close emits one trace");

            vec![
                ShowcaseStep {
                    name: "switch-workspace".to_owned(),
                    host_statuses: Vec::new(),
                    trace: switch_trace,
                },
                ShowcaseStep {
                    name: "close-board".to_owned(),
                    host_statuses: Vec::new(),
                    trace: close_trace,
                },
            ]
        },
    )
}
