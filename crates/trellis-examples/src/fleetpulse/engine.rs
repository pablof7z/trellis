use std::collections::VecDeque;

use crate::showcase_trace::ShowcaseHostStatus;
use trellis_core::{ScopeId, TransactionTrace};

use super::graph::{FleetGraph, build_graph, target_key};
use super::sample::params_for_groups;
use super::status::{FleetHostStatus, FleetStatusFrame};
use super::status_runtime::{FleetStatusRuntime, showcase_status};
use super::types::{
    FleetDashboardHandle, FleetDashboardParams, FleetDataset, FleetEffect, FleetFilterChange,
    FleetFrame, FleetPanel, FleetPermissionChange, FleetUpdate,
};

/// Domain wrapper for the FleetPulse telemetry dashboard showcase.
pub struct FleetPulseApp {
    pub(super) graph: FleetGraph,
    next_handle: u64,
    handle: Option<FleetDashboardHandle>,
    closed: bool,
    pub(super) output_queue: VecDeque<FleetFrame>,
    pub(super) effects: VecDeque<FleetEffect>,
    pub(super) diagnostic_traces: VecDeque<TransactionTrace>,
    showcase_statuses: VecDeque<ShowcaseHostStatus>,
    pub(super) status_runtime: FleetStatusRuntime,
}

impl FleetPulseApp {
    /// Creates a FleetPulse app around host-owned data.
    pub fn new(dataset: FleetDataset) -> Self {
        Self {
            graph: build_graph(dataset),
            next_handle: 1,
            handle: None,
            closed: false,
            output_queue: VecDeque::new(),
            effects: VecDeque::new(),
            diagnostic_traces: VecDeque::new(),
            showcase_statuses: VecDeque::new(),
            status_runtime: FleetStatusRuntime::new(),
        }
    }

