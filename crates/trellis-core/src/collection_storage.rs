use crate::collection_diff::{MapDiff, SetDiff};
use crate::{CollectionDiffTrace, NodeId};
use core::any::Any;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) trait StoredCollection: Any + Send + Sync {
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

pub(crate) trait StoredDiff: Any + Send + Sync {
    fn clone_box(&self) -> Box<dyn StoredDiff>;
    fn trace(&self, node: NodeId) -> CollectionDiffTrace;
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
    K: Clone + Ord + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + 'static,
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
    K: Clone + Ord + Send + Sync + 'static,
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
    K: Clone + Ord + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + 'static,
{
    Box::new(MapCollection { value })
}

pub(crate) fn boxed_set<K>(value: BTreeSet<K>) -> Box<dyn StoredCollection>
where
    K: Clone + Ord + Send + Sync + 'static,
{
    Box::new(SetCollection { value })
}

pub(crate) fn downcast_map<K, V>(value: &dyn StoredCollection) -> Option<&BTreeMap<K, V>>
where
    K: Clone + Ord + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + 'static,
{
    value
        .as_any()
        .downcast_ref::<MapCollection<K, V>>()
        .map(|collection| &collection.value)
}

pub(crate) fn downcast_set<K>(value: &dyn StoredCollection) -> Option<&BTreeSet<K>>
where
    K: Clone + Ord + Send + Sync + 'static,
{
    value
        .as_any()
        .downcast_ref::<SetCollection<K>>()
        .map(|collection| &collection.value)
}

pub(crate) fn downcast_map_diff<K, V>(value: &dyn StoredDiff) -> Option<&MapDiff<K, V>>
where
    K: Clone + Ord + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + 'static,
{
    value.as_any().downcast_ref::<MapDiff<K, V>>()
}

pub(crate) fn downcast_set_diff<K>(value: &dyn StoredDiff) -> Option<&SetDiff<K>>
where
    K: Clone + Ord + Send + Sync + 'static,
{
    value.as_any().downcast_ref::<SetDiff<K>>()
}
