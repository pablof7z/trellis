use crate::{DependencyList, Graph, GraphError, GraphResult, NodeId, NodeKind};
use std::collections::BTreeSet;

impl<C, O> Graph<C, O> {
    pub(crate) fn validate_dependencies(
        &self,
        node_id: NodeId,
        dependencies: &DependencyList,
    ) -> GraphResult<()> {
        for dependency in dependencies.as_slice() {
            if *dependency == node_id {
                return Err(GraphError::SelfDependency(node_id));
            }
            if !self.nodes.contains_key(dependency) {
                return Err(GraphError::UnknownNode(*dependency));
            }
            if self.depends_on(*dependency, node_id) {
                return Err(GraphError::CycleDetected(node_id));
            }
        }
        Ok(())
    }

    pub(crate) fn validate_output_dependencies(
        &self,
        dependencies: &DependencyList,
    ) -> GraphResult<()> {
        for dependency in dependencies.as_slice() {
            if !self.nodes.contains_key(dependency) {
                return Err(GraphError::UnknownNode(*dependency));
            }
        }
        Ok(())
    }

    pub(crate) fn reject_collection_dependencies(
        &self,
        dependencies: &DependencyList,
    ) -> GraphResult<()> {
        for dependency in dependencies.as_slice() {
            if self
                .nodes
                .get(dependency)
                .is_some_and(|meta| meta.kind() == NodeKind::Collection)
            {
                return Err(GraphError::CollectionDependencyNotAllowed(*dependency));
            }
        }
        Ok(())
    }

    fn depends_on(&self, start: NodeId, target: NodeId) -> bool {
        let mut stack = vec![start];
        let mut visited = BTreeSet::new();

        while let Some(node) = stack.pop() {
            if node == target {
                return true;
            }
            if !visited.insert(node) {
                continue;
            }
            let Some(meta) = self.nodes.get(&node) else {
                continue;
            };
            for dependency in meta.dependencies().as_slice() {
                if *dependency == target {
                    return true;
                }
                if !visited.contains(dependency) {
                    stack.push(*dependency);
                }
            }
        }

        false
    }
}
