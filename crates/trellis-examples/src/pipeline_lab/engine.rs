use std::collections::{BTreeMap, VecDeque};

use trellis_core::{InvariantResultTrace, ResourceCommand, TransactionResult, TransactionTrace};

use super::frames::pipeline_frame;
use super::graph::{PipelineGraph, build_graph, resource_from_key};
use super::selectors::downstream_closure;
use super::types::{
    CredentialStore, PipelineCommand, PipelineEffect, PipelineFrame, PipelineGraphSpec,
    PipelineHandle, PipelineJobStatus, PipelineLabEvent, PipelineLabUpdate, PipelineNodeKind,
    PipelineSession,
};

/// Domain wrapper for the PipelineLab visual pipeline showcase.
pub struct PipelineLabApp {
    graph: PipelineGraph,
    next_handle: u64,
    handle: Option<PipelineHandle>,
    closed: bool,
    effects: VecDeque<PipelineEffect>,
    output_queue: VecDeque<PipelineFrame>,
    diagnostic_traces: VecDeque<TransactionTrace>,
}

impl PipelineLabApp {
    /// Creates a PipelineLab app around host-owned graph and credential data.
    pub fn new(pipeline: PipelineGraphSpec, credentials: CredentialStore) -> Self {
        Self {
            graph: build_graph(pipeline, credentials),
            next_handle: 1,
            handle: None,
            closed: true,
            effects: VecDeque::new(),
            output_queue: VecDeque::new(),
            diagnostic_traces: VecDeque::new(),
        }
    }

    /// Opens one pipeline workspace and returns an opaque handle.
    pub fn open_pipeline(&mut self, session: PipelineSession) -> PipelineHandle {
        let handle = self.handle.unwrap_or_else(|| {
            let handle = PipelineHandle(self.next_handle);
            self.next_handle += 1;
            self.handle = Some(handle);
            handle
        });
        self.closed = false;
        let _ = self.commit_inputs(
            Some(session),
            self.current_pipeline(),
            self.current_credentials(),
            self.current_statuses(),
            false,
        );
        handle
    }

    /// Applies one domain event to an open pipeline workspace.
    pub fn apply_event(
        &mut self,
        handle: PipelineHandle,
        event: PipelineLabEvent,
    ) -> PipelineLabUpdate {
        if !self.handle_is_open(handle) {
            return PipelineLabUpdate::default();
        }
        let mut session = self.current_session();
        let mut pipeline = self.current_pipeline();
        let mut credentials = self.current_credentials();
        let mut statuses = self.current_statuses();
        let mut rebaseline = false;
        match event {
            PipelineLabEvent::SelectNodes(nodes) => {
                if let Some(session) = session.as_mut() {
                    session.selected_nodes = nodes;
                    session
                        .hidden_nodes
                        .retain(|node_id| session.selected_nodes.contains(node_id));
                    rebaseline = true;
                }
            }
            PipelineLabEvent::HideNode(node_id) => {
                if let Some(session) = session.as_mut() {
                    session.hidden_nodes.insert(node_id);
                    rebaseline = true;
                }
            }
            PipelineLabEvent::ShowNode(node_id) => {
                if let Some(session) = session.as_mut() {
                    session.hidden_nodes.remove(&node_id);
                    rebaseline = true;
                }
            }
            PipelineLabEvent::EditTransform {
                node_id,
                expression,
            } => {
                let changed = pipeline.nodes.get_mut(&node_id).is_some_and(|node| {
                    if let PipelineNodeKind::Transform {
                        expression: current,
                        revision,
                    } = &mut node.kind
                    {
                        *current = expression;
                        *revision += 1;
                        true
                    } else {
                        false
                    }
                });
                if changed {
                    let roots = [node_id].into_iter().collect();
                    for affected in downstream_closure(&pipeline, &roots) {
                        statuses.remove(&affected);
                    }
                    rebaseline = true;
                }
            }
            PipelineLabEvent::RevokeSourceCredential { source_id } => {
                if let Some(session) = session.as_ref()
                    && let Some(allowed) = credentials.sources_by_user.get_mut(&session.user)
                {
                    allowed.remove(&source_id);
                }
            }
            PipelineLabEvent::ApplyJobStatus { node_id, status } => {
                statuses.insert(node_id, status);
            }
            PipelineLabEvent::ReplaceGraph(next) => {
                pipeline = next;
                statuses.clear();
                rebaseline = true;
            }
            PipelineLabEvent::ReplaceCredentials(next) => {
                credentials = next;
            }
        }
        self.commit_inputs(session, pipeline, credentials, statuses, rebaseline)
    }

