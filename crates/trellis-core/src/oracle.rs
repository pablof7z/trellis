use crate::{Graph, GraphError, GraphResult, NodeId};

/// Result of comparing incremental graph state against full recompute.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FullRecomputeCheck {
    /// Derived nodes checked in deterministic topological order.
    pub checked_derived: Vec<NodeId>,
    /// Collection nodes checked in deterministic topological order.
    pub checked_collections: Vec<NodeId>,
}

impl<C> Graph<C> {
    /// Compares committed incremental state against full recompute.
    pub fn full_recompute_check(&self) -> GraphResult<FullRecomputeCheck> {
        let mut full = self.clone();
        full.derived_values.clear();
        full.collection_values.clear();
        full.previous_collection_values.clear();
        full.collection_diffs.clear();
        let order = full.derived_topological_order()?;

        for node in &order {
            let dependencies = full
                .nodes
                .get(node)
                .expect("derived node metadata exists")
                .dependencies()
                .clone();
            let value = full.compute_derived(*node, dependencies.as_slice())?;
            full.derived_values.insert(*node, value);
        }

        for node in &order {
            let incremental = self
                .derived_values
                .get(node)
                .ok_or(GraphError::FullRecomputeMismatch(*node))?;
            let recomputed = full
                .derived_values
                .get(node)
                .ok_or(GraphError::FullRecomputeMismatch(*node))?;
            if !incremental.equals(recomputed.as_ref()) {
                return Err(GraphError::FullRecomputeMismatch(*node));
            }
        }

        let collection_order = full.collection_topological_order()?;
        let all_nodes: Vec<NodeId> = full.nodes.keys().copied().collect();
        full.recompute_dirty_collections(&all_nodes)?;
        self.compare_full_recomputed_collections(&full, &collection_order)?;

        Ok(FullRecomputeCheck {
            checked_derived: order,
            checked_collections: collection_order,
        })
    }
}
