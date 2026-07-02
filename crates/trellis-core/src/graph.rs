use crate::{
    AuditState, DependencyList, DerivedNode, GraphError, GraphResult, InputNode, NodeHandle,
    NodeId, NodeKind, NodeMeta, OutputKey, OutputMeta, ResourceKey, Revision, ScopeId, ScopeMeta,
    Transaction, TransactionId, TransactionOptions,
    collection::{CollectionSpec, StoredCollection, StoredDiff},
    derive::DerivedSpec,
    input::{StoredInput, value_type},
    output::OutputSpec,
    output_payload::StoredOutput,
    resource::ResourcePlanner,
    topology::TopologyCache,
};
use std::collections::{BTreeMap, BTreeSet};

/// Trellis graph skeleton with transactional input mutation.
pub struct Graph<C = ()> {
    pub(crate) next_node_id: u64,
    pub(crate) next_scope_id: u64,
    pub(crate) next_output_key: u64,
    pub(crate) next_transaction_id: TransactionId,
    pub(crate) revision: Revision,
    pub(crate) nodes: BTreeMap<NodeId, NodeMeta>,
    pub(crate) scopes: BTreeMap<ScopeId, ScopeMeta>,
    pub(crate) scope_children: BTreeMap<ScopeId, BTreeSet<ScopeId>>,
    pub(crate) input_values: BTreeMap<NodeId, Box<dyn StoredInput>>,
    pub(crate) derived_specs: BTreeMap<NodeId, DerivedSpec<C>>,
    pub(crate) derived_values: BTreeMap<NodeId, Box<dyn StoredInput>>,
    pub(crate) collection_specs: BTreeMap<NodeId, CollectionSpec<C>>,
    pub(crate) collection_values: BTreeMap<NodeId, Box<dyn StoredCollection>>,
    pub(crate) previous_collection_values: BTreeMap<NodeId, Box<dyn StoredCollection>>,
    pub(crate) collection_diffs: BTreeMap<NodeId, Box<dyn StoredDiff>>,
    pub(crate) resource_planners: Vec<ResourcePlanner<C>>,
    pub(crate) resource_owners: BTreeMap<ResourceKey, BTreeSet<ScopeId>>,
    pub(crate) output_specs: BTreeMap<OutputKey, OutputSpec<C>>,
    pub(crate) output_values: BTreeMap<OutputKey, Box<dyn StoredOutput>>,
    pub(crate) outputs: BTreeMap<OutputKey, OutputMeta>,
    pub(crate) topology_cache: TopologyCache,
    pub(crate) audit: AuditState,
    pub(crate) transaction_open: bool,
}

impl<C> Graph<C> {
    /// Creates an empty graph.
    pub fn new_with_command_type() -> Self {
        Self {
            next_node_id: 1,
            next_scope_id: 1,
            next_output_key: 1,
            next_transaction_id: TransactionId::default(),
            revision: Revision::default(),
            nodes: BTreeMap::new(),
            scopes: BTreeMap::new(),
            scope_children: BTreeMap::new(),
            input_values: BTreeMap::new(),
            derived_specs: BTreeMap::new(),
            derived_values: BTreeMap::new(),
            collection_specs: BTreeMap::new(),
            collection_values: BTreeMap::new(),
            previous_collection_values: BTreeMap::new(),
            collection_diffs: BTreeMap::new(),
            resource_planners: Vec::new(),
            resource_owners: BTreeMap::new(),
            output_specs: BTreeMap::new(),
            output_values: BTreeMap::new(),
            outputs: BTreeMap::new(),
            topology_cache: TopologyCache::default(),
            audit: AuditState::default(),
            transaction_open: false,
        }
    }

    /// Returns the graph revision.
    pub fn revision(&self) -> Revision {
        self.revision
    }