    /// Closes the pipeline workspace and clears resources and output.
    pub fn close(&mut self, handle: PipelineHandle) -> PipelineLabUpdate {
        if !self.handle_is_open(handle) {
            return PipelineLabUpdate::default();
        }
        let before_effects = self.effects.len();
        let before_frames = self.output_queue.len();
        let mut tx = self.graph.graph.begin_transaction().unwrap();
        tx.close_scope(self.graph.workspace_scope).unwrap();
        let result = tx.commit().unwrap();
        drop(tx);
        self.apply_result(result);
        self.closed = true;
        PipelineLabUpdate {
            emitted_effects: self.effects.len() - before_effects,
            emitted_frames: self.output_queue.len() - before_frames,
        }
    }

    /// Drains queued host lifecycle effects.
    pub fn drain_effects(&mut self) -> Vec<PipelineEffect> {
        self.effects.drain(..).collect()
    }

    /// Drains queued pipeline preview frames for a handle.
    pub fn drain_output(&mut self, handle: PipelineHandle) -> Vec<PipelineFrame> {
        if Some(handle) != self.handle {
            return Vec::new();
        }
        self.output_queue.drain(..).collect()
    }

    /// Drains diagnostic transaction traces.
    pub fn drain_diagnostic_traces(&mut self) -> Vec<TransactionTrace> {
        self.diagnostic_traces.drain(..).collect()
    }

    fn commit_inputs(
        &mut self,
        session: Option<PipelineSession>,
        pipeline: PipelineGraphSpec,
        credentials: CredentialStore,
        statuses: BTreeMap<String, PipelineJobStatus>,
        rebaseline: bool,
    ) -> PipelineLabUpdate {
        let before_effects = self.effects.len();
        let before_frames = self.output_queue.len();
        let mut tx = self.graph.graph.begin_transaction().unwrap();
        tx.set_input(self.graph.session, session).unwrap();
        tx.set_input(self.graph.pipeline, pipeline).unwrap();
        tx.set_input(self.graph.credentials, credentials).unwrap();
        tx.set_input(self.graph.statuses, statuses).unwrap();
        if rebaseline {
            tx.rebaseline_output(self.graph.output.clone()).unwrap();
        }
        let result = tx.commit().unwrap();
        drop(tx);
        self.apply_result(result);
        PipelineLabUpdate {
            emitted_effects: self.effects.len() - before_effects,
            emitted_frames: self.output_queue.len() - before_frames,
        }
    }

    fn handle_is_open(&self, handle: PipelineHandle) -> bool {
        Some(handle) == self.handle && !self.closed
    }

    fn current_session(&self) -> Option<PipelineSession> {
        self.graph
            .graph
            .input_value(self.graph.session)
            .unwrap()
            .cloned()
            .unwrap_or_default()
    }

    fn current_pipeline(&self) -> PipelineGraphSpec {
        self.graph
            .graph
            .input_value(self.graph.pipeline)
            .unwrap()
            .cloned()
            .unwrap_or_default()
    }

    fn current_credentials(&self) -> CredentialStore {
        self.graph
            .graph
            .input_value(self.graph.credentials)
            .unwrap()
            .cloned()
            .unwrap_or_default()
    }

    fn current_statuses(&self) -> BTreeMap<String, PipelineJobStatus> {
        self.graph
            .graph
            .input_value(self.graph.statuses)
            .unwrap()
            .cloned()
            .unwrap_or_default()
    }

    fn apply_result(&mut self, result: TransactionResult<PipelineCommand>) {
        for command in result.resource_plan.commands() {
            match command {
                ResourceCommand::Open {
                    command: PipelineCommand::Open(resource),
                    ..
                } => self
                    .effects
                    .push_back(PipelineEffect::Open(resource.clone())),
                ResourceCommand::Replace {
                    command: PipelineCommand::Open(resource),
                    ..
                }
                | ResourceCommand::Refresh {
                    command: PipelineCommand::Open(resource),
                    ..
                } => self
                    .effects
                    .push_back(PipelineEffect::Open(resource.clone())),
                ResourceCommand::Close { key, .. } => {
                    if let Some(resource) = resource_from_key(key) {
                        self.effects.push_back(PipelineEffect::Close(resource));
                    }
                }
            }
        }

        for frame in &result.output_frames {
            self.output_queue.push_back(pipeline_frame(&frame.kind));
        }

        let mut trace = result.trace();
        trace.invariant_results.push(InvariantResultTrace {
            name: "incremental_equals_full_recompute".to_owned(),
            passed: self.graph.graph.full_recompute_check().is_ok(),
        });
        self.diagnostic_traces.push_back(trace);
    }
}

impl Default for PipelineLabApp {
    fn default() -> Self {
        Self::new(PipelineGraphSpec::default(), CredentialStore::default())
    }
}
