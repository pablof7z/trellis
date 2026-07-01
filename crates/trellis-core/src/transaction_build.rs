use crate::{
    AuditEvent, CollectionNode, DependencyList, DeriveContext, DeriveError, DerivedNode,
    GraphResult, InputNode, ScopeId, Transaction,
};

impl Transaction<'_> {
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
        derive: impl for<'ctx> Fn(&DeriveContext<'ctx>) -> Result<T, DeriveError> + 'static,
    ) -> GraphResult<DerivedNode<T>>
    where
        T: Clone + PartialEq + 'static,
    {
        self.ensure_open()?;
        let id = self.graph.allocate_node_id();
        match self
            .working
            .derived_direct(id, debug_name, dependencies, derive)
        {
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
}
