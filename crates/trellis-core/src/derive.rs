use crate::input::{StoredInput, boxed_input, downcast_input};
use crate::{DerivedNode, Graph, GraphError, GraphResult, InputNode, NodeId, NodeKind};
use std::collections::BTreeSet;
use std::sync::Arc;

type ComputeFn<C> =
    dyn for<'ctx> Fn(&DeriveContext<'ctx, C>) -> Result<Box<dyn StoredInput>, DeriveError>;

pub(crate) struct DerivedSpec<C> {
    compute: Arc<ComputeFn<C>>,
}

impl<C> Clone for DerivedSpec<C> {
    fn clone(&self) -> Self {
        Self {
            compute: Arc::clone(&self.compute),
        }
    }
}

impl<C> DerivedSpec<C> {
    pub(crate) fn new<T, F>(derive: F) -> Self
    where
        T: Clone + PartialEq + 'static,
        F: for<'ctx> Fn(&DeriveContext<'ctx, C>) -> Result<T, DeriveError> + 'static,
    {
        Self {
            compute: Arc::new(move |ctx| derive(ctx).map(boxed_input)),
        }
    }

    pub(crate) fn compute(
        &self,
        ctx: &DeriveContext<'_, C>,
    ) -> Result<Box<dyn StoredInput>, DeriveError> {
        (self.compute)(ctx)
    }
}

/// Read-only context passed to pure derived node computations.
pub struct DeriveContext<'graph, C = ()> {
    graph: &'graph Graph<C>,
    declared_dependencies: &'graph [NodeId],
}

impl<'graph, C> DeriveContext<'graph, C> {
    pub(crate) fn new(graph: &'graph Graph<C>, declared_dependencies: &'graph [NodeId]) -> Self {
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

    /// Reads a declared derived dependency.
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

    fn require_declared(&self, node: NodeId) -> Result<(), DeriveError> {
        if self.declared_dependencies.contains(&node) {
            Ok(())
        } else {
            Err(DeriveError::UndeclaredDependency(node))
        }
    }
}

/// Error returned by a pure derived node computation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeriveError {
    /// A derive function tried to read a node it did not declare.
    UndeclaredDependency(NodeId),
    /// A dependency had no committed value.
    MissingValue(NodeId),
    /// A collection dependency was read with the wrong set/map shape or value type.
    WrongCollectionType(NodeId),
    /// User-defined derivation failed.
    Message(String),
}

impl DeriveError {
    /// Creates a user-defined derive error.
    pub fn message(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }
}

impl<C> Graph<C> {
    pub(crate) fn recompute_dirty_derived(
        &mut self,
        initial_changed: &[NodeId],
    ) -> GraphResult<Vec<NodeId>> {
        let order = self.derived_topological_order()?;
        let mut changed: BTreeSet<NodeId> = initial_changed.iter().copied().collect();
        let mut changed_derived = Vec::new();

        for node in order {
            let dependencies = self
                .nodes
                .get(&node)
                .expect("derived node metadata exists")
                .dependencies()
                .clone();
            let is_dirty = changed.contains(&node)
                || dependencies
                    .as_slice()
                    .iter()
                    .any(|dependency| changed.contains(dependency));

            if !is_dirty {
                continue;
            }

            let next_value = self.compute_derived(node, dependencies.as_slice())?;
            let changed_value = self
                .derived_values
                .get(&node)
                .is_none_or(|current| !current.equals(next_value.as_ref()));

            if changed_value {
                self.derived_values.insert(node, next_value);
                changed.insert(node);
                changed_derived.push(node);
            }
        }

        Ok(changed_derived)
    }

    pub(crate) fn compute_derived(
        &self,
        node: NodeId,
        dependencies: &[NodeId],
    ) -> GraphResult<Box<dyn StoredInput>> {
        let spec = self
            .derived_specs
            .get(&node)
            .ok_or(GraphError::UnknownNode(node))?;
        let ctx = DeriveContext::new(self, dependencies);
        spec.compute(&ctx)
            .map_err(|error| GraphError::DeriveFailed(node, error))
    }

    pub(crate) fn derived_topological_order(&self) -> GraphResult<Vec<NodeId>> {
        let mut order = Vec::new();
        let mut temporary = BTreeSet::new();
        let mut permanent = BTreeSet::new();

        for node in self.nodes.keys().copied() {
            if self
                .nodes
                .get(&node)
                .is_some_and(|meta| meta.kind() == NodeKind::Derived)
            {
                self.visit_derived(node, &mut temporary, &mut permanent, &mut order)?;
            }
        }

        Ok(order)
    }

    fn visit_derived(
        &self,
        node: NodeId,
        temporary: &mut BTreeSet<NodeId>,
        permanent: &mut BTreeSet<NodeId>,
        order: &mut Vec<NodeId>,
    ) -> GraphResult<()> {
        if permanent.contains(&node) {
            return Ok(());
        }
        if !temporary.insert(node) {
            return Err(GraphError::CycleDetected(node));
        }

        let dependencies = self
            .nodes
            .get(&node)
            .expect("derived node metadata exists")
            .dependencies();
        for dependency in dependencies.as_slice() {
            if self
                .nodes
                .get(dependency)
                .is_some_and(|meta| meta.kind() == NodeKind::Derived)
            {
                self.visit_derived(*dependency, temporary, permanent, order)?;
            }
        }

        temporary.remove(&node);
        permanent.insert(node);
        order.push(node);
        Ok(())
    }
}
