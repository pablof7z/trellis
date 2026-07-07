use crate::{AuditEvent, Graph, NodeId, OutputKey, ResourceKey, ScopeId, TransactionTrace};

/// Deterministic labels for graph ids that appear in traces and diagnostics.
///
/// Labels are diagnostic data, not identity. Readers must continue to use the
/// graph-local ids and resource keys as the structural identity of a trace.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct GraphLabelRegistry {
    nodes: Vec<NodeLabel>,
    scopes: Vec<ScopeLabel>,
    resources: Vec<ResourceLabel>,
    outputs: Vec<OutputLabel>,
}

/// Diagnostic label for a graph node id.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct NodeLabel {
    /// Graph-local node id.
    pub id: NodeId,
    /// Stable diagnostic label for the node.
    pub label: String,
}

/// Diagnostic label for a graph scope id.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ScopeLabel {
    /// Graph-local scope id.
    pub id: ScopeId,
    /// Stable diagnostic label for the scope.
    pub label: String,
}

/// Diagnostic label for a resource key.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ResourceLabel {
    /// Structural resource identity.
    pub key: ResourceKey,
    /// Stable diagnostic label for the resource.
    pub label: String,
}

/// Diagnostic label for a materialized output key.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct OutputLabel {
    /// Graph-local output key.
    pub key: OutputKey,
    /// Stable diagnostic label for the output.
    pub label: String,
}

impl GraphLabelRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns whether no labels are recorded.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
            && self.scopes.is_empty()
            && self.resources.is_empty()
            && self.outputs.is_empty()
    }

    /// Returns node labels in stable node-id order.
    pub fn nodes(&self) -> &[NodeLabel] {
        &self.nodes
    }

    /// Returns scope labels in stable scope-id order.
    pub fn scopes(&self) -> &[ScopeLabel] {
        &self.scopes
    }

    /// Returns resource labels in stable resource-key order.
    pub fn resources(&self) -> &[ResourceLabel] {
        &self.resources
    }

    /// Returns output labels in stable output-key order.
    pub fn outputs(&self) -> &[OutputLabel] {
        &self.outputs
    }

    /// Records or replaces a node label.
    pub fn label_node(&mut self, id: NodeId, label: impl Into<String>) {
        upsert_by(
            &mut self.nodes,
            id,
            label.into(),
            |entry| entry.id,
            |id, label| NodeLabel { id, label },
        );
    }

    /// Records or replaces a scope label.
    pub fn label_scope(&mut self, id: ScopeId, label: impl Into<String>) {
        upsert_by(
            &mut self.scopes,
            id,
            label.into(),
            |entry| entry.id,
            |id, label| ScopeLabel { id, label },
        );
    }

    /// Records or replaces a resource label.
    pub fn label_resource(&mut self, key: ResourceKey, label: impl Into<String>) {
        upsert_by(
            &mut self.resources,
            key,
            label.into(),
            |entry| entry.key.clone(),
            |key, label| ResourceLabel { key, label },
        );
    }

    /// Records or replaces an output label.
    pub fn label_output(&mut self, key: OutputKey, label: impl Into<String>) {
        upsert_by(
            &mut self.outputs,
            key,
            label.into(),
            |entry| entry.key,
            |key, label| OutputLabel { key, label },
        );
    }

    /// Adds default labels for ids present in a transaction trace when absent.
    ///
    /// This is intended for structural trace files whose final graph no longer
    /// contains every historical resource or scope referenced by earlier steps.
    pub fn include_trace_defaults(&mut self, trace: &TransactionTrace) {
        for change in &trace.staged_input_changes {
            self.label_node_if_absent(change.node, node_fallback(change.node));
        }
        for node in trace
            .changed_inputs
            .iter()
            .chain(trace.dirty_roots.iter())
            .chain(trace.recomputed_derived_nodes.iter())
            .chain(trace.changed_derived_nodes.iter())
            .chain(trace.recomputed_collection_nodes.iter())
            .chain(trace.changed_collection_nodes.iter())
        {
            self.label_node_if_absent(*node, node_fallback(*node));
        }
        for diff in &trace.collection_diffs {
            self.label_node_if_absent(diff.node, node_fallback(diff.node));
        }
        for command in &trace.resource_commands {
            self.label_resource_if_absent(command.key.clone(), command.key.as_str().to_owned());
            self.label_scope_if_absent(command.scope, scope_fallback(command.scope));
        }
        for coalesced in &trace.resource_coalescences {
            self.label_resource_if_absent(coalesced.key.clone(), coalesced.key.as_str().to_owned());
            self.label_scope_if_absent(coalesced.scope, scope_fallback(coalesced.scope));
        }
        for frame in &trace.output_frames {
            self.label_output_if_absent(frame.output_key, output_fallback(frame.output_key));
            self.label_scope_if_absent(frame.scope, scope_fallback(frame.scope));
        }
        for event in &trace.scope_events {
            self.label_scope_if_absent(event.scope, scope_fallback(event.scope));
        }
        for entry in &trace.audit_log {
            self.include_audit_event_defaults(&entry.event);
        }
    }

    fn include_audit_event_defaults(&mut self, event: &AuditEvent) {
        match event {
            AuditEvent::InputChanged(node)
            | AuditEvent::InputUnchanged(node)
            | AuditEvent::DerivedChanged(node)
            | AuditEvent::CollectionChanged(node)
            | AuditEvent::NodeCreated(node) => {
                self.label_node_if_absent(*node, node_fallback(*node));
            }
            AuditEvent::ScopeCreated(scope) | AuditEvent::ScopeClosed(scope) => {
                self.label_scope_if_absent(*scope, scope_fallback(*scope));
            }
            AuditEvent::NodeAttached { node, scope } => {
                self.label_node_if_absent(*node, node_fallback(*node));
                self.label_scope_if_absent(*scope, scope_fallback(*scope));
            }
            AuditEvent::ResourceOpenCoalesced { key, scope, .. } => {
                self.label_resource_if_absent(key.clone(), key.as_str().to_owned());
                self.label_scope_if_absent(*scope, scope_fallback(*scope));
            }
        }
    }

    fn label_node_if_absent(&mut self, id: NodeId, label: String) {
        if !self.nodes.iter().any(|entry| entry.id == id) {
            self.label_node(id, label);
        }
    }

    fn label_scope_if_absent(&mut self, id: ScopeId, label: String) {
        if !self.scopes.iter().any(|entry| entry.id == id) {
            self.label_scope(id, label);
        }
    }

    fn label_resource_if_absent(&mut self, key: ResourceKey, label: String) {
        if !self.resources.iter().any(|entry| entry.key == key) {
            self.label_resource(key, label);
        }
    }

    fn label_output_if_absent(&mut self, key: OutputKey, label: String) {
        if !self.outputs.iter().any(|entry| entry.key == key) {
            self.label_output(key, label);
        }
    }
}

