use crate::{
    CollectionNode, DependencyList, DerivedNode, GraphError, GraphResult, InputNode, NodeHandle,
    NodeId, NodeKind, NodeMeta, Revision, ScopeId, ScopeMeta,
};
use std::collections::BTreeMap;

/// Metadata-only Trellis graph skeleton.
#[derive(Clone, Debug)]
pub struct Graph {
    next_node_id: u64,
    next_scope_id: u64,
    revision: Revision,
    nodes: BTreeMap<NodeId, NodeMeta>,
    scopes: BTreeMap<ScopeId, ScopeMeta>,
}

impl Graph {
    /// Creates an empty graph.
    pub fn new() -> Self {
        Self {
            next_node_id: 1,
            next_scope_id: 1,
            revision: Revision::default(),
            nodes: BTreeMap::new(),
            scopes: BTreeMap::new(),
        }
    }

    /// Returns the graph revision.
    pub fn revision(&self) -> Revision {
        self.revision
    }

    /// Creates a root scope with no parent.
    pub fn create_scope(&mut self, debug_name: impl Into<String>) -> ScopeId {
        self.create_scope_with_parent(debug_name, None)
            .expect("None parent is always valid")
    }

    /// Creates a scope with an optional parent scope.
    pub fn create_scope_with_parent(
        &mut self,
        debug_name: impl Into<String>,
        parent: Option<ScopeId>,
    ) -> GraphResult<ScopeId> {
        if let Some(parent) = parent {
            self.require_scope(parent)?;
        }

        let id = self.allocate_scope_id();
        self.scopes
            .insert(id, ScopeMeta::new(id, debug_name, parent));
        Ok(id)
    }

    /// Creates an input node.
    pub fn input<T>(&mut self, debug_name: impl Into<String>) -> InputNode<T> {
        let id = self.allocate_node_id();
        let meta = NodeMeta::new(
            id,
            NodeKind::Input,
            debug_name,
            DependencyList::empty(),
            self.revision,
        );
        self.nodes.insert(id, meta);
        InputNode::new(id)
    }

    /// Creates a derived node with explicit dependencies.
    pub fn derived<T>(
        &mut self,
        debug_name: impl Into<String>,
        dependencies: DependencyList,
    ) -> GraphResult<DerivedNode<T>> {
        let id = self.next_node_id();
        self.validate_dependencies(id, &dependencies)?;
        let id = self.allocate_node_id();
        let meta = NodeMeta::new(
            id,
            NodeKind::Derived,
            debug_name,
            dependencies,
            self.revision,
        );
        self.nodes.insert(id, meta);
        Ok(DerivedNode::new(id))
    }

    /// Creates a collection node with explicit dependencies.
    pub fn collection<K, V>(
        &mut self,
        debug_name: impl Into<String>,
        dependencies: DependencyList,
    ) -> GraphResult<CollectionNode<K, V>> {
        let id = self.next_node_id();
        self.validate_dependencies(id, &dependencies)?;
        let id = self.allocate_node_id();
        let meta = NodeMeta::new(
            id,
            NodeKind::Collection,
            debug_name,
            dependencies,
            self.revision,
        );
        self.nodes.insert(id, meta);
        Ok(CollectionNode::new(id))
    }

    /// Attaches a node to an owning scope.
    pub fn attach_node_to_scope<H: NodeHandle>(
        &mut self,
        node: H,
        scope: ScopeId,
    ) -> GraphResult<()> {
        let scope_meta = self.require_scope(scope)?;
        if scope_meta.is_closed() {
            return Err(GraphError::ScopeAlreadyClosed(scope));
        }

        let node_id = node.id();
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

    fn allocate_node_id(&mut self) -> NodeId {
        let id = NodeId::from_index(self.next_node_id);
        self.next_node_id += 1;
        id
    }

    fn next_node_id(&self) -> NodeId {
        NodeId::from_index(self.next_node_id)
    }

    fn allocate_scope_id(&mut self) -> ScopeId {
        let id = ScopeId::from_index(self.next_scope_id);
        self.next_scope_id += 1;
        id
    }

    fn require_scope(&self, id: ScopeId) -> GraphResult<&ScopeMeta> {
        self.scopes.get(&id).ok_or(GraphError::UnknownScope(id))
    }

    fn validate_dependencies(
        &self,
        node_id: NodeId,
        dependencies: &DependencyList,
    ) -> GraphResult<()> {
        for dependency in dependencies.as_slice() {
            if *dependency == node_id {
                return Err(GraphError::SelfDependency(node_id));
            }
            if !self.nodes.contains_key(dependency) {
                return Err(GraphError::UnknownNode(*dependency));
            }
        }
        Ok(())
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}
