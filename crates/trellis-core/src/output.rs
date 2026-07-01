use crate::collection::{downcast_map, downcast_set};
use crate::input::downcast_input;
use crate::{
    CollectionNode, DependencyList, DeriveError, DerivedNode, Graph, InputNode, NodeId, OutputKey,
    Revision, ScopeId, TransactionId,
};
use core::marker::PhantomData;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

type OutputFn<C, O> = dyn for<'ctx> Fn(&OutputContext<'ctx, C, O>) -> Result<O, DeriveError>;

/// Typed handle for a materialized output surface.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct MaterializedOutput<O> {
    key: OutputKey,
    _marker: PhantomData<fn() -> O>,
}

impl<O> MaterializedOutput<O> {
    pub(crate) fn new(key: OutputKey) -> Self {
        Self {
            key,
            _marker: PhantomData,
        }
    }

    /// Returns this output's graph-local key.
    pub fn key(&self) -> OutputKey {
        self.key
    }
}

/// Per-output emission options.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct OutputOptions {
    /// Emit a delta when dependencies changed but materialized value is equal.
    pub emit_equal: bool,
}

/// Inspectable metadata for a materialized output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputMeta {
    key: OutputKey,
    debug_name: String,
    scope: ScopeId,
    dependencies: DependencyList,
    options: OutputOptions,
    created_revision: Revision,
}

impl OutputMeta {
    pub(crate) fn new(
        key: OutputKey,
        debug_name: impl Into<String>,
        scope: ScopeId,
        dependencies: DependencyList,
        options: OutputOptions,
        created_revision: Revision,
    ) -> Self {
        Self {
            key,
            debug_name: debug_name.into(),
            scope,
            dependencies,
            options,
            created_revision,
        }
    }

    /// Returns this output's key.
    pub fn key(&self) -> OutputKey {
        self.key
    }

    /// Returns this output's debug name.
    pub fn debug_name(&self) -> &str {
        &self.debug_name
    }

    /// Returns this output's owning scope.
    pub fn scope(&self) -> ScopeId {
        self.scope
    }

    /// Returns this output's explicit dependencies.
    pub fn dependencies(&self) -> &DependencyList {
        &self.dependencies
    }

    /// Returns this output's emission options.
    pub fn options(&self) -> OutputOptions {
        self.options
    }

    /// Returns the graph revision at which this output was created.
    pub fn created_revision(&self) -> Revision {
        self.created_revision
    }
}

pub(crate) struct OutputSpec<C, O> {
    materialize: Arc<OutputFn<C, O>>,
}

impl<C, O> Clone for OutputSpec<C, O> {
    fn clone(&self) -> Self {
        Self {
            materialize: Arc::clone(&self.materialize),
        }
    }
}

impl<C, O> OutputSpec<C, O> {
    pub(crate) fn new(
        materialize: impl for<'ctx> Fn(&OutputContext<'ctx, C, O>) -> Result<O, DeriveError> + 'static,
    ) -> Self {
        Self {
            materialize: Arc::new(materialize),
        }
    }

    pub(crate) fn materialize(&self, ctx: &OutputContext<'_, C, O>) -> Result<O, DeriveError> {
        (self.materialize)(ctx)
    }
}

/// Read-only context passed to materialized output computations.
pub struct OutputContext<'graph, C = (), O = ()> {
    graph: &'graph Graph<C, O>,
    declared_dependencies: &'graph [NodeId],
}

impl<'graph, C, O> OutputContext<'graph, C, O> {
    pub(crate) fn new(graph: &'graph Graph<C, O>, declared_dependencies: &'graph [NodeId]) -> Self {
        Self {
            graph,
            declared_dependencies,
        }
    }

    /// Reads a declared input dependency.
    pub fn input<T>(&self, input: InputNode<T>) -> Result<&'graph T, DeriveError>
    where
        T: Clone + PartialEq + 'static,
    {
        let node = input.id();
        self.require_declared(node)?;
        self.graph
            .input_values
            .get(&node)
            .and_then(|value| downcast_input::<T>(value.as_ref()))
            .ok_or(DeriveError::MissingValue(node))
    }

    /// Reads a declared scalar derived dependency.
    pub fn derived<T>(&self, derived: DerivedNode<T>) -> Result<&'graph T, DeriveError>
    where
        T: Clone + PartialEq + 'static,
    {
        let node = derived.id();
        self.require_declared(node)?;
        self.graph
            .derived_values
            .get(&node)
            .and_then(|value| downcast_input::<T>(value.as_ref()))
            .ok_or(DeriveError::MissingValue(node))
    }

    /// Reads a declared map collection dependency.
    pub fn map_collection<K, V>(
        &self,
        collection: CollectionNode<K, V>,
    ) -> Result<&'graph BTreeMap<K, V>, DeriveError>
    where
        K: Clone + Ord + 'static,
        V: Clone + PartialEq + 'static,
    {
        let node = collection.id();
        self.require_declared(node)?;
        self.graph
            .validate_map_collection_read::<K, V>(node)
            .map_err(|_| DeriveError::WrongCollectionType(node))?;
        self.graph
            .collection_values
            .get(&node)
            .and_then(|value| downcast_map::<K, V>(value.as_ref()))
            .ok_or(DeriveError::MissingValue(node))
    }

    /// Reads a declared set collection dependency.
    pub fn set_collection<K>(
        &self,
        collection: CollectionNode<K, ()>,
    ) -> Result<&'graph BTreeSet<K>, DeriveError>
    where
        K: Clone + Ord + 'static,
    {
        let node = collection.id();
        self.require_declared(node)?;
        self.graph
            .validate_set_collection_read::<K>(node)
            .map_err(|_| DeriveError::WrongCollectionType(node))?;
        self.graph
            .collection_values
            .get(&node)
            .and_then(|value| downcast_set::<K>(value.as_ref()))
            .ok_or(DeriveError::MissingValue(node))
    }

    fn require_declared(&self, node: NodeId) -> Result<(), DeriveError> {
        if self.declared_dependencies.contains(&node) {
            Ok(())
        } else {
            Err(DeriveError::UndeclaredDependency(node))
        }
    }
}

/// Reason a materialized output was cleared.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ClearReason {
    /// The owning scope was closed.
    ScopeClosed,
}

/// Reason a materialized output was rebaselined.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum RebaselineReason {
    /// The host explicitly requested a rebaseline.
    Requested,
}

/// Data-only output frame kind.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OutputFrameKind<O> {
    /// Complete current state for a newly attached output.
    Baseline(O),
    /// State-replacement delta for an existing output.
    Delta(O),
    /// Clear the consumer state for this output.
    Clear(ClearReason),
    /// Complete current state after an explicit discontinuity.
    Rebaseline(O, RebaselineReason),
}

/// Data-only materialized output frame returned from a transaction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputFrame<O> {
    /// Output key this frame targets.
    pub output_key: OutputKey,
    /// Scope that owns this output.
    pub scope: ScopeId,
    /// Transaction that emitted this frame.
    pub transaction_id: TransactionId,
    /// Graph revision this frame belongs to.
    pub revision: Revision,
    /// Frame payload.
    pub kind: OutputFrameKind<O>,
}
