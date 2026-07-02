pub(crate) use crate::collection_storage::{
    StoredCollection, StoredDiff, boxed_map, boxed_set, downcast_map, downcast_map_diff,
    downcast_set, downcast_set_diff,
};
use crate::input::downcast_input;
use crate::{CollectionNode, DeriveError, DerivedNode, Graph, InputNode, NodeId};
use core::marker::PhantomData;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

type CollectionComputeResult = Result<Box<dyn StoredCollection>, DeriveError>;
type ComputeFn<C> =
    dyn for<'ctx> Fn(&CollectionContext<'ctx, C>) -> CollectionComputeResult + Send + Sync;

pub(crate) struct MapCollectionShape<K, V>(PhantomData<fn() -> (K, V)>);
pub(crate) struct SetCollectionShape<K>(PhantomData<fn() -> K>);
pub(crate) struct CollectionSpec<C> {
    compute: Arc<ComputeFn<C>>,
}

impl<C> Clone for CollectionSpec<C> {
    fn clone(&self) -> Self {
        Self {
            compute: Arc::clone(&self.compute),
        }
    }
}
impl<C> CollectionSpec<C> {
    pub(crate) fn map<K, V, F>(derive: F) -> Self
    where
        K: Clone + Ord + Send + Sync + 'static,
        V: Clone + PartialEq + Send + Sync + 'static,
        F: for<'ctx> Fn(&CollectionContext<'ctx, C>) -> Result<BTreeMap<K, V>, DeriveError>
            + Send
            + Sync
            + 'static,
    {
        Self {
            compute: Arc::new(move |ctx| derive(ctx).map(boxed_map)),
        }
    }

    pub(crate) fn set<K, F>(derive: F) -> Self
    where
        K: Clone + Ord + Send + Sync + 'static,
        F: for<'ctx> Fn(&CollectionContext<'ctx, C>) -> Result<BTreeSet<K>, DeriveError>
            + Send
            + Sync
            + 'static,
    {
        Self {
            compute: Arc::new(move |ctx| derive(ctx).map(boxed_set)),
        }
    }

    pub(crate) fn compute(
        &self,
        ctx: &CollectionContext<'_, C>,
    ) -> Result<Box<dyn StoredCollection>, DeriveError> {
        (self.compute)(ctx)
    }
}
/// Read-only context passed to pure collection node computations.
pub struct CollectionContext<'graph, C = ()> {
    graph: &'graph Graph<C>,
    declared_dependencies: &'graph [NodeId],
}

impl<'graph, C> CollectionContext<'graph, C> {
    pub(crate) fn new(graph: &'graph Graph<C>, declared_dependencies: &'graph [NodeId]) -> Self {
        Self {
            graph,
            declared_dependencies,
        }
    }

    /// Reads a declared input dependency.
    pub fn input<T>(&self, input: InputNode<T>) -> Result<&'graph T, DeriveError>
    where
        T: Clone + PartialEq + Send + Sync + 'static,
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
        T: Clone + PartialEq + Send + Sync + 'static,
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
        K: Clone + Ord + Send + Sync + 'static,
        V: Clone + PartialEq + Send + Sync + 'static,
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
        K: Clone + Ord + Send + Sync + 'static,
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
