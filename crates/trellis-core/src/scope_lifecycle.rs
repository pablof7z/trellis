use crate::{Graph, GraphResult, ResourceKey, ScopeId};
use std::collections::BTreeSet;

impl<C> Graph<C> {
    pub(crate) fn close_scope_direct(&mut self, scope: ScopeId) -> GraphResult<Vec<ScopeId>> {
        self.require_scope(scope)?;
        let scopes = self.scope_close_order(scope);
        for closing in &scopes {
            if let Some(scope_meta) = self.scopes.get_mut(closing) {
                scope_meta.close();
            }
            self.resource_planners
                .retain(|planner| planner.scope != *closing);
        }
        Ok(scopes)
    }

    pub(crate) fn reclaim_closed_scopes(&mut self, closed_scopes: &[ScopeId]) {
        let reclaimed_nodes = self.reclaim_closed_scope_nodes(closed_scopes);
        self.reclaim_closed_scope_metadata(closed_scopes);
        if !reclaimed_nodes.is_empty() || !closed_scopes.is_empty() {
            self.invalidate_topology_cache();
        }
    }

    /// Returns child scopes in stable id order.
    pub fn child_scopes(&self, scope: ScopeId) -> GraphResult<Vec<ScopeId>> {
        self.require_scope(scope)?;
        Ok(self.child_scopes_unchecked(scope))
    }

    /// Returns resources whose owner set is empty or contains no live scope.
    pub fn orphan_resources(&self) -> Vec<ResourceKey> {
        self.resource_owners
            .iter()
            .filter_map(|(key, owners)| {
                let has_live_owner = owners
                    .iter()
                    .any(|scope| self.scopes.get(scope).is_some_and(|meta| !meta.is_closed()));
                (!has_live_owner).then(|| key.clone())
            })
            .collect()
    }

    fn scope_close_order(&self, scope: ScopeId) -> Vec<ScopeId> {
        let mut scopes = Vec::new();
        let mut stack = vec![ScopeVisitFrame::Enter(scope)];

        while let Some(frame) = stack.pop() {
            match frame {
                ScopeVisitFrame::Exit(scope) => {
                    if self
                        .scopes
                        .get(&scope)
                        .is_some_and(|scope_meta| !scope_meta.is_closed())
                    {
                        scopes.push(scope);
                    }
                }
                ScopeVisitFrame::Enter(scope) => {
                    stack.push(ScopeVisitFrame::Exit(scope));
                    let mut children = self.child_scopes_unchecked(scope);
                    children.reverse();
                    for child in children {
                        stack.push(ScopeVisitFrame::Enter(child));
                    }
                }
            }
        }

        scopes
    }

    fn child_scopes_unchecked(&self, scope: ScopeId) -> Vec<ScopeId> {
        self.scope_children
            .get(&scope)
            .map(|children| children.iter().copied().collect())
            .unwrap_or_default()
    }

    fn reclaim_closed_scope_nodes(&mut self, closed_scopes: &[ScopeId]) -> Vec<crate::NodeId> {
        let closed_scopes = closed_scopes.iter().copied().collect::<BTreeSet<_>>();
        let nodes = self
            .nodes
            .values()
            .filter_map(|node| {
                node.owning_scope()
                    .filter(|scope| closed_scopes.contains(scope))
                    .map(|_| node.id())
            })
            .collect::<Vec<_>>();
        for node in &nodes {
            self.remove_node_storage(*node);
        }
        nodes
    }

    fn remove_node_storage(&mut self, node: crate::NodeId) {
        self.nodes.remove(&node);
        self.input_values.remove(&node);
        self.derived_specs.remove(&node);
        self.derived_values.remove(&node);
        self.collection_specs.remove(&node);
        self.collection_values.remove(&node);
        self.previous_collection_values.remove(&node);
        self.collection_diffs.remove(&node);
        self.resource_planners
            .retain(|planner| planner.collection != node);
        self.audit.node_changes.remove(&node);
    }

    fn reclaim_closed_scope_metadata(&mut self, closed_scopes: &[ScopeId]) {
        for scope in closed_scopes {
            if let Some(scope_meta) = self.scopes.remove(scope)
                && let Some(parent) = scope_meta.parent()
            {
                let remove_parent = if let Some(children) = self.scope_children.get_mut(&parent) {
                    children.remove(scope);
                    children.is_empty()
                } else {
                    false
                };
                if remove_parent {
                    self.scope_children.remove(&parent);
                }
            }
            self.scope_children.remove(scope);
        }
    }
}

enum ScopeVisitFrame {
    Enter(ScopeId),
    Exit(ScopeId),
}
