use std::collections::VecDeque;

use trellis_core::{OutputFrameKind, ResourceCommand, TransactionResult, TransactionTrace};

use super::graph::{WorkspaceBoardGraph, build_graph, target_from_key};
use super::types::{
    BoardFrame, BoardSnapshot, BoardView, SyncCommand, SyncEffect, WorkspaceBoardEvent,
    WorkspaceBoardHandle, WorkspaceBoardParams, WorkspaceBoardUpdate, WorkspaceDataset,
};

/// Domain wrapper for the Workspace Sync Board showcase.
pub struct WorkspaceBoardApp {
    graph: WorkspaceBoardGraph,
    next_handle: u64,
    handle: Option<WorkspaceBoardHandle>,
    closed: bool,
    output_queue: VecDeque<BoardFrame>,
    sync_effects: VecDeque<SyncEffect>,
    diagnostic_traces: VecDeque<TransactionTrace>,
}

impl WorkspaceBoardApp {
    /// Creates a board app around host-owned data.
    pub fn new(dataset: WorkspaceDataset) -> Self {
        Self {
            graph: build_graph(dataset),
            next_handle: 1,
            handle: None,
            closed: false,
            output_queue: VecDeque::new(),
            sync_effects: VecDeque::new(),
            diagnostic_traces: VecDeque::new(),
        }
    }

    /// Opens a workspace board and returns an opaque handle.
    pub fn open_workspace_board(&mut self, params: WorkspaceBoardParams) -> WorkspaceBoardHandle {
        let handle = self.handle.unwrap_or_else(|| {
            let handle = WorkspaceBoardHandle(self.next_handle);
            self.next_handle += 1;
            self.handle = Some(handle);
            handle
        });
        self.closed = false;

        let mut tx = self.graph.graph.begin_transaction().unwrap();
        tx.set_input(self.graph.params, Some(params)).unwrap();
        let result = tx.commit().unwrap();
        drop(tx);
        self.apply_result(result);
        handle
    }

    /// Applies one domain event to an open board.
    pub fn apply_user_event(
        &mut self,
        handle: WorkspaceBoardHandle,
        event: WorkspaceBoardEvent,
    ) -> WorkspaceBoardUpdate {
        if Some(handle) != self.handle || self.closed {
            return WorkspaceBoardUpdate::default();
        }

        let mut params = self.current_params();
        let mut dataset = self.current_dataset();
        let mut rebaseline = false;

        match event {
            WorkspaceBoardEvent::SwitchView(next) => {
                params = Some(next);
                rebaseline = true;
            }
            WorkspaceBoardEvent::RevokeProjectPermission { project_id } => {
                if let Some(user) = params.as_ref().map(|params| params.user.clone()) {
                    dataset
                        .permissions
                        .entry(user)
                        .or_default()
                        .remove(&project_id);
                }
            }
            WorkspaceBoardEvent::SetVisibleColumns(columns) => {
                if let Some(params) = params.as_mut() {
                    params.visible_columns = columns;
                    rebaseline = true;
                }
            }
            WorkspaceBoardEvent::ReplaceProjectIssues { project_id, issues } => {
                dataset.issues.insert(project_id, issues);
            }
        }

        self.commit_inputs(params, dataset, rebaseline)
    }

    /// Closes the board and tears down scoped resources and output.
    pub fn close(&mut self, handle: WorkspaceBoardHandle) -> WorkspaceBoardUpdate {
        if Some(handle) != self.handle || self.closed {
            return WorkspaceBoardUpdate::default();
        }
        let before_effects = self.sync_effects.len();
        let before_frames = self.output_queue.len();

        let mut tx = self.graph.graph.begin_transaction().unwrap();
        tx.close_scope(self.graph.scope).unwrap();
        let result = tx.commit().unwrap();
        drop(tx);
        self.apply_result(result);
        self.closed = true;

        WorkspaceBoardUpdate {
            emitted_effects: self.sync_effects.len() - before_effects,
            emitted_frames: self.output_queue.len() - before_frames,
        }
    }

    /// Drains typed sync effects for the host executor.
    pub fn drain_sync_effects(&mut self) -> Vec<SyncEffect> {
        self.sync_effects.drain(..).collect()
    }

    /// Drains typed output frames for a board handle.
    pub fn drain_output(&mut self, handle: WorkspaceBoardHandle) -> Vec<BoardFrame> {
        if Some(handle) != self.handle {
            return Vec::new();
        }
        self.output_queue.drain(..).collect()
    }

