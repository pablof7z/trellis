use crate::{Graph, GraphError, GraphResult, NodeId, OutputKey, ResourceKey};
use std::collections::BTreeMap;

/// Result of comparing incremental graph state against full recompute.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FullRecomputeCheck {
    /// Derived nodes checked in deterministic topological order.
    pub checked_derived: Vec<NodeId>,
    /// Collection nodes checked in deterministic topological order.
    pub checked_collections: Vec<NodeId>,
    /// Desired resource keys whose owner sets were checked.
    pub checked_resources: Vec<ResourceKey>,
    /// Materialized outputs whose current values were checked.
    pub checked_outputs: Vec<OutputKey>,
}

impl<C, O: Clone> Graph<C, O> {
    /// Recomputes supported graph state from canonical inputs and compares it.
    pub fn full_recompute(&self) -> GraphResult<FullRecomputeCheck>
    where
        O: PartialEq,
    {
        self.full_recompute_check()
    }

    /// Asserts that incremental state equals a supported full recompute.
    pub fn assert_incremental_equals_full(&self) -> GraphResult<FullRecomputeCheck>
    where
        O: PartialEq,
    {
        self.full_recompute_check()
    }

    /// Compares committed incremental state against full recompute.
    pub fn full_recompute_check(&self) -> GraphResult<FullRecomputeCheck>
    where
        O: PartialEq,
    {
        let mut full = self.clone();
        full.derived_values.clear();
        full.collection_values.clear();
        full.previous_collection_values.clear();
        full.collection_diffs.clear();
        full.resource_owners.clear();
        full.output_values.clear();
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
        let checked_resources = self.compare_full_recomputed_resources(&mut full)?;
        let checked_outputs = self.compare_full_recomputed_outputs(&mut full, &all_nodes)?;

        Ok(FullRecomputeCheck {
            checked_derived: order,
            checked_collections: collection_order,
            checked_resources,
            checked_outputs,
        })
    }

    fn compare_full_recomputed_resources(
        &self,
        full: &mut Graph<C, O>,
    ) -> GraphResult<Vec<ResourceKey>> {
        let planner_collections: Vec<NodeId> = full
            .resource_planners
            .iter()
            .map(|planner| planner.collection)
            .collect();
        full.baseline_collection_diffs(&planner_collections);
        full.produce_resource_plan(&[])?;
        if self.resource_owners != full.resource_owners {
            let node = planner_collections
                .into_iter()
                .next()
                .unwrap_or_else(|| NodeId::from_index(1));
            return Err(GraphError::FullRecomputeMismatch(node));
        }
        Ok(self.resource_owners.keys().cloned().collect())
    }

    fn compare_full_recomputed_outputs(
        &self,
        full: &mut Graph<C, O>,
        all_nodes: &[NodeId],
    ) -> GraphResult<Vec<OutputKey>>
    where
        O: PartialEq,
    {
        full.produce_output_frames(
            all_nodes,
            &[],
            &BTreeMap::new(),
            self.next_transaction_id,
            self.revision,
        )?;
        if self.output_values != full.output_values {
            let node = self
                .outputs
                .values()
                .flat_map(|meta| meta.dependencies().as_slice().iter().copied())
                .next()
                .unwrap_or_else(|| NodeId::from_index(1));
            return Err(GraphError::FullRecomputeMismatch(node));
        }
        Ok(self.output_values.keys().copied().collect())
    }
}
