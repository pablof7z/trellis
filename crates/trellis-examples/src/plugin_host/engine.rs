use std::collections::{BTreeMap, BTreeSet, VecDeque};

use trellis_core::{
    InvariantResultTrace, OutputFrameKind, OutputKey, ResourceCommand, TransactionResult,
    TransactionTrace,
};

use super::graph::{PluginHostGraph, PluginSlot, build_graph, resource_from_key};
use super::types::{
    PluginCommand, PluginEffect, PluginFrame, PluginHandle, PluginHostEvent, PluginHostUpdate,
    PluginManifest, PluginPermission, PluginShellSnapshot, WorkspaceKind,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct ActivePlugin {
    slot: PluginSlot,
    open: bool,
}

/// Domain wrapper for the PluginHost capability lifecycle showcase.
pub struct PluginHostApp {
    graph: PluginHostGraph,
    next_handle: u64,
    handles: BTreeMap<PluginHandle, ActivePlugin>,
    output_queue: BTreeMap<PluginHandle, VecDeque<PluginFrame>>,
    effects: VecDeque<PluginEffect>,
    diagnostic_traces: VecDeque<TransactionTrace>,
}

impl PluginHostApp {
    /// Creates a plugin host app with workspace and permission inputs.
    pub fn new(workspace_kind: WorkspaceKind, permissions: BTreeSet<PluginPermission>) -> Self {
        Self {
            graph: build_graph(workspace_kind, permissions),
            next_handle: 1,
            handles: BTreeMap::new(),
            output_queue: BTreeMap::new(),
            effects: VecDeque::new(),
            diagnostic_traces: VecDeque::new(),
        }
    }

    /// Enables one plugin manifest and returns an opaque handle.
    pub fn enable_plugin(&mut self, manifest: PluginManifest) -> PluginHandle {
        let slot = self
            .next_slot()
            .expect("PluginHost showcase supports two plugins");
        let handle = PluginHandle(self.next_handle);
        self.next_handle += 1;
        self.handles
            .insert(handle, ActivePlugin { slot, open: true });
        self.output_queue.entry(handle).or_default();

        let plugin_input = self.graph.plugin(slot).manifest;
        let mut tx = self.graph.graph.begin_transaction().unwrap();
        tx.set_input(plugin_input, Some(manifest)).unwrap();
        let result = tx.commit().unwrap();
        drop(tx);
        self.apply_result(result);
        handle
    }

    /// Applies a plugin-host domain event.
    pub fn apply_plugin_event(
        &mut self,
        handle: PluginHandle,
        event: PluginHostEvent,
    ) -> PluginHostUpdate {
        let Some(active) = self
            .handles
            .get(&handle)
            .copied()
            .filter(|active| active.open)
        else {
            return PluginHostUpdate::default();
        };

        let before_effects = self.effects.len();
        let before_frames = self.output_frame_count();
        let manifest_input = self.graph.plugin(active.slot).manifest;
        let workspace_input = self.graph.workspace;
        let permissions_input = self.graph.permissions;
        let revoked_permissions = match event {
            PluginHostEvent::RevokePermission(permission) => {
                let mut permissions = self.current_permissions();
                permissions.remove(&permission);
                Some(permissions)
            }
            _ => None,
        };
        let mut tx = self.graph.graph.begin_transaction().unwrap();
        match event {
            PluginHostEvent::ReplaceManifest(manifest) => {
                tx.set_input(manifest_input, Some(manifest)).unwrap();
            }
            PluginHostEvent::SetWorkspaceKind(workspace) => {
                tx.set_input(workspace_input, workspace).unwrap();
            }
            PluginHostEvent::ReplacePermissions(permissions) => {
                tx.set_input(permissions_input, permissions).unwrap();
            }
            PluginHostEvent::RevokePermission(_) => {
                tx.set_input(permissions_input, revoked_permissions.unwrap())
                    .unwrap();
            }
        }
        let result = tx.commit().unwrap();
        drop(tx);
        self.apply_result(result);
        PluginHostUpdate {
            emitted_effects: self.effects.len() - before_effects,
            emitted_frames: self.output_frame_count() - before_frames,
        }
    }

    /// Disables a plugin and closes its scoped capabilities and output.
    pub fn disable_plugin(&mut self, handle: PluginHandle) -> PluginHostUpdate {
        let Some(active) = self
            .handles
            .get(&handle)
            .copied()
            .filter(|active| active.open)
        else {
            return PluginHostUpdate::default();
        };
        let before_effects = self.effects.len();
        let before_frames = self.output_frame_count();
        let scope = self.graph.plugin(active.slot).scope;
        let mut tx = self.graph.graph.begin_transaction().unwrap();
        tx.close_scope(scope).unwrap();
        let result = tx.commit().unwrap();
        drop(tx);
        self.apply_result(result);
        if let Some(active) = self.handles.get_mut(&handle) {
            active.open = false;
        }
        PluginHostUpdate {
            emitted_effects: self.effects.len() - before_effects,
            emitted_frames: self.output_frame_count() - before_frames,
        }
    }

    /// Drains queued host lifecycle effects.
    pub fn drain_effects(&mut self) -> Vec<PluginEffect> {
        self.effects.drain(..).collect()
    }

    /// Drains queued shell frames for a plugin handle.
    pub fn drain_output(&mut self, handle: PluginHandle) -> Vec<PluginFrame> {
        self.output_queue
            .entry(handle)
            .or_default()
            .drain(..)
            .collect()
    }

    /// Drains diagnostic transaction traces.
    pub fn drain_diagnostic_traces(&mut self) -> Vec<TransactionTrace> {
        self.diagnostic_traces.drain(..).collect()
    }

    fn next_slot(&self) -> Option<PluginSlot> {
        [PluginSlot::Primary, PluginSlot::Secondary]
            .into_iter()
            .find(|slot| !self.handles.values().any(|active| active.slot == *slot))
    }

    fn current_permissions(&self) -> BTreeSet<PluginPermission> {
        self.graph
            .graph
            .input_value(self.graph.permissions)
            .unwrap()
            .cloned()
            .unwrap_or_default()
    }

    fn apply_result(&mut self, result: TransactionResult<PluginCommand>) {
        for command in result.resource_plan.commands() {
            match command {
                ResourceCommand::Open {
                    command: PluginCommand::Open(resource),
                    ..
                } => self.effects.push_back(PluginEffect::Open(resource.clone())),
                ResourceCommand::Replace {
                    command: PluginCommand::Open(resource),
                    ..
                }
                | ResourceCommand::Refresh {
                    command: PluginCommand::Open(resource),
                    ..
                } => self.effects.push_back(PluginEffect::Open(resource.clone())),
                ResourceCommand::Close { key, .. } => {
                    if let Some(resource) = resource_from_key(key) {
                        self.effects.push_back(PluginEffect::Close(resource));
                    }
                }
            }
        }

        for frame in &result.output_frames {
            let Some(handle) = self.handle_for_output(frame.output_key) else {
                continue;
            };
            let frame = plugin_frame(&frame.kind);
            self.output_queue
                .entry(handle)
                .or_default()
                .push_back(frame);
        }

        let mut trace = result.trace();
        trace.invariant_results.push(InvariantResultTrace {
            name: "incremental_equals_full_recompute".to_owned(),
            passed: self.graph.graph.full_recompute_check().is_ok(),
        });
        self.diagnostic_traces.push_back(trace);
    }

    fn handle_for_output(&self, key: OutputKey) -> Option<PluginHandle> {
        self.handles.iter().find_map(|(handle, active)| {
            (self.graph.plugin(active.slot).output.key() == key).then_some(*handle)
        })
    }

    fn output_frame_count(&self) -> usize {
        self.output_queue.values().map(VecDeque::len).sum()
    }
}

impl Default for PluginHostApp {
    fn default() -> Self {
        Self::new(WorkspaceKind::Rust, BTreeSet::new())
    }
}

fn plugin_frame(kind: &OutputFrameKind) -> PluginFrame {
    match kind {
        OutputFrameKind::Baseline(value) => PluginFrame::Baseline(
            value
                .get::<PluginShellSnapshot>()
                .cloned()
                .unwrap_or_default(),
        ),
        OutputFrameKind::Delta(value) => PluginFrame::Delta(
            value
                .get::<PluginShellSnapshot>()
                .cloned()
                .unwrap_or_default(),
        ),
        OutputFrameKind::Rebaseline(value, _) => PluginFrame::Rebaseline(
            value
                .get::<PluginShellSnapshot>()
                .cloned()
                .unwrap_or_default(),
        ),
        OutputFrameKind::Clear(_) => PluginFrame::Cleared,
    }
}
