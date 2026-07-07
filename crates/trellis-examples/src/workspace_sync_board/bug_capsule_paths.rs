use std::collections::BTreeSet;

use crate::seeded_bugs::{self, SeededBugFailure, SeededBugRun};

use super::engine::org_workspace_params;
use super::{
    BoardColumn, BoardFrame, BoardSnapshot, IssueRecord, SyncEffect, SyncTarget, WorkspaceBoardApp,
    WorkspaceBoardEvent, WorkspaceBoardHandle,
};

pub(super) fn run_workspace_switch(invariant: &'static str) -> (SeededBugRun, SeededBugRun) {
    let (mut app, handle, _) = open_workspace_a();
    app.apply_user_event(
        handle,
        WorkspaceBoardEvent::SwitchView(org_workspace_params("org-b", "workspace-b")),
    );
    let effects = app.drain_sync_effects();
    let frames = app.drain_output(handle);
    let traces = app.drain_diagnostic_traces();

    let success_failures = switch_close_failures(invariant, &effects);
    let bug_effects = effects
        .iter()
        .filter(|effect| {
            !matches!(
                effect,
                SyncEffect::Close(SyncTarget::Project { project_id })
                    if project_id == "backend"
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    let bug_failures = switch_close_failures(invariant, &bug_effects);

    (
        seeded_bugs::run(
            "trellis",
            "workspace-switch",
            traces.len(),
            success_failures,
        ),
        seeded_bugs::run(
            "naive",
            "workspace-switch",
            traces.len(),
            bug_failures_with_frame_guard(bug_failures, invariant, &frames, "mobile"),
        ),
    )
}

pub(super) fn run_permission_revoke(invariant: &'static str) -> (SeededBugRun, SeededBugRun) {
    let (mut app, handle, initial) = open_workspace_a();
    app.apply_user_event(
        handle,
        WorkspaceBoardEvent::RevokeProjectPermission {
            project_id: "backend".to_owned(),
        },
    );
    let frames = app.drain_output(handle);
    let traces = app.drain_diagnostic_traces();

    let success_failures = revoked_rows_failures(invariant, frame_snapshot(&frames));
    let bug_failures = revoked_rows_failures(invariant, Some(&initial));

    (
        seeded_bugs::run(
            "trellis",
            "workspace-revoke",
            traces.len(),
            success_failures,
        ),
        seeded_bugs::run("naive", "workspace-revoke", traces.len(), bug_failures),
    )
}

pub(super) fn run_empty_workspace(invariant: &'static str) -> (SeededBugRun, SeededBugRun) {
    let mut app = WorkspaceBoardApp::default();
    let handle = app.open_workspace_board(org_workspace_params("org-missing", "empty"));
    let effects = app.drain_sync_effects();
    let frames = app.drain_output(handle);
    let traces = app.drain_diagnostic_traces();

    let success_failures = empty_workspace_failures(invariant, &effects, frame_snapshot(&frames));
    let bug_failures = vec![failure(
        "empty-workspace-opens-no-broad-sync",
        "empty workspace opened wildcard sync",
        "ResourceLedger",
        invariant,
        "Naive fallback opened sync/* even though visible projects was empty.",
    )];

    (
        seeded_bugs::run("trellis", "workspace-empty", traces.len(), success_failures),
        seeded_bugs::run("naive", "workspace-empty", traces.len(), bug_failures),
    )
}

pub(super) fn run_removed_issue_delta(invariant: &'static str) -> (SeededBugRun, SeededBugRun) {
    let (mut app, handle, initial) = open_workspace_a();
    app.apply_user_event(
        handle,
        WorkspaceBoardEvent::ReplaceProjectIssues {
            project_id: "backend".to_owned(),
            issues: vec![remaining_backend_issue()],
        },
    );
    let frames = app.drain_output(handle);
    let traces = app.drain_diagnostic_traces();

    let success_failures = removed_issue_failures(invariant, frame_snapshot(&frames));
    let bug_failures = removed_issue_failures(invariant, Some(&initial));

    (
        seeded_bugs::run(
            "trellis",
            "workspace-removed-issue",
            traces.len(),
            success_failures,
        ),
        seeded_bugs::run(
            "naive",
            "workspace-removed-issue",
            traces.len(),
            bug_failures,
        ),
    )
}

fn open_workspace_a() -> (WorkspaceBoardApp, WorkspaceBoardHandle, BoardSnapshot) {
    let mut app = WorkspaceBoardApp::default();
    let handle = app.open_workspace_board(org_workspace_params("org-a", "workspace-a"));
    app.drain_sync_effects();
    let initial = frame_snapshot(&app.drain_output(handle))
        .expect("workspace open emits a snapshot")
        .clone();
    app.drain_diagnostic_traces();
    (app, handle, initial)
}

fn switch_close_failures(invariant: &'static str, effects: &[SyncEffect]) -> Vec<SeededBugFailure> {
    let closed_project = effects.contains(&SyncEffect::Close(SyncTarget::Project {
        project_id: "backend".to_owned(),
    }));
    let closed_comments = effects.contains(&SyncEffect::Close(SyncTarget::Comments {
        project_id: "backend".to_owned(),
    }));

    if closed_project && closed_comments {
        Vec::new()
    } else {
        vec![failure(
            "old-workspace-sync-window-closed",
            "old workspace sync windows closed",
            "ResourceLedger",
            invariant,
            "backend project/comment sync windows remained live after switching to workspace-b.",
        )]
    }
}

fn bug_failures_with_frame_guard(
    mut failures: Vec<SeededBugFailure>,
    invariant: &'static str,
    frames: &[BoardFrame],
    expected_project: &str,
) -> Vec<SeededBugFailure> {
    if !project_ids(frame_snapshot(frames)).contains(expected_project) {
        failures.push(failure(
            "workspace-switch-output-rebaseline",
            "workspace switch rebaselined output",
            "FullRecomputeOracle",
            invariant,
            "workspace switch output did not rebaseline to the selected workspace.",
        ));
    }
    failures
}

fn revoked_rows_failures(
    invariant: &'static str,
    snapshot: Option<&BoardSnapshot>,
) -> Vec<SeededBugFailure> {
    if project_ids(snapshot).contains("backend") {
        vec![failure(
            "revoked-project-rows-cleared",
            "revoked project rows cleared",
            "OutputLedger",
            invariant,
            "backend rows remained visible after backend permission was revoked.",
        )]
    } else {
        Vec::new()
    }
}

fn empty_workspace_failures(
    invariant: &'static str,
    effects: &[SyncEffect],
    snapshot: Option<&BoardSnapshot>,
) -> Vec<SeededBugFailure> {
    if effects.is_empty() && snapshot.is_none_or(BoardSnapshot::is_empty) {
        Vec::new()
    } else {
        vec![failure(
            "empty-workspace-opens-no-broad-sync",
            "empty workspace opened broad sync demand",
            "ResourceLedger",
            invariant,
            "empty workspace emitted sync effects or visible output.",
        )]
    }
}

fn removed_issue_failures(
    invariant: &'static str,
    snapshot: Option<&BoardSnapshot>,
) -> Vec<SeededBugFailure> {
    if snapshot.is_some_and(|snapshot| snapshot_has_issue(snapshot, "B-1")) {
        vec![failure(
            "workspace-delta-matches-full-recompute",
            "removed issue delta matched full recompute",
            "FullRecomputeOracle",
            invariant,
            "issue B-1 remained visible after the project issue set removed it.",
        )]
    } else {
        Vec::new()
    }
}

fn frame_snapshot(frames: &[BoardFrame]) -> Option<&BoardSnapshot> {
    frames.first().and_then(|frame| match frame {
        BoardFrame::Baseline(snapshot)
        | BoardFrame::Delta(snapshot)
        | BoardFrame::Rebaseline(snapshot) => Some(snapshot),
        BoardFrame::Cleared => None,
    })
}

fn project_ids(snapshot: Option<&BoardSnapshot>) -> BTreeSet<String> {
    snapshot.map_or_else(BTreeSet::new, BoardSnapshot::project_ids)
}

fn snapshot_has_issue(snapshot: &BoardSnapshot, issue_id: &str) -> bool {
    snapshot
        .columns
        .values()
        .flatten()
        .any(|row| row.issue_id == issue_id)
}

fn remaining_backend_issue() -> IssueRecord {
    IssueRecord {
        id: "B-2".to_owned(),
        project_id: "backend".to_owned(),
        title: "Ship permission audit".to_owned(),
        column: BoardColumn::Doing,
        assignee: "casey".to_owned(),
    }
}

fn failure(
    id: &str,
    label: &str,
    source: &str,
    invariant: &'static str,
    details: &str,
) -> SeededBugFailure {
    seeded_bugs::failure(id, label, source, invariant, details)
}
