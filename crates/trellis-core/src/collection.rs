use crate::collection_diff::{MapDiff, SetDiff};
use crate::input::downcast_input;
use crate::{CollectionNode, DeriveError, DerivedNode, Graph, InputNode, NodeId};
use core::any::Any;
use core::marker::PhantomData;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

type ComputeFn =
    dyn for<'ctx> Fn(&CollectionContext<'ctx>) -> Result<Box<dyn StoredCollection>, DeriveError>;

pub(crate) struct MapCollectionShape<K, V>(PhantomData<fn() -> (K, V)>);

pub(crate) struct SetCollectionShape<K>(PhantomData<fn() -> K>);

#[derive(Clone)]
pub(crate) struct CollectionSpec {
    compute: Arc<ComputeFn>,
}

impl CollectionSpec {
    pub(crate) fn map<K, V, F>(derive: F) -> Self
    where
        K: Clone + Ord + 'static,
        V: Clone + PartialEq + 'static,
        F: for<'ctx> Fn(&CollectionContext<'ctx>) -> Result<BTreeMap<K, V>, DeriveError> + 'static,
    {
        Self {
            compute: Arc::new(move |ctx| derive(ctx).map(boxed_map)),
        }
    }

    pub(crate) fn set<K, F>(derive: F) -> Self
    where
        K: Clone + Ord + 'static,
        F: for<'ctx> Fn(&CollectionContext<'ctx>) -> Result<BTreeSet<K>, DeriveError> + 'static,
    {
        Self {
            compute: Arc::new(move |ctx| derive(ctx).map(boxed_set)),
        }
    }

    pub(crate) fn compute(
        &self,
        ctx: &CollectionContext<'_>,
    ) -> Result<Box<dyn StoredCollection>, DeriveError> {
        (self.compute)(ctx)
    }
}

/// Read-only context passed to pure collection node computations.
pub struct CollectionContext<'graph> {
    graph: &'graph Graph,
    declared_dependencies: &'graph [NodeId],
}

impl<'graph> CollectionContext<'graph> {
    pub(crate) fn new(graph: &'graph Graph, declared_dependencies: &'graph [NodeId]) -> Self {
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

pub(crate) trait StoredCollection: Any {
    fn clone_box(&self) -> Box<dyn StoredCollection>;
    fn empty_box(&self) -> Box<dyn StoredCollection>;
    fn equals(&self, other: &dyn StoredCollection) -> bool;
    fn diff(&self, next: &dyn StoredCollection) -> Box<dyn StoredDiff>;
    fn as_any(&self) -> &dyn Any;
}

impl Clone for Box<dyn StoredCollection> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

pub(crate) trait StoredDiff: Any {
    fn clone_box(&self) -> Box<dyn StoredDiff>;
    fn as_any(&self) -> &dyn Any;
}

impl Clone for Box<dyn StoredDiff> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

#[derive(Clone)]
struct MapCollection<K, V> {
    value: BTreeMap<K, V>,
}

#[derive(Clone)]
struct SetCollection<K> {
    value: BTreeSet<K>,
}

impl<K, V> StoredCollection for MapCollection<K, V>
where
    K: Clone + Ord + 'static,
    V: Clone + PartialEq + 'static,
{
    fn clone_box(&self) -> Box<dyn StoredCollection> {
        Box::new(self.clone())
    }

    fn empty_box(&self) -> Box<dyn StoredCollection> {
        boxed_map(BTreeMap::<K, V>::new())
    }

    fn equals(&self, other: &dyn StoredCollection) -> bool {
        other
            .as_any()
            .downcast_ref::<Self>()
            .is_some_and(|other| self.value == other.value)
    }

    fn diff(&self, next: &dyn StoredCollection) -> Box<dyn StoredDiff> {
        let next = next
            .as_any()
            .downcast_ref::<Self>()
            .expect("collection type stays stable");
        Box::new(MapDiff::between(&self.value, &next.value))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<K> StoredCollection for SetCollection<K>
where
    K: Clone + Ord + 'static,
{
    fn clone_box(&self) -> Box<dyn StoredCollection> {
        Box::new(self.clone())
    }

    fn empty_box(&self) -> Box<dyn StoredCollection> {
        boxed_set(BTreeSet::<K>::new())
    }

    fn equals(&self, other: &dyn StoredCollection) -> bool {
        other
            .as_any()
            .downcast_ref::<Self>()
            .is_some_and(|other| self.value == other.value)
    }

    fn diff(&self, next: &dyn StoredCollection) -> Box<dyn StoredDiff> {
        let next = next
            .as_any()
            .downcast_ref::<Self>()
            .expect("collection type stays stable");
        Box::new(SetDiff::between(&self.value, &next.value))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub(crate) fn boxed_map<K, V>(value: BTreeMap<K, V>) -> Box<dyn StoredCollection>
where
    K: Clone + Ord + 'static,
    V: Clone + PartialEq + 'static,
{
    Box::new(MapCollection { value })
}

pub(crate) fn boxed_set<K>(value: BTreeSet<K>) -> Box<dyn StoredCollection>
where
    K: Clone + Ord + 'static,
{
    Box::new(SetCollection { value })
}

pub(crate) fn downcast_map<K, V>(value: &dyn StoredCollection) -> Option<&BTreeMap<K, V>>
where
    K: Clone + Ord + 'static,
    V: Clone + PartialEq + 'static,
{
    value
        .as_any()
        .downcast_ref::<MapCollection<K, V>>()
        .map(|collection| &collection.value)
}

pub(crate) fn downcast_set<K>(value: &dyn StoredCollection) -> Option<&BTreeSet<K>>
where
    K: Clone + Ord + 'static,
{
    value
        .as_any()
        .downcast_ref::<SetCollection<K>>()
        .map(|collection| &collection.value)
}

pub(crate) fn downcast_map_diff<K, V>(value: &dyn StoredDiff) -> Option<&MapDiff<K, V>>
where
    K: Clone + Ord + 'static,
    V: Clone + PartialEq + 'static,
{
    value.as_any().downcast_ref::<MapDiff<K, V>>()
}

pub(crate) fn downcast_set_diff<K>(value: &dyn StoredDiff) -> Option<&SetDiff<K>>
where
    K: Clone + Ord + 'static,
{
    value.as_any().downcast_ref::<SetDiff<K>>()
}
