use crate::collection::{
    CollectionContext, CollectionSpec, MapCollectionShape, SetCollectionShape,
};
use crate::input::value_type;
use crate::{CollectionNode, DependencyList, Graph, GraphResult, NodeId, NodeKind, NodeMeta};
use std::collections::{BTreeMap, BTreeSet};

impl<C: 'static> Graph<C> {
    pub(crate) fn collection_map_direct<K, V>(
        &mut self,
        id: NodeId,
        debug_name: impl Into<String>,
        dependencies: DependencyList,
        derive: impl for<'ctx> Fn(
            &CollectionContext<'ctx, C>,
        ) -> Result<BTreeMap<K, V>, crate::DeriveError>
        + Send
        + Sync
        + 'static,
    ) -> GraphResult<CollectionNode<K, V>>
    where
        K: Clone + Ord + Send + Sync + 'static,
        V: Clone + PartialEq + Send + Sync + 'static,
    {
        self.validate_dependencies(id, &dependencies)?;
        let meta = NodeMeta::new(
            id,
            NodeKind::Collection,
            debug_name,
            dependencies,
            self.revision,
            Some(value_type::<MapCollectionShape<K, V>>()),
        );
        self.invalidate_topology_cache();
        self.nodes.insert(id, meta);
        self.collection_specs
            .insert(id, CollectionSpec::<C>::map(derive));
        Ok(CollectionNode::new(id))
    }

    pub(crate) fn collection_set_direct<K>(
        &mut self,
        id: NodeId,
        debug_name: impl Into<String>,
        dependencies: DependencyList,
        derive: impl for<'ctx> Fn(
            &CollectionContext<'ctx, C>,
        ) -> Result<BTreeSet<K>, crate::DeriveError>
        + Send
        + Sync
        + 'static,
    ) -> GraphResult<CollectionNode<K, ()>>
    where
        K: Clone + Ord + Send + Sync + 'static,
    {
        self.validate_dependencies(id, &dependencies)?;
        let meta = NodeMeta::new(
            id,
            NodeKind::Collection,
            debug_name,
            dependencies,
            self.revision,
            Some(value_type::<SetCollectionShape<K>>()),
        );
        self.invalidate_topology_cache();
        self.nodes.insert(id, meta);
        self.collection_specs
            .insert(id, CollectionSpec::<C>::set(derive));
        Ok(CollectionNode::new(id))
    }
}