    /// Begins an input transaction with default options.
    pub fn begin_transaction(&mut self) -> GraphResult<Transaction<'_, C>> {
        self.begin_transaction_with_options(TransactionOptions::default())
    }

    /// Begins an input transaction with explicit options.
    pub fn begin_transaction_with_options(
        &mut self,
        options: TransactionOptions,
    ) -> GraphResult<Transaction<'_, C>> {
        if self.transaction_open {
            return Err(GraphError::NestedTransaction);
        }

        self.transaction_open = true;
        let id = self.allocate_transaction_id();
        Ok(Transaction::new(self, id, options))
    }

    pub(crate) fn create_scope_with_parent_direct(
        &mut self,
        id: ScopeId,
        debug_name: impl Into<String>,
        parent: Option<ScopeId>,
    ) -> GraphResult<ScopeId> {
        if let Some(parent) = parent {
            let parent_meta = self.require_scope(parent)?;
            if parent_meta.is_closed() {
                return Err(GraphError::ScopeAlreadyClosed(parent));
            }
        }

        self.scopes
            .insert(id, ScopeMeta::new(id, debug_name, parent));
        if let Some(parent) = parent {
            self.scope_children.entry(parent).or_default().insert(id);
        }
        Ok(id)
    }

    pub(crate) fn input_direct<T>(
        &mut self,
        id: NodeId,
        debug_name: impl Into<String>,
    ) -> GraphResult<InputNode<T>>
    where
        T: Clone + PartialEq + Send + Sync + 'static,
    {
        let meta = NodeMeta::new(
            id,
            NodeKind::Input,
            debug_name,
            DependencyList::empty(),
            self.revision,
            Some(value_type::<T>()),
        );
        self.invalidate_topology_cache();
        self.nodes.insert(id, meta);
        Ok(InputNode::new(id))
    }

    pub(crate) fn derived_direct<T>(
        &mut self,
        id: NodeId,
        debug_name: impl Into<String>,
        dependencies: DependencyList,
        derive: impl for<'ctx> Fn(&crate::DeriveContext<'ctx, C>) -> Result<T, crate::DeriveError>
        + Send
        + Sync
        + 'static,
    ) -> GraphResult<DerivedNode<T>>
    where
        T: Clone + PartialEq + Send + Sync + 'static,
    {
        self.validate_dependencies(id, &dependencies)?;
        self.reject_collection_dependencies(&dependencies)?;
        let meta = NodeMeta::new(
            id,
            NodeKind::Derived,
            debug_name,
            dependencies,
            self.revision,
            Some(value_type::<T>()),
        );
        self.invalidate_topology_cache();
        self.nodes.insert(id, meta);
        self.derived_specs.insert(id, DerivedSpec::<C>::new(derive));
        Ok(DerivedNode::new(id))
    }

    pub(crate) fn attach_node_to_scope_direct(
        &mut self,
        node_id: NodeId,
        scope: ScopeId,
    ) -> GraphResult<()> {
        let scope_meta = self.require_scope(scope)?;
        if scope_meta.is_closed() {
            return Err(GraphError::ScopeAlreadyClosed(scope));
        }

        let node_meta = self
            .nodes
            .get_mut(&node_id)
            .ok_or(GraphError::UnknownNode(node_id))?;

        if node_meta.owning_scope().is_some() {
            return Err(GraphError::NodeAlreadyAttached(node_id));
        }

        node_meta.attach_scope(scope);
        Ok(())
    }

    /// Returns metadata for a node.
    pub fn node_meta<H: NodeHandle>(&self, node: H) -> Option<&NodeMeta> {
        self.nodes.get(&node.id())
    }

    /// Returns metadata for a node id.
    pub fn node_meta_by_id(&self, id: NodeId) -> Option<&NodeMeta> {
        self.nodes.get(&id)
    }

    /// Returns metadata for a scope.
    pub fn scope_meta(&self, id: ScopeId) -> Option<&ScopeMeta> {
        self.scopes.get(&id)
    }

    /// Returns metadata for a materialized output.
    pub fn output_meta(&self, key: OutputKey) -> Option<&OutputMeta> {
        self.outputs.get(&key)
    }

    /// Returns declared dependencies for a node.
    pub fn dependencies<H: NodeHandle>(&self, node: H) -> Option<&DependencyList> {
        self.node_meta(node).map(NodeMeta::dependencies)
    }

    /// Returns all node metadata in stable id order.
    pub fn nodes(&self) -> impl Iterator<Item = &NodeMeta> {
        self.nodes.values()
    }

    /// Returns all scope metadata in stable id order.
    pub fn scopes(&self) -> impl Iterator<Item = &ScopeMeta> {
        self.scopes.values()
    }

    pub(crate) fn allocate_node_id(&mut self) -> NodeId {
        let id = NodeId::from_index(self.next_node_id);
        self.next_node_id += 1;
        id
    }

    pub(crate) fn allocate_scope_id(&mut self) -> ScopeId {
        let id = ScopeId::from_index(self.next_scope_id);
        self.next_scope_id += 1;
        id
    }

    pub(crate) fn allocate_output_key(&mut self) -> OutputKey {
        let key = OutputKey::from_index(self.next_output_key);
        self.next_output_key += 1;
        key
    }

    fn allocate_transaction_id(&mut self) -> TransactionId {
        self.next_transaction_id = self.next_transaction_id.next();
        self.next_transaction_id
    }

    pub(crate) fn require_scope(&self, id: ScopeId) -> GraphResult<&ScopeMeta> {
        self.scopes.get(&id).ok_or(GraphError::UnknownScope(id))
    }
}
