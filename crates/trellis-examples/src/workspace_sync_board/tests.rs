use super::engine::{columns, org_workspace_params, personal_params};
use super::*;

#[test]
fn workspace_switch_closes_old_windows_opens_new_and_rebaselines() {
    let mut app = WorkspaceBoardApp::default();
    let handle = app.open_workspace_board(org_workspace_params("org-a", "workspace-a"));
    app.drain_sync_effects();
    app.drain_output(handle);
    app.drain_diagnostic_traces();

    let update = app.apply_user_event(
        handle,
        WorkspaceBoardEvent::SwitchView(org_workspace_params("org-b", "workspace-b")),
    );
    assert!(update.emitted_effects > 0);
    assert!(update.emitted_frames > 0);

    let effects = app.drain_sync_effects();
    assert!(effects.contains(&SyncEffect::Close(SyncTarget::Project {
        project_id: "backend".to_owned()
    })));
    assert!(effects.contains(&SyncEffect::Open(SyncTarget::Project {
        project_id: "mobile".to_owned()
    })));
    assert!(effects.contains(&SyncEffect::Close(SyncTarget::Profile {
        user: "alex".to_owned()
    })));
    assert!(effects.contains(&SyncEffect::Open(SyncTarget::Profile {
        user: "riley".to_owned()
    })));

    let frames = app.drain_output(handle);
    assert!(matches!(
        &frames[0],
        BoardFrame::Rebaseline(snapshot)
            if snapshot.project_ids() == ["mobile".to_owned()].into_iter().collect()
    ));
    assert_oracle_trace(&mut app);
}

#[test]
fn permission_revoke_withdraws_windows_and_clears_rows() {
    let mut app = WorkspaceBoardApp::default();
    let handle = app.open_workspace_board(org_workspace_params("org-a", "workspace-a"));
    app.drain_sync_effects();
    app.drain_output(handle);
    app.drain_diagnostic_traces();

    let update = app.apply_user_event(
        handle,
        WorkspaceBoardEvent::RevokeProjectPermission {
            project_id: "backend".to_owned(),
        },
    );
    assert!(update.emitted_effects > 0);

    let effects = app.drain_sync_effects();
    assert!(effects.contains(&SyncEffect::Close(SyncTarget::Project {
        project_id: "backend".to_owned()
    })));
    assert!(effects.contains(&SyncEffect::Close(SyncTarget::Comments {
        project_id: "backend".to_owned()
    })));

    let frames = app.drain_output(handle);
    assert!(matches!(&frames[0], BoardFrame::Delta(snapshot) if snapshot.is_empty()));
    assert_oracle_trace(&mut app);
}

#[test]
fn empty_workspace_opens_no_windows() {
    let mut app = WorkspaceBoardApp::default();
    let handle = app.open_workspace_board(org_workspace_params("org-missing", "empty"));
    assert!(app.drain_sync_effects().is_empty());
    let frames = app.drain_output(handle);
    assert!(snapshot(&frames[0]).is_empty());
    assert_oracle_trace(&mut app);
}

#[test]
fn personal_view_uses_assigned_issue_projects() {
    let mut app = WorkspaceBoardApp::default();
    let handle = app.open_workspace_board(personal_params());

    let effects = app.drain_sync_effects();
    assert!(effects.contains(&SyncEffect::Open(SyncTarget::Project {
        project_id: "backend".to_owned()
    })));
    assert!(effects.contains(&SyncEffect::Open(SyncTarget::Project {
        project_id: "docs".to_owned()
    })));
    assert!(!effects.contains(&SyncEffect::Open(SyncTarget::Project {
        project_id: "mobile".to_owned()
    })));

    let frames = app.drain_output(handle);
    assert_eq!(
        snapshot(&frames[0]).project_ids(),
        ["backend".to_owned(), "docs".to_owned()]
            .into_iter()
            .collect()
    );
}

#[test]
fn column_filter_change_emits_rebaseline() {
    let mut app = WorkspaceBoardApp::default();
    let handle = app.open_workspace_board(org_workspace_params("org-a", "workspace-a"));
    app.drain_sync_effects();
    app.drain_output(handle);
    app.drain_diagnostic_traces();

    let update = app.apply_user_event(
        handle,
        WorkspaceBoardEvent::SetVisibleColumns(columns([BoardColumn::Todo])),
    );
    assert_eq!(update.emitted_effects, 0);
    assert_eq!(update.emitted_frames, 1);

    let frames = app.drain_output(handle);
    assert!(matches!(
        &frames[0],
        BoardFrame::Rebaseline(snapshot)
            if snapshot.columns.keys().eq([&BoardColumn::Todo])
                && snapshot.project_ids() == ["backend".to_owned()].into_iter().collect()
    ));
    assert_oracle_trace(&mut app);
}

#[test]
fn close_tears_down_scope_and_clears_output() {
    let mut app = WorkspaceBoardApp::default();
    let handle = app.open_workspace_board(org_workspace_params("org-a", "workspace-a"));
    app.drain_sync_effects();
    app.drain_output(handle);
    app.drain_diagnostic_traces();

    let update = app.close(handle);
    assert!(update.emitted_effects > 0);
    assert_eq!(update.emitted_frames, 1);
    assert!(matches!(app.drain_output(handle), frames if frames == vec![BoardFrame::Cleared]));
    let traces = app.drain_diagnostic_traces();
    assert!(
        traces[0]
            .scope_events
            .iter()
            .any(|event| { event.kind == trellis_core::ScopeLifecycleKind::Closed })
    );
}

fn assert_oracle_trace(app: &mut WorkspaceBoardApp) {
    let traces = app.drain_diagnostic_traces();
    assert!(traces.iter().any(|trace| {
        trace.invariant_results.iter().any(|invariant| {
            invariant.name == "incremental_equals_full_recompute" && invariant.passed
        })
    }));
}

fn snapshot(frame: &BoardFrame) -> &BoardSnapshot {
    match frame {
        BoardFrame::Baseline(snapshot)
        | BoardFrame::Delta(snapshot)
        | BoardFrame::Rebaseline(snapshot) => snapshot,
        BoardFrame::Cleared => panic!("expected a board snapshot frame"),
    }
}
