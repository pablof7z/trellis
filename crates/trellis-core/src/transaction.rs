use crate::input::{StoredInput, boxed_input};
use crate::{
    Graph, GraphError, GraphResult, InputNode, NodeId, TransactionId,
    transaction_types::{AuditEntry, AuditEvent, TransactionOptions, TransactionResult},
};
use std::collections::BTreeMap;

/// Staged canonical input transaction.
pub struct Transaction<'graph> {
    pub(crate) graph: &'graph mut Graph,
    pub(crate) working: Graph,
    id: TransactionId,
    options: TransactionOptions,
    staged_inputs: BTreeMap<NodeId, Box<dyn StoredInput>>,
    pub(crate) staged_events: Vec<AuditEvent>,
    pub(crate) graph_mutated: bool,
    pub(crate) failed: Option<GraphError>,
    closed: bool,
}

impl<'graph> Transaction<'graph> {
    pub(crate) fn new(
        graph: &'graph mut Graph,
        id: TransactionId,
        options: TransactionOptions,
    ) -> Self {
        let mut working = graph.clone();
        working.transaction_open = false;
        Self {
            graph,
            working,
            id,
            options,
            staged_inputs: BTreeMap::new(),
            staged_events: Vec::new(),
            graph_mutated: false,
            failed: None,
            closed: false,
        }
    }

    /// Returns this transaction's id.
    pub fn id(&self) -> TransactionId {
        self.id
    }

    /// Stages a typed canonical input change.
    pub fn set_input<T>(&mut self, input: InputNode<T>, value: T) -> GraphResult<()>
    where
        T: Clone + PartialEq + 'static,
    {
        self.set_input_by_id(input.id(), value)
    }

    /// Stages a canonical input change by node id.
    pub fn set_input_by_id<T>(&mut self, node: NodeId, value: T) -> GraphResult<()>
    where
        T: Clone + PartialEq + 'static,
    {
        self.ensure_open()?;
        if let Err(error) = self.working.validate_input_write::<T>(node) {
            self.failed.get_or_insert_with(|| error.clone());
            return Err(error);
        }
        self.staged_inputs.insert(node, boxed_input(value));
        Ok(())
    }

    /// Commits staged input changes atomically.
    pub fn commit(&mut self) -> GraphResult<TransactionResult> {
        self.ensure_open()?;
        if let Some(error) = self.failed.clone() {
            self.close();
            return Err(error);
        }

        let mut changed_inputs = Vec::new();
        for (node, staged) in &self.staged_inputs {
            let changed = self
                .working
                .input_values
                .get(node)
                .is_none_or(|current| !current.equals(staged.as_ref()));
            if changed || !self.options.skip_equal_inputs {
                changed_inputs.push(*node);
            }
        }

        let next_revision = if changed_inputs.is_empty() && !self.graph_mutated {
            self.graph.revision
        } else {
            self.graph.revision.next()
        };

        let mut audit_events = self.staged_events.clone();
        for node in self.staged_inputs.keys() {
            let event = if changed_inputs.contains(node) {
                AuditEvent::InputChanged(*node)
            } else {
                AuditEvent::InputUnchanged(*node)
            };
            audit_events.push(event);
        }

        for node in &changed_inputs {
            if let Some(staged) = self.staged_inputs.get(node) {
                self.working.input_values.insert(*node, staged.clone());
                if let Some(meta) = self.working.nodes.get_mut(node) {
                    meta.mark_changed(next_revision);
                }
            }
        }
        for event in &self.staged_events {
            if let AuditEvent::NodeCreated(node) = event
                && let Some(meta) = self.working.nodes.get_mut(node)
            {
                meta.mark_created(next_revision);
            }
        }
        let created_nodes: Vec<NodeId> = self
            .staged_events
            .iter()
            .filter_map(|event| match event {
                AuditEvent::NodeCreated(node) => Some(*node),
                _ => None,
            })
            .collect();
        let mut initial_changed = changed_inputs.clone();
        initial_changed.extend(created_nodes);
        let changed_derived_nodes = match self.working.recompute_dirty_derived(&initial_changed) {
            Ok(nodes) => nodes,
            Err(error) => {
                self.close();
                return Err(error);
            }
        };
        for node in &changed_derived_nodes {
            if let Some(meta) = self.working.nodes.get_mut(node) {
                meta.mark_changed(next_revision);
            }
            audit_events.push(AuditEvent::DerivedChanged(*node));
        }
        let audit_log = audit_events
            .into_iter()
            .map(|event| AuditEntry {
                transaction_id: self.id,
                revision: next_revision,
                event,
            })
            .collect();
        self.working.revision = next_revision;
        self.working.next_node_id = self.graph.next_node_id;
        self.working.next_scope_id = self.graph.next_scope_id;

        let result = TransactionResult {
            transaction_id: self.id,
            revision: next_revision,
            changed_inputs,
            changed_derived_nodes,
            audit_log,
        };
        *self.graph = self.working.clone();
        self.graph.transaction_open = true;
        self.close();
        Ok(result)
    }

    pub(crate) fn ensure_open(&self) -> GraphResult<()> {
        if self.closed {
            Err(GraphError::TransactionClosed(self.id))
        } else {
            Ok(())
        }
    }

    fn close(&mut self) {
        self.closed = true;
        self.graph.transaction_open = false;
    }
}

impl Drop for Transaction<'_> {
    fn drop(&mut self) {
        if !self.closed {
            self.graph.transaction_open = false;
        }
    }
}
