use crate::collection::{CollectionContext, StoredCollection};
use crate::{Graph, GraphError, GraphResult, NodeId, NodeKind};
use std::collections::BTreeSet;

impl<C> Graph<C> {
    pub(crate) fn recompute_dirty_collections(
        &mut self,
        initial_changed: &[NodeId],
    ) -> GraphResult<Vec<NodeId>> {
        self.collection_diffs.clear();
        self.previous_collection_values = self.collection_values.clone();
        let order = self.collection_topological_order()?;
        let mut changed: BTreeSet<NodeId> = initial_changed.iter().copied().collect();
        let mut changed_collections = Vec::new();

        for node in order {
            let dependencies = self
                .nodes
                .get(&node)
                .expect("collection node metadata exists")
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

            let next = self.compute_collection(node, dependencies.as_slice())?;
            let previous = self
                .previous_collection_values
                .get(&node)
                .cloned()
                .unwrap_or_else(|| next.empty_box());
            let diff = previous.diff(next.as_ref());
            let changed_value = !previous.equals(next.as_ref());

            self.collection_diffs.insert(node, diff.clone());
            self.collection_values.insert(node, next);

            if changed_value {
                changed.insert(node);
                changed_collections.push(node);
            }
        }

        Ok(changed_collections)
    }

    pub(crate) fn compare_full_recomputed_collections(
        &self,
        full: &Graph<C>,
        order: &[NodeId],
    ) -> GraphResult<()> {
        for node in order {
            let incremental = self
                .collection_values
                .get(node)
                .ok_or(GraphError::FullRecomputeMismatch(*node))?;
            let recomputed = full
                .collection_values
                .get(node)
                .ok_or(GraphError::FullRecomputeMismatch(*node))?;
            if !incremental.equals(recomputed.as_ref()) {
                return Err(GraphError::FullRecomputeMismatch(*node));
            }
        }
        Ok(())
    }

    pub(crate) fn baseline_collection_diffs(&mut self, collections: &[NodeId]) {
        for node in collections {
            if self.collection_diffs.contains_key(node) {
                continue;
            }
            let Some(current) = self.collection_values.get(node) else {
                continue;
            };
            let previous = current.empty_box();
            let diff = previous.diff(current.as_ref());
            self.previous_collection_values.insert(*node, previous);
            self.collection_diffs.insert(*node, diff);
        }
    }

    pub(crate) fn collection_topological_order(&self) -> GraphResult<Vec<NodeId>> {
        let mut order = Vec::new();
        let mut temporary = BTreeSet::new();
        let mut permanent = BTreeSet::new();

        for node in self.nodes.keys().copied() {
            if self
                .nodes
                .get(&node)
                .is_some_and(|meta| meta.kind() == NodeKind::Collection)
            {
                self.visit_collection(node, &mut temporary, &mut permanent, &mut order)?;
            }
        }

        Ok(order)
    }

    fn compute_collection(
        &self,
        node: NodeId,
        dependencies: &[NodeId],
    ) -> GraphResult<Box<dyn StoredCollection>> {
        let spec = self
            .collection_specs
            .get(&node)
            .ok_or(GraphError::UnknownNode(node))?;
        let ctx = CollectionContext::new(self, dependencies);
        spec.compute(&ctx)
            .map_err(|error| GraphError::CollectionFailed(node, error))
    }

    fn visit_collection(
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
            .expect("collection node metadata exists")
            .dependencies();
        for dependency in dependencies.as_slice() {
            if self
                .nodes
                .get(dependency)
                .is_some_and(|meta| meta.kind() == NodeKind::Collection)
            {
                self.visit_collection(*dependency, temporary, permanent, order)?;
            }
        }

        temporary.remove(&node);
        permanent.insert(node);
        order.push(node);
        Ok(())
    }
}
