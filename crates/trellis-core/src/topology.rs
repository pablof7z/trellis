use crate::{Graph, GraphError, GraphResult, NodeId, NodeKind};
use std::collections::BTreeSet;

impl<C, O> Graph<C, O> {
    pub(crate) fn topological_order_for_kind(&self, kind: NodeKind) -> GraphResult<Vec<NodeId>> {
        let mut order = Vec::new();
        let mut temporary = BTreeSet::new();
        let mut permanent = BTreeSet::new();

        for node in self.nodes.keys().copied() {
            if self.node_is_kind(node, kind) {
                self.visit_kind_iterative(node, kind, &mut temporary, &mut permanent, &mut order)?;
            }
        }

        Ok(order)
    }

    fn visit_kind_iterative(
        &self,
        root: NodeId,
        kind: NodeKind,
        temporary: &mut BTreeSet<NodeId>,
        permanent: &mut BTreeSet<NodeId>,
        order: &mut Vec<NodeId>,
    ) -> GraphResult<()> {
        if permanent.contains(&root) {
            return Ok(());
        }

        let mut stack = vec![VisitFrame::Enter(root)];
        while let Some(frame) = stack.pop() {
            match frame {
                VisitFrame::Exit(node) => {
                    temporary.remove(&node);
                    permanent.insert(node);
                    order.push(node);
                }
                VisitFrame::Enter(node) => {
                    if permanent.contains(&node) {
                        continue;
                    }
                    if !temporary.insert(node) {
                        return Err(GraphError::CycleDetected(node));
                    }

                    stack.push(VisitFrame::Exit(node));
                    let dependencies = self
                        .nodes
                        .get(&node)
                        .expect("node metadata exists for topological traversal")
                        .dependencies();
                    for dependency in dependencies.as_slice().iter().rev() {
                        if !self.node_is_kind(*dependency, kind) {
                            continue;
                        }
                        if temporary.contains(dependency) {
                            return Err(GraphError::CycleDetected(*dependency));
                        }
                        if !permanent.contains(dependency) {
                            stack.push(VisitFrame::Enter(*dependency));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn node_is_kind(&self, node: NodeId, kind: NodeKind) -> bool {
        self.nodes
            .get(&node)
            .is_some_and(|meta| meta.kind() == kind)
    }
}

enum VisitFrame {
    Enter(NodeId),
    Exit(NodeId),
}
