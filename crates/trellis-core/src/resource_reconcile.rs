use crate::resource_reconcile_aggregate::{
    PlannedResourceCommand, ResourceCommandIntent, ResourceKeyTransition, compare_emitted_commands,
    push_plan_command, shared_payload,
};
use crate::{
    Graph, GraphError, GraphResult, ResourceCoalescedTrace, ResourceCommand, ResourceCommandCause,
    ResourceCommandKind, ResourceKey, ResourcePlan, ScopeId,
};
use std::collections::{BTreeMap, BTreeSet};

impl<C: Clone + PartialEq> Graph<C> {
    pub(crate) fn produce_resource_plan(
        &mut self,
        closed_scopes: &[ScopeId],
    ) -> GraphResult<ResourcePlan<C>> {
        self.audit.pending_resource_causes.clear();
        self.audit.pending_resource_coalescences.clear();
        let planners = self.resource_planners.clone();
        let mut planned_commands = Vec::new();
        for planner in planners {
            if self.collection_diffs.contains_key(&planner.collection) {
                let planned = planner.run(self)?;
                let cause = ResourceCommandCause::Planner {
                    collection: planner.collection,
                };
                self.collect_resource_plan(planner.scope, planned, cause, &mut planned_commands)?;
            }
        }

        for scope in closed_scopes {
            self.collect_scope_close_commands(*scope, &mut planned_commands);
        }

        self.reconcile_planned_resource_commands(planned_commands)
    }

    fn collect_resource_plan(
        &self,
        planner_scope: ScopeId,
        plan: ResourcePlan<C>,
        cause: ResourceCommandCause,
        planned_commands: &mut Vec<PlannedResourceCommand<C>>,
    ) -> GraphResult<()> {
        for command in plan.into_commands() {
            if command.scope() != planner_scope {
                return Err(GraphError::ResourceScopeMismatch(command.scope()));
            }
            let order = planned_commands.len();
            planned_commands.push(PlannedResourceCommand {
                command,
                cause,
                order,
            });
        }
        Ok(())
    }

    fn collect_scope_close_commands(
        &self,
        scope: ScopeId,
        planned_commands: &mut Vec<PlannedResourceCommand<C>>,
    ) {
        let mut keys: Vec<(u64, ResourceKey)> = self
            .resource_acquisitions
            .iter()
            .filter(|((owner_scope, _), _)| *owner_scope == scope)
            .map(|((_, key), sequence)| (*sequence, key.clone()))
            .collect();
        keys.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
        let cause = ResourceCommandCause::ScopeClosed { scope };
        for (_, key) in keys {
            let order = planned_commands.len();
            planned_commands.push(PlannedResourceCommand {
                command: ResourceCommand::Close { key, scope },
                cause,
                order,
            });
        }
    }

    fn reconcile_planned_resource_commands(
        &mut self,
        planned_commands: Vec<PlannedResourceCommand<C>>,
    ) -> GraphResult<ResourcePlan<C>> {
        let mut key_intents = self.initial_resource_intents();
        for planned in &planned_commands {
            self.apply_resource_intent(planned, &mut key_intents)?;
        }

        let keys: BTreeSet<ResourceKey> = self
            .resource_owners
            .keys()
            .chain(key_intents.keys())
            .cloned()
            .collect();
        let mut emitted_commands = Vec::new();
        for key in keys {
            let before_owners = self.resource_owners.get(&key).cloned().unwrap_or_default();
            let before_payload = self.resource_payloads.get(&key).cloned();
            let final_intents = key_intents.remove(&key).unwrap_or_default();
            let final_payload = shared_payload(&key, &final_intents)?;
            let final_owners: BTreeSet<ScopeId> = final_intents
                .iter()
                .filter_map(|(scope, intent)| intent.payload.as_ref().map(|_| *scope))
                .collect();

            self.emit_canonical_resource_commands(
                key.clone(),
                ResourceKeyTransition {
                    before_owners: &before_owners,
                    before_payload: before_payload.as_ref(),
                    final_owners: &final_owners,
                    final_payload: final_payload.as_ref(),
                    final_intents: &final_intents,
                },
                &mut emitted_commands,
            );
            self.apply_canonical_resource_state(key, final_owners, final_payload);
        }
        emitted_commands.sort_by(compare_emitted_commands);
        let mut plan = ResourcePlan::new();
        for emitted in emitted_commands {
            push_plan_command(&mut plan, emitted.command);
            self.audit.pending_resource_causes.push(emitted.cause);
        }
        Ok(plan)
    }

