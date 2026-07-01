use crate::collection::{
    MapCollectionShape, SetCollectionShape, downcast_map, downcast_map_diff, downcast_set,
    downcast_set_diff,
};
use crate::input::{downcast_input, value_type};
use crate::{
    CollectionNode, DerivedNode, Graph, GraphError, GraphResult, InputNode, MapDiff, NodeId,
    NodeKind, SetDiff,
};
use std::collections::{BTreeMap, BTreeSet};

impl<C, O> Graph<C, O> {
    /// Returns the committed value for a typed input node.
    pub fn input_value<T>(&self, input: InputNode<T>) -> GraphResult<Option<&T>>
    where
        T: Clone + PartialEq + 'static,
    {
        self.input_value_by_id(input.id())
    }

    /// Returns the committed value for an input node id.
    pub fn input_value_by_id<T>(&self, node: NodeId) -> GraphResult<Option<&T>>
    where
        T: Clone + PartialEq + 'static,
    {
        self.validate_input_write::<T>(node)?;
        Ok(self
            .input_values
            .get(&node)
            .and_then(|value| downcast_input::<T>(value.as_ref())))
    }

    /// Returns the committed value for a typed derived node.
    pub fn derived_value<T>(&self, derived: DerivedNode<T>) -> GraphResult<Option<&T>>
    where
        T: Clone + PartialEq + 'static,
    {
        self.derived_value_by_id(derived.id())
    }

    /// Returns the committed value for a derived node id.
    pub fn derived_value_by_id<T>(&self, node: NodeId) -> GraphResult<Option<&T>>
    where
        T: Clone + PartialEq + 'static,
    {
        self.validate_derived_write::<T>(node)?;
        Ok(self
            .derived_values
            .get(&node)
            .and_then(|value| downcast_input::<T>(value.as_ref())))
    }

    /// Returns the committed map for a collection node.
    pub fn map_collection<K, V>(
        &self,
        collection: CollectionNode<K, V>,
    ) -> GraphResult<Option<&BTreeMap<K, V>>>
    where
        K: Clone + Ord + 'static,
        V: Clone + PartialEq + 'static,
    {
        self.map_collection_by_id(collection.id())
    }

    /// Returns the committed set for a collection node.
    pub fn set_collection<K>(
        &self,
        collection: CollectionNode<K, ()>,
    ) -> GraphResult<Option<&BTreeSet<K>>>
    where
        K: Clone + Ord + 'static,
    {
        self.set_collection_by_id(collection.id())
    }

    /// Returns the last committed map diff for the current transaction.
    pub fn map_diff<K, V>(
        &self,
        collection: CollectionNode<K, V>,
    ) -> GraphResult<Option<&MapDiff<K, V>>>
    where
        K: Clone + Ord + 'static,
        V: Clone + PartialEq + 'static,
    {
        self.map_diff_by_id(collection.id())
    }

    /// Returns the last committed set diff for the current transaction.
    pub fn set_diff<K>(&self, collection: CollectionNode<K, ()>) -> GraphResult<Option<&SetDiff<K>>>
    where
        K: Clone + Ord + 'static,
    {
        self.set_diff_by_id(collection.id())
    }

    pub(crate) fn validate_input_write<T>(&self, node: NodeId) -> GraphResult<()>
    where
        T: 'static,
    {
        let meta = self.nodes.get(&node).ok_or(GraphError::UnknownNode(node))?;
        if meta.kind() != NodeKind::Input {
            return Err(GraphError::NotInputNode(node));
        }
        if meta.value_type() != Some(value_type::<T>()) {
            return Err(GraphError::WrongInputType(node));
        }
        Ok(())
    }

    pub(crate) fn validate_derived_write<T>(&self, node: NodeId) -> GraphResult<()>
    where
        T: 'static,
    {
        let meta = self.nodes.get(&node).ok_or(GraphError::UnknownNode(node))?;
        if meta.kind() != NodeKind::Derived {
            return Err(GraphError::NotDerivedNode(node));
        }
        if meta.value_type() != Some(value_type::<T>()) {
            return Err(GraphError::WrongDerivedType(node));
        }
        Ok(())
    }

    /// Returns the committed map for a collection node id.
    pub fn map_collection_by_id<K, V>(&self, node: NodeId) -> GraphResult<Option<&BTreeMap<K, V>>>
    where
        K: Clone + Ord + 'static,
        V: Clone + PartialEq + 'static,
    {
        self.validate_map_collection_read::<K, V>(node)?;
        Ok(self
            .collection_values
            .get(&node)
            .and_then(|value| downcast_map::<K, V>(value.as_ref())))
    }

    /// Returns the committed set for a collection node id.
    pub fn set_collection_by_id<K>(&self, node: NodeId) -> GraphResult<Option<&BTreeSet<K>>>
    where
        K: Clone + Ord + 'static,
    {
        self.validate_set_collection_read::<K>(node)?;
        Ok(self
            .collection_values
            .get(&node)
            .and_then(|value| downcast_set::<K>(value.as_ref())))
    }

    /// Returns the current transaction's map diff for a collection node id.
    pub fn map_diff_by_id<K, V>(&self, node: NodeId) -> GraphResult<Option<&MapDiff<K, V>>>
    where
        K: Clone + Ord + 'static,
        V: Clone + PartialEq + 'static,
    {
        self.validate_map_collection_read::<K, V>(node)?;
        Ok(self
            .collection_diffs
            .get(&node)
            .and_then(|value| downcast_map_diff::<K, V>(value.as_ref())))
    }

    /// Returns the current transaction's set diff for a collection node id.
    pub fn set_diff_by_id<K>(&self, node: NodeId) -> GraphResult<Option<&SetDiff<K>>>
    where
        K: Clone + Ord + 'static,
    {
        self.validate_set_collection_read::<K>(node)?;
        Ok(self
            .collection_diffs
            .get(&node)
            .and_then(|value| downcast_set_diff::<K>(value.as_ref())))
    }

    pub(crate) fn validate_map_collection_read<K, V>(&self, node: NodeId) -> GraphResult<()>
    where
        K: 'static,
        V: 'static,
    {
        let meta = self.nodes.get(&node).ok_or(GraphError::UnknownNode(node))?;
        if meta.kind() != NodeKind::Collection {
            return Err(GraphError::NotCollectionNode(node));
        }
        if meta.value_type() != Some(value_type::<MapCollectionShape<K, V>>()) {
            return Err(GraphError::WrongCollectionType(node));
        }
        Ok(())
    }

    pub(crate) fn validate_set_collection_read<K>(&self, node: NodeId) -> GraphResult<()>
    where
        K: 'static,
    {
        let meta = self.nodes.get(&node).ok_or(GraphError::UnknownNode(node))?;
        if meta.kind() != NodeKind::Collection {
            return Err(GraphError::NotCollectionNode(node));
        }
        if meta.value_type() != Some(value_type::<SetCollectionShape<K>>()) {
            return Err(GraphError::WrongCollectionType(node));
        }
        Ok(())
    }
}
