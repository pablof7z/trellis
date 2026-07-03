use crate::{
    Graph, GraphError, GraphResult, ResourceCoalescedTrace, ResourceCommand, ResourceCommandCause,
    ResourceCommandKind, ResourceKey, ResourcePayloadConflict, ResourcePlan, ScopeId,
};
use std::collections::BTreeSet;

impl<C: Clone + PartialEq> Graph<C> {
    pub(crate) fn produce_resource_plan(
        &mut self,
        closed_scopes: &[ScopeId],
    ) -> GraphResult<ResourcePlan<C>> {
        self.audit.pending_resource_causes.clear();
        self.audit.pending_resource_coalescences.clear();
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
                self.require_resource_owner(&key, scope, ResourceCommandKind::Replace)?;
                self.resource_owners
                    .entry(key.clone())
                    .or_default()
                    .insert(scope);
                self.resource_payloads.insert(key.clone(), command.clone());
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
                self.require_resource_owner(&key, scope, ResourceCommandKind::Refresh)?;
                self.resource_owners
                    .entry(key.clone())
                    .or_default()
                    .insert(scope);
                self.resource_payloads.insert(key.clone(), command.clone());
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
        let existing_payload = self.resource_payloads.get(&key);
        let existing_owners = self.resource_owners.get(&key);
        let was_empty = existing_owners.is_none_or(BTreeSet::is_empty);
        let already_owned = existing_owners.is_some_and(|owners| owners.contains(&scope));
        if let Some(existing_payload) = existing_payload
            && existing_payload != &command
        {
            return Err(GraphError::ResourcePayloadConflict(
                ResourcePayloadConflict {
                    key,
                    joining_scope: scope,
                    existing_owners: existing_owners
                        .into_iter()
                        .flat_map(|owners| owners.iter().copied())
                        .collect(),
                },
            ));
        }
        let existing_owner_count = existing_owners.map_or(0, BTreeSet::len);
        let owners = self.resource_owners.entry(key.clone()).or_default();
        owners.insert(scope);
        if was_empty {
            self.resource_payloads.insert(key.clone(), command.clone());
            self.record_resource_acquisition(scope, &key);
            plan.open(key, scope, command);
            self.audit.pending_resource_causes.push(cause);
        } else if !already_owned {
            self.record_resource_acquisition(scope, &key);
            self.audit
                .pending_resource_coalescences
                .push(ResourceCoalescedTrace {
                    key,
                    scope,
                    existing_owner_count,
                });
        }
        Ok(())
    }

    fn close_scope_resources(&mut self, scope: ScopeId) -> ResourcePlan<C> {
        let mut keys: Vec<(u64, ResourceKey)> = self
            .resource_acquisitions
            .iter()
            .filter(|((owner_scope, _), _)| *owner_scope == scope)
            .map(|((_, key), sequence)| (*sequence, key.clone()))
            .collect();
        keys.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
        let mut plan = ResourcePlan::new();
        let cause = ResourceCommandCause::ScopeClosed { scope };
        for (_, key) in keys {
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
        self.resource_acquisitions.remove(&(scope, key.clone()));
        if owners.is_empty() {
            self.resource_owners.remove(key);
            self.resource_payloads.remove(key);
            plan.close(key.clone(), scope);
            self.audit.pending_resource_causes.push(cause);
        }
    }

    fn require_resource_owner(
        &self,
        key: &ResourceKey,
        scope: ScopeId,
        command_kind: ResourceCommandKind,
    ) -> GraphResult<()> {
        let Some(owners) = self.resource_owners.get(key) else {
            return Err(GraphError::ResourceNotOwned {
                key: key.clone(),
                scope,
                command_kind,
            });
        };
        if !owners.contains(&scope) {
            return Err(GraphError::ResourceNotOwned {
                key: key.clone(),
                scope,
                command_kind,
            });
        }
        Ok(())
    }
}

impl<C> Graph<C> {
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

    pub(crate) fn take_pending_resource_coalescences(&mut self) -> Vec<ResourceCoalescedTrace> {
        std::mem::take(&mut self.audit.pending_resource_coalescences)
    }

    fn record_resource_acquisition(&mut self, scope: ScopeId, key: &ResourceKey) {
        let entry = (scope, key.clone());
        if !self.resource_acquisitions.contains_key(&entry) {
            let sequence = self.next_resource_acquisition;
            self.next_resource_acquisition += 1;
            self.resource_acquisitions.insert(entry, sequence);
        }
    }
}
