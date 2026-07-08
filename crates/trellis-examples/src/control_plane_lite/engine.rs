use std::collections::{BTreeMap, VecDeque};

use trellis_core::{InvariantResultTrace, ResourceCommand, TransactionResult, TransactionTrace};

use super::frames::control_frame;
use super::graph::{ControlGraph, build_graph, resource_from_key};
use super::types::{
    ControlCommand, ControlEffect, ControlFrame, ControlPlaneEvent, ControlPlaneHandle,
    ControlPlaneUpdate, ControlResource, ControlResourceStatus, DesiredAppConfig,
};

/// Domain wrapper for the ControlPlane Lite reconciler showcase.
pub struct ControlPlaneLiteApp {
    graph: ControlGraph,
    next_handle: u64,
    handle: Option<ControlPlaneHandle>,
    closed: bool,
    effects: VecDeque<ControlEffect>,
    output_queue: VecDeque<ControlFrame>,
    diagnostic_traces: VecDeque<TransactionTrace>,
}

impl ControlPlaneLiteApp {
    /// Creates an empty ControlPlane Lite app.
    pub fn new() -> Self {
        Self {
            graph: build_graph(),
            next_handle: 1,
            handle: None,
            closed: true,
            effects: VecDeque::new(),
            output_queue: VecDeque::new(),
            diagnostic_traces: VecDeque::new(),
        }
    }

    /// Opens one controller scope and returns an opaque handle.
    pub fn open_controller(&mut self, config: DesiredAppConfig) -> ControlPlaneHandle {
        let handle = self.handle.unwrap_or_else(|| {
            let handle = ControlPlaneHandle(self.next_handle);
            self.next_handle += 1;
            self.handle = Some(handle);
            handle
        });
        self.closed = false;
        let _ = self.commit_inputs(Some(config), self.current_statuses(), false);
        handle
    }

    /// Applies one desired-state or host-status event.
    pub fn apply_event(
        &mut self,
        handle: ControlPlaneHandle,
        event: ControlPlaneEvent,
    ) -> ControlPlaneUpdate {
        if !self.handle_is_open(handle) {
            return ControlPlaneUpdate::default();
        }
        let mut config = self.current_config();
        let mut statuses = self.current_statuses();
        let mut rebaseline = false;
        match event {
            ControlPlaneEvent::ReplaceConfig(next) => {
                config = Some(next);
                rebaseline = true;
            }
            ControlPlaneEvent::ApplyResourceStatus { resource, status } => {
                statuses.insert(resource, status);
            }
            ControlPlaneEvent::ClearResourceStatus { resource } => {
                statuses.remove(&resource);
            }
            ControlPlaneEvent::ReplaceStatuses(next) => {
                statuses = next;
            }
        }
        self.commit_inputs(config, statuses, rebaseline)
    }

    /// Closes the controller and clears resources and output.
    pub fn close(&mut self, handle: ControlPlaneHandle) -> ControlPlaneUpdate {
        if !self.handle_is_open(handle) {
            return ControlPlaneUpdate::default();
        }
        let before_effects = self.effects.len();
        let before_frames = self.output_queue.len();
        let mut tx = self.graph.graph.begin_transaction().unwrap();
        tx.close_scope(self.graph.controller_scope).unwrap();
        let result = tx.commit().unwrap();
        drop(tx);
        self.apply_result(result);
        self.closed = true;
        ControlPlaneUpdate {
            emitted_effects: self.effects.len() - before_effects,
            emitted_frames: self.output_queue.len() - before_frames,
        }
    }

    /// Drains queued host lifecycle effects.
    pub fn drain_effects(&mut self) -> Vec<ControlEffect> {
        self.effects.drain(..).collect()
    }

    /// Drains queued status frames for a handle.
    pub fn drain_output(&mut self, handle: ControlPlaneHandle) -> Vec<ControlFrame> {
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
        config: Option<DesiredAppConfig>,
        statuses: BTreeMap<ControlResource, ControlResourceStatus>,
        rebaseline: bool,
    ) -> ControlPlaneUpdate {
        let before_effects = self.effects.len();
        let before_frames = self.output_queue.len();
        let mut tx = self.graph.graph.begin_transaction().unwrap();
        tx.set_input(self.graph.config, config).unwrap();
        tx.set_input(self.graph.statuses, statuses).unwrap();
        if rebaseline {
            tx.rebaseline_output(self.graph.output.clone()).unwrap();
        }
        let result = tx.commit().unwrap();
        drop(tx);
        self.apply_result(result);
        ControlPlaneUpdate {
            emitted_effects: self.effects.len() - before_effects,
            emitted_frames: self.output_queue.len() - before_frames,
        }
    }

    fn handle_is_open(&self, handle: ControlPlaneHandle) -> bool {
        Some(handle) == self.handle && !self.closed
    }

    fn current_config(&self) -> Option<DesiredAppConfig> {
        self.graph
            .graph
            .input_value(self.graph.config)
            .unwrap()
            .cloned()
            .unwrap_or_default()
    }

    fn current_statuses(&self) -> BTreeMap<ControlResource, ControlResourceStatus> {
        self.graph
            .graph
            .input_value(self.graph.statuses)
            .unwrap()
            .cloned()
            .unwrap_or_default()
    }

    fn apply_result(&mut self, result: TransactionResult<ControlCommand>) {
        for command in result.resource_plan.commands() {
            match command {
                ResourceCommand::Open {
                    command: ControlCommand::Open(resource),
                    ..
                } => self
                    .effects
                    .push_back(ControlEffect::Open(resource.clone())),
                ResourceCommand::Replace {
                    command: ControlCommand::Open(resource),
                    ..
                }
                | ResourceCommand::Refresh {
                    command: ControlCommand::Open(resource),
                    ..
                } => self
                    .effects
                    .push_back(ControlEffect::Open(resource.clone())),
                ResourceCommand::Close { key, .. } => {
                    if let Some(resource) = resource_from_key(key) {
                        self.effects.push_back(ControlEffect::Close(resource));
                    }
                }
            }
        }

        for frame in &result.output_frames {
            self.output_queue.push_back(control_frame(&frame.kind));
        }

        let mut trace = result.trace();
        trace.invariant_results.push(InvariantResultTrace {
            name: "incremental_equals_full_recompute".to_owned(),
            passed: self.graph.graph.full_recompute_check().is_ok(),
        });
        self.diagnostic_traces.push_back(trace);
    }
}

impl Default for ControlPlaneLiteApp {
    fn default() -> Self {
        Self::new()
    }
}
