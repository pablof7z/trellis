use crate::output_payload::StoredOutput;
use crate::{
    FullRecomputeOutputMismatch, FullRecomputeResourceMismatch, Graph, GraphError, GraphResult,
    NodeId, OutputKey, ResourceKey, ScopeId,
};
use std::collections::{BTreeMap, BTreeSet};

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

impl<C> Graph<C> {
    /// Recomputes supported graph state from canonical inputs and compares it.
    pub fn full_recompute(&self) -> GraphResult<FullRecomputeCheck> {
        self.full_recompute_check()
    }

    /// Asserts that incremental state equals a supported full recompute.
    pub fn assert_incremental_equals_full(&self) -> GraphResult<FullRecomputeCheck> {
        self.full_recompute_check()
    }

    /// Compares committed incremental state against full recompute.
    pub fn full_recompute_check(&self) -> GraphResult<FullRecomputeCheck> {
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
                .dependencies();
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
        full: &mut Graph<C>,
    ) -> GraphResult<Vec<ResourceKey>> {
        let planner_collections: Vec<NodeId> = full
            .resource_planners
            .iter()
            .map(|planner| planner.collection)
            .collect();
        full.baseline_collection_diffs(&planner_collections);
        full.produce_resource_plan(&[])?;
        if let Some(mismatch) =
            first_resource_owner_mismatch(&self.resource_owners, &full.resource_owners)
        {
            return Err(GraphError::FullRecomputeResourceMismatch(mismatch));
        }
        Ok(self.resource_owners.keys().cloned().collect())
    }

    fn compare_full_recomputed_outputs(
        &self,
        full: &mut Graph<C>,
        all_nodes: &[NodeId],
    ) -> GraphResult<Vec<OutputKey>> {
        full.produce_output_frames(
            all_nodes,
            &[],
            &BTreeMap::new(),
            self.next_transaction_id,
            self.revision,
        )?;
        if let Some(mismatch) =
            first_output_value_mismatch(&self.output_values, &full.output_values)
        {
            return Err(GraphError::FullRecomputeOutputMismatch(mismatch));
        }
        Ok(self.output_values.keys().copied().collect())
    }
}

fn first_resource_owner_mismatch(
    incremental: &BTreeMap<ResourceKey, BTreeSet<ScopeId>>,
    recomputed: &BTreeMap<ResourceKey, BTreeSet<ScopeId>>,
) -> Option<FullRecomputeResourceMismatch> {
    let keys: BTreeSet<ResourceKey> = incremental
        .keys()
        .chain(recomputed.keys())
        .cloned()
        .collect();
    for key in keys {
        let incremental_owners = owner_vec(incremental.get(&key));
        let recomputed_owners = owner_vec(recomputed.get(&key));
        if incremental_owners != recomputed_owners {
            return Some(FullRecomputeResourceMismatch {
                key,
                incremental_owners,
                recomputed_owners,
            });
        }
    }
    None
}

fn owner_vec(owners: Option<&BTreeSet<ScopeId>>) -> Vec<ScopeId> {
    owners
        .into_iter()
        .flat_map(|owners| owners.iter().copied())
        .collect()
}

fn first_output_value_mismatch(
    incremental: &BTreeMap<OutputKey, Box<dyn StoredOutput>>,
    recomputed: &BTreeMap<OutputKey, Box<dyn StoredOutput>>,
) -> Option<FullRecomputeOutputMismatch> {
    let keys: BTreeSet<OutputKey> = incremental
        .keys()
        .chain(recomputed.keys())
        .copied()
        .collect();
    for key in keys {
        let incremental_value = incremental.get(&key);
        let recomputed_value = recomputed.get(&key);
        let matches = match (incremental_value, recomputed_value) {
            (Some(incremental), Some(recomputed)) => incremental.equals(recomputed.as_ref()),
            (None, None) => true,
            _ => false,
        };
        if !matches {
            return Some(FullRecomputeOutputMismatch {
                key,
                incremental_present: incremental_value.is_some(),
                recomputed_present: recomputed_value.is_some(),
            });
        }
    }
    None
}
