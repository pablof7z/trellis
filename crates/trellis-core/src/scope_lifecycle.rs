use crate::{Graph, GraphResult, ResourceKey, ScopeId};

impl<C, O> Graph<C, O> {
    pub(crate) fn close_scope_direct(&mut self, scope: ScopeId) -> GraphResult<Vec<ScopeId>> {
        self.require_scope(scope)?;
        let scopes = self.scope_close_order(scope);
        for closing in &scopes {
            if let Some(scope_meta) = self.scopes.get_mut(closing) {
                scope_meta.close();
            }
            self.resource_planners
                .retain(|planner| planner.scope != *closing);
            for node in self.nodes.values_mut() {
                node.detach_scope(*closing);
            }
        }
        Ok(scopes)
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
        self.scopes
            .values()
            .filter_map(|scope_meta| {
                (scope_meta.parent() == Some(scope)).then_some(scope_meta.id())
            })
            .collect()
    }
}

enum ScopeVisitFrame {
    Enter(ScopeId),
    Exit(ScopeId),
}
