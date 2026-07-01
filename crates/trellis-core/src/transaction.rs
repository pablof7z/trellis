use crate::input::{StoredInput, boxed_input};
use crate::{
    CollectionNode, DependencyList, DerivedNode, Graph, GraphError, GraphResult, InputNode, NodeId,
    ScopeId, TransactionId,
    transaction_types::{AuditEntry, AuditEvent, TransactionOptions, TransactionResult},
};
use std::collections::BTreeMap;

/// Staged canonical input transaction.
pub struct Transaction<'graph> {
    graph: &'graph mut Graph,
    working: Graph,
    id: TransactionId,
    options: TransactionOptions,
    staged_inputs: BTreeMap<NodeId, Box<dyn StoredInput>>,
    staged_events: Vec<AuditEvent>,
    graph_mutated: bool,
    failed: Option<GraphError>,
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

    /// Stages creation of a root scope with no parent.
    pub fn create_scope(&mut self, debug_name: impl Into<String>) -> GraphResult<ScopeId> {
        self.ensure_open()?;
        let scope = self.graph.allocate_scope_id();
        let scope = self
            .working
            .create_scope_with_parent_direct(scope, debug_name, None)?;
        self.graph_mutated = true;
        self.staged_events.push(AuditEvent::ScopeCreated(scope));
        Ok(scope)
    }

    /// Stages creation of a scope with an optional parent.
    pub fn create_scope_with_parent(
        &mut self,
        debug_name: impl Into<String>,
        parent: Option<ScopeId>,
    ) -> GraphResult<ScopeId> {
        self.ensure_open()?;
        let scope = self.graph.allocate_scope_id();
        match self
            .working
            .create_scope_with_parent_direct(scope, debug_name, parent)
        {
            Ok(scope) => {
                self.graph_mutated = true;
                self.staged_events.push(AuditEvent::ScopeCreated(scope));
                Ok(scope)
            }
            Err(error) => {
                self.failed.get_or_insert_with(|| error.clone());
                Err(error)
            }
        }
    }

    /// Stages creation of an input node.
    pub fn input<T>(&mut self, debug_name: impl Into<String>) -> GraphResult<InputNode<T>>
    where
        T: Clone + PartialEq + 'static,
    {
        self.ensure_open()?;
        let id = self.graph.allocate_node_id();
        let input = self.working.input_direct(id, debug_name)?;
        self.graph_mutated = true;
        self.staged_events.push(AuditEvent::NodeCreated(input.id()));
        Ok(input)
    }

    /// Stages creation of a derived node with explicit dependencies.
    pub fn derived<T>(
        &mut self,
        debug_name: impl Into<String>,
        dependencies: DependencyList,
    ) -> GraphResult<DerivedNode<T>> {
        self.ensure_open()?;
        let id = self.graph.allocate_node_id();
        match self.working.derived_direct(id, debug_name, dependencies) {
            Ok(derived) => {
                self.graph_mutated = true;
                self.staged_events
                    .push(AuditEvent::NodeCreated(derived.id()));
                Ok(derived)
            }
            Err(error) => {
                self.failed.get_or_insert_with(|| error.clone());
                Err(error)
            }
        }
    }

    /// Stages creation of a collection node with explicit dependencies.
    pub fn collection<K, V>(
        &mut self,
        debug_name: impl Into<String>,
        dependencies: DependencyList,
    ) -> GraphResult<CollectionNode<K, V>> {
        self.ensure_open()?;
        let id = self.graph.allocate_node_id();
        match self.working.collection_direct(id, debug_name, dependencies) {
            Ok(collection) => {
                self.graph_mutated = true;
                self.staged_events
                    .push(AuditEvent::NodeCreated(collection.id()));
                Ok(collection)
            }
            Err(error) => {
                self.failed.get_or_insert_with(|| error.clone());
                Err(error)
            }
        }
    }

    /// Stages attaching a node to an owning scope.
    pub fn attach_node_to_scope(
        &mut self,
        node: impl crate::NodeHandle,
        scope: ScopeId,
    ) -> GraphResult<()> {
        self.ensure_open()?;
        let node_id = node.id();
        match self.working.attach_node_to_scope_direct(node_id, scope) {
            Ok(()) => {
                self.graph_mutated = true;
                self.staged_events.push(AuditEvent::NodeAttached {
                    node: node_id,
                    scope,
                });
                Ok(())
            }
            Err(error) => {
                self.failed.get_or_insert_with(|| error.clone());
                Err(error)
            }
        }
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

        let audit_log = audit_events
            .into_iter()
            .map(|event| AuditEntry {
                transaction_id: self.id,
                revision: next_revision,
                event,
            })
            .collect();

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
        self.working.revision = next_revision;
        self.working.next_node_id = self.graph.next_node_id;
        self.working.next_scope_id = self.graph.next_scope_id;

        let result = TransactionResult {
            transaction_id: self.id,
            revision: next_revision,
            changed_inputs,
            audit_log,
        };
        *self.graph = self.working.clone();
        self.graph.transaction_open = true;
        self.close();
        Ok(result)
    }

    fn ensure_open(&self) -> GraphResult<()> {
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
