use std::collections::VecDeque;

use trellis_core::{
    InvariantResultTrace, OutputFrameKind, ResourceCommand, TransactionResult, TransactionTrace,
};

use super::graph::{MarketGraph, build_graph, resource_from_key};
use super::types::{
    MarketCommand, MarketDataset, MarketDeskEvent, MarketDeskUpdate, MarketEffect, MarketFrame,
    MarketSnapshot, MarketTerminalHandle, MarketWorkspace,
};

/// Domain wrapper for the MarketDesk live market-data showcase.
pub struct MarketDeskApp {
    graph: MarketGraph,
    next_handle: u64,
    handle: Option<MarketTerminalHandle>,
    closed: bool,
    effects: VecDeque<MarketEffect>,
    output_queue: VecDeque<MarketFrame>,
    diagnostic_traces: VecDeque<TransactionTrace>,
}

impl MarketDeskApp {
    /// Creates a MarketDesk app around host-owned data.
    pub fn new(dataset: MarketDataset) -> Self {
        Self {
            graph: build_graph(dataset),
            next_handle: 1,
            handle: None,
            closed: true,
            effects: VecDeque::new(),
            output_queue: VecDeque::new(),
            diagnostic_traces: VecDeque::new(),
        }
    }

    /// Opens one market terminal workspace and returns an opaque handle.
    pub fn open_terminal(&mut self, workspace: MarketWorkspace) -> MarketTerminalHandle {
        let handle = self.handle.unwrap_or_else(|| {
            let handle = MarketTerminalHandle(self.next_handle);
            self.next_handle += 1;
            self.handle = Some(handle);
            handle
        });
        self.closed = false;
        let _ = self.commit_inputs(Some(workspace), self.current_dataset());
        handle
    }

    /// Applies one domain event to an open terminal.
    pub fn apply_event(
        &mut self,
        handle: MarketTerminalHandle,
        event: MarketDeskEvent,
    ) -> MarketDeskUpdate {
        if !self.handle_is_open(handle) {
            return MarketDeskUpdate::default();
        }
        let mut workspace = self.current_workspace();
        let mut dataset = self.current_dataset();
        let active_user = workspace.as_ref().map(|workspace| workspace.user.clone());

        match event {
            MarketDeskEvent::ReplaceWatchlist(symbols) => {
                if let Some(workspace) = workspace.as_mut() {
                    workspace.watchlist = symbols;
                }
            }
            MarketDeskEvent::OpenChart(symbol) => {
                if let Some(workspace) = workspace.as_mut() {
                    workspace.open_charts.insert(symbol);
                }
            }
            MarketDeskEvent::CloseChart(symbol) => {
                if let Some(workspace) = workspace.as_mut() {
                    workspace.open_charts.remove(&symbol);
                }
            }
            MarketDeskEvent::RevokeEntitlement { symbol } => {
                if let Some(user) = active_user {
                    dataset
                        .entitlements
                        .entry(user)
                        .or_default()
                        .remove(&symbol);
                }
            }
            MarketDeskEvent::ReplaceEntitlements(symbols) => {
                if let Some(user) = active_user {
                    dataset.entitlements.insert(user, symbols);
                }
            }
        }
        self.commit_inputs(workspace, dataset)
    }

    /// Closes the workspace and clears all scoped streams and output.
    pub fn close(&mut self, handle: MarketTerminalHandle) -> MarketDeskUpdate {
        if !self.handle_is_open(handle) {
            return MarketDeskUpdate::default();
        }
        let before_effects = self.effects.len();
        let before_frames = self.output_queue.len();
        let mut tx = self.graph.graph.begin_transaction().unwrap();
        tx.close_scope(self.graph.workspace_scope).unwrap();
        let result = tx.commit().unwrap();
        drop(tx);
        self.apply_result(result);
        self.closed = true;
        MarketDeskUpdate {
            emitted_effects: self.effects.len() - before_effects,
            emitted_frames: self.output_queue.len() - before_frames,
        }
    }

    /// Drains queued host lifecycle effects.
    pub fn drain_effects(&mut self) -> Vec<MarketEffect> {
        self.effects.drain(..).collect()
    }

    /// Drains queued terminal frames for a handle.
    pub fn drain_output(&mut self, handle: MarketTerminalHandle) -> Vec<MarketFrame> {
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
        workspace: Option<MarketWorkspace>,
        dataset: MarketDataset,
    ) -> MarketDeskUpdate {
        let before_effects = self.effects.len();
        let before_frames = self.output_queue.len();
        let mut tx = self.graph.graph.begin_transaction().unwrap();
        tx.set_input(self.graph.workspace, workspace).unwrap();
        tx.set_input(self.graph.dataset, dataset).unwrap();
        let result = tx.commit().unwrap();
        drop(tx);
        self.apply_result(result);
        MarketDeskUpdate {
            emitted_effects: self.effects.len() - before_effects,
            emitted_frames: self.output_queue.len() - before_frames,
        }
    }

    fn handle_is_open(&self, handle: MarketTerminalHandle) -> bool {
        Some(handle) == self.handle && !self.closed
    }

    fn current_workspace(&self) -> Option<MarketWorkspace> {
        self.graph
            .graph
            .input_value(self.graph.workspace)
            .unwrap()
            .cloned()
            .unwrap_or_default()
    }

    fn current_dataset(&self) -> MarketDataset {
        self.graph
            .graph
            .input_value(self.graph.dataset)
            .unwrap()
            .cloned()
            .unwrap_or_default()
    }

    fn apply_result(&mut self, result: TransactionResult<MarketCommand>) {
        for command in result.resource_plan.commands() {
            match command {
                ResourceCommand::Open {
                    command: MarketCommand::Open(resource),
                    ..
                } => self.effects.push_back(MarketEffect::Open(resource.clone())),
                ResourceCommand::Replace {
                    command: MarketCommand::Open(resource),
                    ..
                }
                | ResourceCommand::Refresh {
                    command: MarketCommand::Open(resource),
                    ..
                } => self.effects.push_back(MarketEffect::Open(resource.clone())),
                ResourceCommand::Close { key, .. } => {
                    if let Some(resource) = resource_from_key(key) {
                        self.effects.push_back(MarketEffect::Close(resource));
                    }
                }
            }
        }

        for frame in &result.output_frames {
            self.output_queue.push_back(market_frame(&frame.kind));
        }

        let mut trace = result.trace();
        trace.invariant_results.push(InvariantResultTrace {
            name: "incremental_equals_full_recompute".to_owned(),
            passed: self.graph.graph.full_recompute_check().is_ok(),
        });
        self.diagnostic_traces.push_back(trace);
    }
}

impl Default for MarketDeskApp {
    fn default() -> Self {
        Self::new(MarketDataset::default())
    }
}

fn market_frame(kind: &OutputFrameKind) -> MarketFrame {
    match kind {
        OutputFrameKind::Baseline(value) => {
            MarketFrame::Baseline(value.get::<MarketSnapshot>().cloned().unwrap_or_default())
        }
        OutputFrameKind::Delta(value) => {
            MarketFrame::Delta(value.get::<MarketSnapshot>().cloned().unwrap_or_default())
        }
        OutputFrameKind::Rebaseline(value, _) => {
            MarketFrame::Rebaseline(value.get::<MarketSnapshot>().cloned().unwrap_or_default())
        }
        OutputFrameKind::Clear(_) => MarketFrame::Cleared,
    }
}