    fn initial_resource_intents(
        &self,
    ) -> BTreeMap<ResourceKey, BTreeMap<ScopeId, ResourceCommandIntent<C>>> {
        let mut intents = BTreeMap::new();
        for (key, owners) in &self.resource_owners {
            let Some(payload) = self.resource_payloads.get(key) else {
                continue;
            };
            let scope_intents = intents.entry(key.clone()).or_insert_with(BTreeMap::new);
            for scope in owners {
                scope_intents.insert(
                    *scope,
                    ResourceCommandIntent {
                        payload: Some(payload.clone()),
                        operation: None,
                        cause: ResourceCommandCause::ScopeClosed { scope: *scope },
                        order: usize::MAX,
                        previous_owner: true,
                    },
                );
            }
        }
        intents
    }

    fn apply_resource_intent(
        &self,
        planned: &PlannedResourceCommand<C>,
        key_intents: &mut BTreeMap<ResourceKey, BTreeMap<ScopeId, ResourceCommandIntent<C>>>,
    ) -> GraphResult<()> {
        match &planned.command {
            ResourceCommand::Open {
                key,
                scope,
                command,
            } => {
                self.require_scope_open(*scope)?;
                let scope_intents = key_intents.entry(key.clone()).or_default();
                let previous_owner = scope_intents
                    .get(scope)
                    .is_some_and(|intent| intent.previous_owner);
                scope_intents.insert(
                    *scope,
                    ResourceCommandIntent {
                        payload: Some(command.clone()),
                        operation: Some(ResourceCommandKind::Open),
                        cause: planned.cause,
                        order: planned.order,
                        previous_owner,
                    },
                );
            }
            ResourceCommand::Close { key, scope } => {
                let scope_intents = key_intents.entry(key.clone()).or_default();
                let previous_owner = scope_intents
                    .get(scope)
                    .is_some_and(|intent| intent.previous_owner);
                scope_intents.insert(
                    *scope,
                    ResourceCommandIntent {
                        payload: None,
                        operation: Some(ResourceCommandKind::Close),
                        cause: planned.cause,
                        order: planned.order,
                        previous_owner,
                    },
                );
            }
            ResourceCommand::Replace {
                key,
                scope,
                command,
            }
            | ResourceCommand::Refresh {
                key,
                scope,
                command,
            } => {
                self.require_scope_open(*scope)?;
                let command_kind = match &planned.command {
                    ResourceCommand::Replace { .. } => ResourceCommandKind::Replace,
                    ResourceCommand::Refresh { .. } => ResourceCommandKind::Refresh,
                    _ => unreachable!(),
                };
                let Some(scope_intents) = key_intents.get_mut(key) else {
                    return Err(GraphError::ResourceNotOwned {
                        key: key.clone(),
                        scope: *scope,
                        command_kind,
                    });
                };
                let Some(intent) = scope_intents.get_mut(scope) else {
                    return Err(GraphError::ResourceNotOwned {
                        key: key.clone(),
                        scope: *scope,
                        command_kind,
                    });
                };
                *intent = ResourceCommandIntent {
                    payload: Some(command.clone()),
                    operation: Some(command_kind),
                    cause: planned.cause,
                    order: planned.order,
                    previous_owner: intent.previous_owner,
                };
            }
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
}