    /// Drains diagnostic traces for tests and headless inspectors.
    pub fn drain_diagnostic_traces(&mut self) -> Vec<TransactionTrace> {
        self.diagnostic_traces.drain(..).collect()
    }

    fn commit_inputs(
        &mut self,
        params: Option<WorkspaceBoardParams>,
        dataset: WorkspaceDataset,
        rebaseline: bool,
    ) -> WorkspaceBoardUpdate {
        let before_effects = self.sync_effects.len();
        let before_frames = self.output_queue.len();

        let mut tx = self.graph.graph.begin_transaction().unwrap();
        tx.set_input(self.graph.params, params).unwrap();
        tx.set_input(self.graph.dataset, dataset).unwrap();
        if rebaseline {
            tx.rebaseline_output(self.graph.output.clone()).unwrap();
        }
        let result = tx.commit().unwrap();
        drop(tx);
        self.apply_result(result);

        WorkspaceBoardUpdate {
            emitted_effects: self.sync_effects.len() - before_effects,
            emitted_frames: self.output_queue.len() - before_frames,
        }
    }

    fn apply_result(&mut self, result: TransactionResult<SyncCommand>) {
        for command in result.resource_plan.commands() {
            match command {
                ResourceCommand::Open {
                    command: SyncCommand::Open(target),
                    ..
                } => self
                    .sync_effects
                    .push_back(SyncEffect::Open(target.clone())),
                ResourceCommand::Replace {
                    command: SyncCommand::Open(target),
                    ..
                } => self
                    .sync_effects
                    .push_back(SyncEffect::Replace(target.clone())),
                ResourceCommand::Close { key, .. } => {
                    if let Some(target) = target_from_key(key) {
                        self.sync_effects.push_back(SyncEffect::Close(target));
                    }
                }
                ResourceCommand::Refresh { .. } => {}
            }
        }

        for frame in &result.output_frames {
            let frame = match &frame.kind {
                OutputFrameKind::Baseline(snapshot) => {
                    BoardFrame::Baseline(board_snapshot(snapshot))
                }
                OutputFrameKind::Delta(snapshot) => BoardFrame::Delta(board_snapshot(snapshot)),
                OutputFrameKind::Rebaseline(snapshot, _) => {
                    BoardFrame::Rebaseline(board_snapshot(snapshot))
                }
                OutputFrameKind::Clear(_) => BoardFrame::Cleared,
            };
            self.output_queue.push_back(frame);
        }

        let mut trace = result.trace();
        trace
            .invariant_results
            .push(trellis_core::InvariantResultTrace {
                name: "incremental_equals_full_recompute".to_owned(),
                passed: self.graph.graph.full_recompute_check().is_ok(),
            });
        self.diagnostic_traces.push_back(trace);
    }

    fn current_params(&self) -> Option<WorkspaceBoardParams> {
        self.graph
            .graph
            .input_value(self.graph.params)
            .unwrap()
            .cloned()
            .unwrap_or_default()
    }

    fn current_dataset(&self) -> WorkspaceDataset {
        self.graph
            .graph
            .input_value(self.graph.dataset)
            .unwrap()
            .cloned()
            .unwrap_or_default()
    }
}

impl Default for WorkspaceBoardApp {
    fn default() -> Self {
        Self::new(WorkspaceDataset::sample())
    }
}

pub(super) fn org_workspace_params(org: &str, workspace: &str) -> WorkspaceBoardParams {
    WorkspaceBoardParams {
        user: "alex".to_owned(),
        view: BoardView::OrgWorkspace {
            org: org.to_owned(),
            workspace: workspace.to_owned(),
            active_only: true,
        },
        visible_columns: WorkspaceBoardParams::all_columns(),
    }
}

#[cfg(test)]
pub(super) fn personal_params() -> WorkspaceBoardParams {
    WorkspaceBoardParams {
        user: "alex".to_owned(),
        view: BoardView::PersonalAssigned {
            user: "alex".to_owned(),
        },
        visible_columns: WorkspaceBoardParams::all_columns(),
    }
}

#[cfg(test)]
pub(super) fn columns(
    columns: impl IntoIterator<Item = super::types::BoardColumn>,
) -> std::collections::BTreeSet<super::types::BoardColumn> {
    columns.into_iter().collect()
}

fn board_snapshot(snapshot: &trellis_core::OutputPayload) -> BoardSnapshot {
    snapshot
        .get::<BoardSnapshot>()
        .expect("workspace board output payload type")
        .clone()
}
