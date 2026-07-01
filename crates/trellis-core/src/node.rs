use crate::{DependencyList, NodeId, Revision, ScopeId};
use core::any::TypeId;
use core::marker::PhantomData;

/// The metadata kind of a graph node.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum NodeKind {
    /// Canonical input supplied by the host.
    Input,
    /// Value derived from declared dependencies.
    Derived,
    /// Collection derived from declared dependencies.
    Collection,
}

/// Common behavior for typed node handles.
pub trait NodeHandle {
    /// Returns the graph-local node id backing this typed handle.
    fn id(self) -> NodeId;
}

/// Typed handle for an input node.
#[derive(Debug, Eq, PartialEq, Hash)]
pub struct InputNode<T> {
    id: NodeId,
    _marker: PhantomData<fn() -> T>,
}

impl<T> InputNode<T> {
    pub(crate) fn new(id: NodeId) -> Self {
        Self {
            id,
            _marker: PhantomData,
        }
    }

    /// Returns the graph-local node id.
    pub fn id(&self) -> NodeId {
        self.id
    }
}

impl<T> Copy for InputNode<T> {}

impl<T> Clone for InputNode<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> NodeHandle for InputNode<T> {
    fn id(self) -> NodeId {
        self.id
    }
}

/// Typed handle for a derived node.
#[derive(Debug, Eq, PartialEq, Hash)]
pub struct DerivedNode<T> {
    id: NodeId,
    _marker: PhantomData<fn() -> T>,
}

impl<T> DerivedNode<T> {
    pub(crate) fn new(id: NodeId) -> Self {
        Self {
            id,
            _marker: PhantomData,
        }
    }

    /// Returns the graph-local node id.
    pub fn id(&self) -> NodeId {
        self.id
    }
}

impl<T> Copy for DerivedNode<T> {}

impl<T> Clone for DerivedNode<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> NodeHandle for DerivedNode<T> {
    fn id(self) -> NodeId {
        self.id
    }
}

/// Typed handle for a collection node.
#[derive(Debug, Eq, PartialEq, Hash)]
pub struct CollectionNode<K, V> {
    id: NodeId,
    _marker: PhantomData<fn() -> (K, V)>,
}

impl<K, V> CollectionNode<K, V> {
    pub(crate) fn new(id: NodeId) -> Self {
        Self {
            id,
            _marker: PhantomData,
        }
    }

    /// Returns the graph-local node id.
    pub fn id(&self) -> NodeId {
        self.id
    }
}

impl<K, V> Copy for CollectionNode<K, V> {}

impl<K, V> Clone for CollectionNode<K, V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<K, V> NodeHandle for CollectionNode<K, V> {
    fn id(self) -> NodeId {
        self.id
    }
}

/// Inspectable metadata for a graph node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeMeta {
    id: NodeId,
    kind: NodeKind,
    debug_name: String,
    dependencies: DependencyList,
    owning_scope: Option<ScopeId>,
    created_revision: Revision,
    last_changed_revision: Revision,
    value_type: Option<TypeId>,
}

impl NodeMeta {
    pub(crate) fn new(
        id: NodeId,
        kind: NodeKind,
        debug_name: impl Into<String>,
        dependencies: DependencyList,
        created_revision: Revision,
        value_type: Option<TypeId>,
    ) -> Self {
        Self {
            id,
            kind,
            debug_name: debug_name.into(),
            dependencies,
            owning_scope: None,
            created_revision,
            last_changed_revision: created_revision,
            value_type,
        }
    }

    /// Returns this node's id.
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// Returns this node's kind.
    pub fn kind(&self) -> NodeKind {
        self.kind
    }

    /// Returns this node's debug name.
    pub fn debug_name(&self) -> &str {
        &self.debug_name
    }

    /// Returns this node's declared dependencies.
    pub fn dependencies(&self) -> &DependencyList {
        &self.dependencies
    }

    /// Returns this node's owning scope, if one has been attached.
    pub fn owning_scope(&self) -> Option<ScopeId> {
        self.owning_scope
    }

    /// Returns the graph revision at which this node was created.
    pub fn created_revision(&self) -> Revision {
        self.created_revision
    }

    /// Returns the graph revision at which this node last changed.
    pub fn last_changed_revision(&self) -> Revision {
        self.last_changed_revision
    }

    pub(crate) fn attach_scope(&mut self, scope: ScopeId) {
        self.owning_scope = Some(scope);
    }

    pub(crate) fn detach_scope(&mut self, scope: ScopeId) {
        if self.owning_scope == Some(scope) {
            self.owning_scope = None;
        }
    }

    pub(crate) fn value_type(&self) -> Option<TypeId> {
        self.value_type
    }

    pub(crate) fn mark_changed(&mut self, revision: Revision) {
        self.last_changed_revision = revision;
    }

    pub(crate) fn mark_created(&mut self, revision: Revision) {
        self.created_revision = revision;
        self.last_changed_revision = revision;
    }
}
