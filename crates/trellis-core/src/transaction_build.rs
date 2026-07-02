use crate::{
    AuditEvent, CollectionContext, CollectionNode, DependencyList, DeriveContext, DeriveError,
    DerivedNode, GraphResult, InputNode, ScopeId, Transaction,
};
use std::collections::{BTreeMap, BTreeSet};

impl<C: 'static> Transaction<'_, C> {
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

    /// Stages closing a scope for scoped resource, output, and node teardown.
    pub fn close_scope(&mut self, scope: ScopeId) -> GraphResult<()> {
        self.ensure_open()?;
        match self.working.close_scope_direct(scope) {
            Ok(closed_scopes) => {
                if !closed_scopes.is_empty() {
                    self.graph_mutated = true;
                }
                self.staged_events
                    .extend(closed_scopes.into_iter().map(AuditEvent::ScopeClosed));
                Ok(())
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
        T: Clone + PartialEq + Send + Sync + 'static,
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
        derive: impl for<'ctx> Fn(&DeriveContext<'ctx, C>) -> Result<T, DeriveError>
        + Send
        + Sync
        + 'static,
    ) -> GraphResult<DerivedNode<T>>
    where
        T: Clone + PartialEq + Send + Sync + 'static,
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

    /// Stages creation of a map collection node with explicit dependencies.
    pub fn collection<K, V>(
        &mut self,
        debug_name: impl Into<String>,
        dependencies: DependencyList,
        derive: impl for<'ctx> Fn(&CollectionContext<'ctx, C>) -> Result<BTreeMap<K, V>, DeriveError>
        + Send
        + Sync
        + 'static,
    ) -> GraphResult<CollectionNode<K, V>>
    where
        K: Clone + Ord + Send + Sync + 'static,
        V: Clone + PartialEq + Send + Sync + 'static,
    {
        self.map_collection(debug_name, dependencies, derive)
    }

    /// Stages creation of a map collection node with explicit dependencies.
    pub fn map_collection<K, V>(
        &mut self,
        debug_name: impl Into<String>,
        dependencies: DependencyList,
        derive: impl for<'ctx> Fn(&CollectionContext<'ctx, C>) -> Result<BTreeMap<K, V>, DeriveError>
        + Send
        + Sync
        + 'static,
    ) -> GraphResult<CollectionNode<K, V>>
    where
        K: Clone + Ord + Send + Sync + 'static,
        V: Clone + PartialEq + Send + Sync + 'static,
    {
        self.ensure_open()?;
        let id = self.graph.allocate_node_id();
        match self
            .working
            .collection_map_direct(id, debug_name, dependencies, derive)
        {
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

    /// Stages creation of a set collection node with explicit dependencies.
    pub fn set_collection<K>(
        &mut self,
        debug_name: impl Into<String>,
        dependencies: DependencyList,
        derive: impl for<'ctx> Fn(&CollectionContext<'ctx, C>) -> Result<BTreeSet<K>, DeriveError>
        + Send
        + Sync
        + 'static,
    ) -> GraphResult<CollectionNode<K, ()>>
    where
        K: Clone + Ord + Send + Sync + 'static,
    {
        self.ensure_open()?;
        let id = self.graph.allocate_node_id();
        match self
            .working
            .collection_set_direct(id, debug_name, dependencies, derive)
        {
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