    /// Opens a fleet dashboard and returns an opaque handle.
    pub fn open_fleet_dashboard(&mut self, params: FleetDashboardParams) -> FleetDashboardHandle {
        let handle = self.handle.unwrap_or_else(|| {
            let handle = FleetDashboardHandle(self.next_handle);
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

    /// Applies a dashboard filter change.
    pub fn apply_filter_change(
        &mut self,
        handle: FleetDashboardHandle,
        change: FleetFilterChange,
    ) -> FleetUpdate {
        if !self.handle_is_open(handle) {
            return FleetUpdate::default();
        }
        let mut params = self.current_params();
        if let Some(params) = params.as_mut() {
            params.customer = change.customer;
            params.site = change.site;
            params.groups = change.groups;
        }
        self.commit_inputs(
            params,
            self.current_dataset(),
            self.current_host_status(),
            true,
        )
    }

    /// Applies a permission change for the active user.
    pub fn apply_permission_change(
        &mut self,
        handle: FleetDashboardHandle,
        change: FleetPermissionChange,
    ) -> FleetUpdate {
        if !self.handle_is_open(handle) {
            return FleetUpdate::default();
        }
        let params = self.current_params();
        let mut dataset = self.current_dataset();
        if let Some(user) = params.as_ref().map(|params| params.user.clone()) {
            match change {
                FleetPermissionChange::RevokeDevice { device_id } => {
                    dataset
                        .permissions
                        .entry(user)
                        .or_default()
                        .devices
                        .remove(&device_id);
                }
                FleetPermissionChange::ReplacePermissions(next) => {
                    dataset.permissions.insert(user, next);
                }
            }
        }
        self.commit_inputs(params, dataset, self.current_host_status(), false)
    }

    /// Applies one host status as a canonical domain input.
    pub fn apply_host_status(
        &mut self,
        handle: FleetDashboardHandle,
        status: FleetHostStatus,
    ) -> FleetUpdate {
        if !self.handle_is_open(handle) {
            return FleetUpdate::default();
        }
        let key = target_key(&status.target);
        let scope = self.panel_scope(status.panel.clone());
        let scope_owns_resource = self
            .graph
            .graph
            .resource_owners(&key)
            .is_some_and(|owners| owners.contains(&scope));
        let frame = self
            .status_runtime
            .classify_status(status, key, scope, scope_owns_resource);
        self.showcase_statuses.push_back(showcase_status(&frame));
        self.commit_inputs(
            self.current_params(),
            self.current_dataset(),
            Some(frame),
            false,
        )
    }

    /// Closes the dashboard and tears down scoped resources and output.
    pub fn close(&mut self, handle: FleetDashboardHandle) -> FleetUpdate {
        if !self.handle_is_open(handle) {
            return FleetUpdate::default();
        }
        let before_effects = self.effects.len();
        let before_frames = self.output_queue.len();

        let mut tx = self.graph.graph.begin_transaction().unwrap();
        tx.close_scope(self.graph.dashboard_scope).unwrap();
        let result = tx.commit().unwrap();
        drop(tx);
        self.apply_result(result);
        self.closed = true;

        FleetUpdate {
            emitted_effects: self.effects.len() - before_effects,
            emitted_frames: self.output_queue.len() - before_frames,
        }
    }

    /// Drains typed effects for the host executor.
    pub fn drain_effects(&mut self) -> Vec<FleetEffect> {
        self.effects.drain(..).collect()
    }

    /// Drains typed output frames for a dashboard handle.
    pub fn drain_output(&mut self, handle: FleetDashboardHandle) -> Vec<FleetFrame> {
        if Some(handle) != self.handle {
            return Vec::new();
        }
        self.output_queue.drain(..).collect()
    }

    /// Drains diagnostic traces for tests and headless inspectors.
    pub fn drain_diagnostic_traces(&mut self) -> Vec<TransactionTrace> {
        self.diagnostic_traces.drain(..).collect()
    }

    /// Returns the latest command revision for a target.
    pub fn command_revision_for(&self, target: &super::types::FleetTarget) -> Option<u64> {
        self.status_runtime
            .command_revision_for(&target_key(target))
    }

    pub(super) fn drain_showcase_host_statuses(&mut self) -> Vec<ShowcaseHostStatus> {
        self.showcase_statuses.drain(..).collect()
    }

    pub(super) fn close_panel(
        &mut self,
        handle: FleetDashboardHandle,
        panel: FleetPanel,
    ) -> FleetUpdate {
        if !self.handle_is_open(handle) {
            return FleetUpdate::default();
        }
        let before_effects = self.effects.len();
        let before_frames = self.output_queue.len();
        let scope = self.panel_scope(panel);
        let mut tx = self.graph.graph.begin_transaction().unwrap();
        tx.close_scope(scope).unwrap();
        let result = tx.commit().unwrap();
        drop(tx);
        self.apply_result(result);
        FleetUpdate {
            emitted_effects: self.effects.len() - before_effects,
            emitted_frames: self.output_queue.len() - before_frames,
        }
    }

    fn commit_inputs(
        &mut self,
        params: Option<FleetDashboardParams>,
        dataset: FleetDataset,
        host_status: Option<FleetStatusFrame>,
        rebaseline: bool,
    ) -> FleetUpdate {
        let before_effects = self.effects.len();
        let before_frames = self.output_queue.len();

        let mut tx = self.graph.graph.begin_transaction().unwrap();
        tx.set_input(self.graph.params, params).unwrap();
        tx.set_input(self.graph.dataset, dataset).unwrap();
        tx.set_input(self.graph.host_status, host_status).unwrap();
        if rebaseline {
            tx.rebaseline_output(self.graph.output.clone()).unwrap();
        }
        let result = tx.commit().unwrap();
        drop(tx);
        self.apply_result(result);

        FleetUpdate {
            emitted_effects: self.effects.len() - before_effects,
            emitted_frames: self.output_queue.len() - before_frames,
        }
    }

    fn handle_is_open(&self, handle: FleetDashboardHandle) -> bool {
        Some(handle) == self.handle && !self.closed
    }

    fn current_params(&self) -> Option<FleetDashboardParams> {
        self.graph
            .graph
            .input_value(self.graph.params)
            .unwrap()
            .cloned()
            .unwrap_or_default()
    }

    fn current_dataset(&self) -> FleetDataset {
        self.graph
            .graph
            .input_value(self.graph.dataset)
            .unwrap()
            .cloned()
            .unwrap_or_default()
    }

    fn current_host_status(&self) -> Option<FleetStatusFrame> {
        self.graph
            .graph
            .input_value(self.graph.host_status)
            .unwrap()
            .cloned()
            .unwrap_or_default()
    }

    fn panel_scope(&self, panel: FleetPanel) -> ScopeId {
        match panel {
            FleetPanel::Overview => self.graph.overview_scope,
            FleetPanel::Alerts => self.graph.alerts_scope,
        }
    }
}

impl Default for FleetPulseApp {
    fn default() -> Self {
        Self::new(FleetDataset::sample())
    }
}

pub(super) fn default_params() -> FleetDashboardParams {
    params_for_groups(["pumps"])
}
