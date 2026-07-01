use crate::{CollectionDiffKind, CollectionDiffTrace, NodeId, collection::StoredDiff};
use core::any::Any;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
/// A value that was added to a structural diff.
pub struct Added<T> {
    /// Added value.
    pub value: T,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// A value that was removed from a structural diff.
pub struct Removed<T> {
    /// Removed value.
    pub value: T,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// A value that was unchanged in a structural diff.
pub struct Unchanged<T> {
    /// Unchanged value.
    pub value: T,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// A map entry that changed value without changing key identity.
pub struct Updated<K, V> {
    /// Updated key.
    pub key: K,
    /// Previously committed value.
    pub previous: V,
    /// Newly committed value.
    pub current: V,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Deterministic structural diff for a set collection.
pub struct SetDiff<K> {
    /// Members added in stable key order.
    pub added: Vec<Added<K>>,
    /// Members removed in stable key order.
    pub removed: Vec<Removed<K>>,
    /// Members retained in stable key order.
    pub unchanged: Vec<Unchanged<K>>,
}

impl<K> SetDiff<K>
where
    K: Clone + Ord,
{
    pub(crate) fn between(previous: &BTreeSet<K>, current: &BTreeSet<K>) -> Self {
        Self {
            added: current
                .difference(previous)
                .cloned()
                .map(|value| Added { value })
                .collect(),
            removed: previous
                .difference(current)
                .cloned()
                .map(|value| Removed { value })
                .collect(),
            unchanged: previous
                .intersection(current)
                .cloned()
                .map(|value| Unchanged { value })
                .collect(),
        }
    }

    /// Returns true when the diff has no structural changes.
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty()
    }
}

impl<K> StoredDiff for SetDiff<K>
where
    K: Clone + Ord + Send + Sync + 'static,
{
    fn clone_box(&self) -> Box<dyn StoredDiff> {
        Box::new(self.clone())
    }

    fn trace(&self, node: NodeId) -> CollectionDiffTrace {
        CollectionDiffTrace {
            node,
            kind: CollectionDiffKind::Set,
            added: self.added.len(),
            removed: self.removed.len(),
            updated: 0,
            unchanged: self.unchanged.len(),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Deterministic structural diff for a map collection.
pub struct MapDiff<K, V> {
    /// Entries added in stable key order.
    pub added: Vec<Added<(K, V)>>,
    /// Entries removed in stable key order.
    pub removed: Vec<Removed<(K, V)>>,
    /// Entries updated in stable key order.
    pub updated: Vec<Updated<K, V>>,
    /// Entries retained in stable key order.
    pub unchanged: Vec<Unchanged<(K, V)>>,
}

impl<K, V> MapDiff<K, V>
where
    K: Clone + Ord,
    V: Clone + PartialEq,
{
    pub(crate) fn between(previous: &BTreeMap<K, V>, current: &BTreeMap<K, V>) -> Self {
        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut updated = Vec::new();
        let mut unchanged = Vec::new();

        for (key, previous_value) in previous {
            match current.get(key) {
                Some(current_value) if current_value == previous_value => {
                    unchanged.push(Unchanged {
                        value: (key.clone(), current_value.clone()),
                    });
                }
                Some(current_value) => updated.push(Updated {
                    key: key.clone(),
                    previous: previous_value.clone(),
                    current: current_value.clone(),
                }),
                None => removed.push(Removed {
                    value: (key.clone(), previous_value.clone()),
                }),
            }
        }

        for (key, value) in current {
            if !previous.contains_key(key) {
                added.push(Added {
                    value: (key.clone(), value.clone()),
                });
            }
        }

        Self {
            added,
            removed,
            updated,
            unchanged,
        }
    }

    /// Returns true when the diff has no structural changes.
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.updated.is_empty()
    }
}

impl<K, V> StoredDiff for MapDiff<K, V>
where
    K: Clone + Ord + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + 'static,
{
    fn clone_box(&self) -> Box<dyn StoredDiff> {
        Box::new(self.clone())
    }

    fn trace(&self, node: NodeId) -> CollectionDiffTrace {
        CollectionDiffTrace {
            node,
            kind: CollectionDiffKind::Map,
            added: self.added.len(),
            removed: self.removed.len(),
            updated: self.updated.len(),
            unchanged: self.unchanged.len(),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