impl<C> Graph<C> {
    /// Exports deterministic labels for the graph ids currently known to core.
    pub fn label_registry(&self) -> GraphLabelRegistry {
        let mut registry = GraphLabelRegistry::new();
        for scope in self.scopes.values() {
            registry.label_scope(scope.id(), scope.debug_name());
        }
        for node in self.nodes.values() {
            registry.label_node(node.id(), node.debug_name());
        }
        for key in self.resource_owners.keys() {
            registry.label_resource(key.clone(), key.as_str());
        }
        for output in self.outputs.values() {
            registry.label_output(output.key(), output.debug_name());
        }
        registry
    }
}

fn upsert_by<T, K>(
    entries: &mut Vec<T>,
    key: K,
    label: String,
    entry_key: impl Fn(&T) -> K,
    make_entry: impl FnOnce(K, String) -> T,
) where
    K: Clone + Ord,
{
    if let Some(entry) = entries.iter_mut().find(|entry| entry_key(entry) == key) {
        *entry = make_entry(key, label);
    } else {
        entries.push(make_entry(key, label));
    }
    entries.sort_by_key(|entry| entry_key(entry));
}

fn node_fallback(id: NodeId) -> String {
    format!("node/{}", id.get())
}

fn scope_fallback(id: ScopeId) -> String {
    format!("scope/{}", id.get())
}

fn output_fallback(key: OutputKey) -> String {
    format!("output/{}", key.get())
}
