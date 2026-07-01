use crate::{GraphError, GraphResult, NodeId};
use std::collections::BTreeSet;

/// Deterministic list of explicit node dependencies.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DependencyList {
    nodes: Vec<NodeId>,
}

impl DependencyList {
    /// Creates a dependency list, rejecting duplicate node ids.
    pub fn new(nodes: impl IntoIterator<Item = NodeId>) -> GraphResult<Self> {
        let mut seen = BTreeSet::new();
        let mut ordered = Vec::new();

        for node in nodes {
            if !seen.insert(node) {
                return Err(GraphError::DuplicateDependency(node));
            }
            ordered.push(node);
        }

        Ok(Self { nodes: ordered })
    }

    /// Creates an empty dependency list.
    pub fn empty() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Returns dependencies in declared order.
    pub fn as_slice(&self) -> &[NodeId] {
        &self.nodes
    }

    /// Returns true when the list contains no dependencies.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

impl IntoIterator for DependencyList {
    type Item = NodeId;
    type IntoIter = std::vec::IntoIter<NodeId>;

    fn into_iter(self) -> Self::IntoIter {
        self.nodes.into_iter()
    }
}
