use crate::{
    Graph, GraphError, GraphResult, ResourceCommand, ResourceCommandCause, ResourceKey,
    ResourcePlan, ScopeId,
};
use std::collections::BTreeSet;

impl<C, O> Graph<C, O> {
    pub(crate) fn produce_resource_plan(
        &mut self,
        closed_scopes: &[ScopeId],
    ) -> GraphResult<ResourcePlan<C>> {
        self.audit.pending_resource_causes.clear();
        let planners = self.resource_planners.clone();
        let mut plan = ResourcePlan::new();
        for planner in planners {
            if self.collection_diffs.contains_key(&planner.collection) {
                let planned = planner.run(self)?;
                let cause = ResourceCommandCause::Planner {
                    collection: planner.collection,
                };
                let reconciled = self.reconcile_resource_plan(planner.scope, planned, cause)?;
                plan.append(reconciled);
            }
        }

        for scope in closed_scopes {
            let close_plan = self.close_scope_resources(*scope);
            plan.append(close_plan);
        }

        Ok(plan)
    }

    fn reconcile_resource_plan(
        &mut self,
        planner_scope: ScopeId,
        plan: ResourcePlan<C>,
        cause: ResourceCommandCause,
    ) -> GraphResult<ResourcePlan<C>> {
        let mut reconciled = ResourcePlan::new();
        for command in plan.into_commands() {
            if command.scope() != planner_scope {
                return Err(GraphError::ResourceScopeMismatch(command.scope()));
            }
            self.reconcile_resource_command(command, &mut reconciled, cause)?;
        }
        Ok(reconciled)
    }

    fn reconcile_resource_command(
        &mut self,
        command: ResourceCommand<C>,
        plan: &mut ResourcePlan<C>,
        cause: ResourceCommandCause,
    ) -> GraphResult<()> {
        match command {
            ResourceCommand::Open {
                key,
                scope,
                command,
            } => self.reconcile_open(key, scope, command, plan, cause),
            ResourceCommand::Close { key, scope } => {
                self.remove_resource_owner(&key, scope, plan, cause);
                Ok(())
            }
            ResourceCommand::Replace {
                key,
                scope,
                command,
            } => {
                self.require_scope_open(scope)?;
                self.require_resource_owner(&key, scope)?;
                self.resource_owners
                    .entry(key.clone())
                    .or_default()
                    .insert(scope);
                plan.replace(key, scope, command);
                self.audit.pending_resource_causes.push(cause);
                Ok(())
            }
            ResourceCommand::Refresh {
                key,
                scope,
                command,
            } => {
                self.require_scope_open(scope)?;
                self.require_resource_owner(&key, scope)?;
                self.resource_owners
                    .entry(key.clone())
                    .or_default()
                    .insert(scope);
                plan.refresh(key, scope, command);
                self.audit.pending_resource_causes.push(cause);
                Ok(())
            }
        }
    }

    fn reconcile_open(
        &mut self,
        key: ResourceKey,
        scope: ScopeId,
        command: C,
        plan: &mut ResourcePlan<C>,
        cause: ResourceCommandCause,
    ) -> GraphResult<()> {
        self.require_scope_open(scope)?;
        let owners = self.resource_owners.entry(key.clone()).or_default();
        let was_empty = owners.is_empty();
        owners.insert(scope);
        if was_empty {
            plan.open(key, scope, command);
            self.audit.pending_resource_causes.push(cause);
        }
        Ok(())
    }

    fn close_scope_resources(&mut self, scope: ScopeId) -> ResourcePlan<C> {
        let keys: Vec<ResourceKey> = self.resource_owners.keys().cloned().collect();
        let mut plan = ResourcePlan::new();
        let cause = ResourceCommandCause::ScopeClosed { scope };
        for key in keys {
            self.remove_resource_owner(&key, scope, &mut plan, cause);
        }
        plan
    }

    fn remove_resource_owner(
        &mut self,
        key: &ResourceKey,
        scope: ScopeId,
        plan: &mut ResourcePlan<C>,
        cause: ResourceCommandCause,
    ) {
        let Some(owners) = self.resource_owners.get_mut(key) else {
            return;
        };
        owners.remove(&scope);
        if owners.is_empty() {
            self.resource_owners.remove(key);
            plan.close(key.clone(), scope);
            self.audit.pending_resource_causes.push(cause);
        }
    }

    fn require_resource_owner(&self, key: &ResourceKey, scope: ScopeId) -> GraphResult<()> {
        let Some(owners) = self.resource_owners.get(key) else {
            return Err(GraphError::ResourceNotOwned);
        };
        if !owners.contains(&scope) {
            return Err(GraphError::ResourceNotOwned);
        }
        Ok(())
    }

    pub(crate) fn require_scope_open(&self, scope: ScopeId) -> GraphResult<()> {
        let scope_meta = self
            .scope_meta(scope)
            .ok_or(GraphError::UnknownScope(scope))?;
        if scope_meta.is_closed() {
            return Err(GraphError::ScopeAlreadyClosed(scope));
        }
        Ok(())
    }

    /// Returns resource owners in deterministic resource-key order.
    pub fn resource_owners(&self, key: &ResourceKey) -> Option<&BTreeSet<ScopeId>> {
        self.resource_owners.get(key)
    }
}
