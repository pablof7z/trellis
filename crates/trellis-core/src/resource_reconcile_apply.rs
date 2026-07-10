use crate::resource_reconcile_aggregate::{
    EmittedResourceCommand, ResourceCommandIntent, ResourceKeyTransition,
    close_scope_for_removed_owners, first_planned_order, first_scope_by_order,
    first_update_operation, push_emitted_command, push_emitted_command_at, scopes_by_order,
};
use crate::{
    Graph, ResourceCoalescedTrace, ResourceCommand, ResourceCommandKind, ResourceKey, ScopeId,
};
use std::collections::{BTreeMap, BTreeSet};

impl<C: Clone + PartialEq> Graph<C> {
    pub(crate) fn emit_canonical_resource_commands(
        &mut self,
        key: ResourceKey,
        transition: ResourceKeyTransition<'_, C>,
        emitted_commands: &mut Vec<EmittedResourceCommand<C>>,
    ) {
        let ResourceKeyTransition {
            before_owners,
            before_payload,
            final_owners,
            final_payload,
            final_intents,
        } = transition;
        let removed_owners: BTreeSet<_> = before_owners.difference(final_owners).copied().collect();
        let added_owners: BTreeSet<_> = final_owners.difference(before_owners).copied().collect();
        let retained_owners: BTreeSet<_> =
            before_owners.intersection(final_owners).copied().collect();

        match (before_payload, final_payload) {
            (None, Some(payload)) => {
                let Some(open_scope) = first_scope_by_order(final_intents, final_owners) else {
                    return;
                };
                push_emitted_command(
                    emitted_commands,
                    final_intents,
                    open_scope,
                    0,
                    ResourceCommand::Open {
                        key: key.clone(),
                        scope: open_scope,
                        command: payload.clone(),
                    },
                );
                self.record_added_owner_coalescences(
                    &key,
                    &added_owners,
                    final_intents,
                    Some(open_scope),
                );
            }
            (Some(_), None) => {
                let close_scope = close_scope_for_removed_owners(
                    &removed_owners,
                    final_intents,
                    &self.resource_acquisitions,
                    &key,
                );
                if let Some(close_scope) = close_scope {
                    push_emitted_command(
                        emitted_commands,
                        final_intents,
                        close_scope,
                        0,
                        ResourceCommand::Close {
                            key: key.clone(),
                            scope: close_scope,
                        },
                    );
                }
            }
            (Some(before), Some(after)) if before != after => {
                if retained_owners.is_empty() {
                    let transition_order = first_planned_order(final_intents);
                    if let Some(close_scope) = close_scope_for_removed_owners(
                        &removed_owners,
                        final_intents,
                        &self.resource_acquisitions,
                        &key,
                    ) {
                        push_emitted_command_at(
                            emitted_commands,
                            final_intents,
                            close_scope,
                            transition_order,
                            0,
                            ResourceCommand::Close {
                                key: key.clone(),
                                scope: close_scope,
                            },
                        );
                    }
                    if let Some(open_scope) = first_scope_by_order(final_intents, final_owners) {
                        push_emitted_command_at(
                            emitted_commands,
                            final_intents,
                            open_scope,
                            transition_order,
                            1,
                            ResourceCommand::Open {
                                key: key.clone(),
                                scope: open_scope,
                                command: after.clone(),
                            },
                        );
                        self.record_added_owner_coalescences(
                            &key,
                            &added_owners,
                            final_intents,
                            Some(open_scope),
                        );
                    }
                } else if let Some((scope, operation)) =
                    first_update_operation(final_intents, &retained_owners)
                {
                    let command = match operation {
                        ResourceCommandKind::Replace => ResourceCommand::Replace {
                            key: key.clone(),
                            scope,
                            command: after.clone(),
                        },
                        ResourceCommandKind::Refresh => ResourceCommand::Refresh {
                            key: key.clone(),
                            scope,
                            command: after.clone(),
                        },
                        ResourceCommandKind::Open | ResourceCommandKind::Close => {
                            ResourceCommand::Replace {
                                key: key.clone(),
                                scope,
                                command: after.clone(),
                            }
                        }
                    };
                    push_emitted_command(emitted_commands, final_intents, scope, 0, command);
                    self.record_added_owner_coalescences(&key, &added_owners, final_intents, None);
                }
            }
            (Some(_), Some(_)) => {
                self.record_added_owner_coalescences(&key, &added_owners, final_intents, None);
            }
            (None, None) => {}
        }
    }

    fn record_added_owner_coalescences(
        &mut self,
        key: &ResourceKey,
        added_owners: &BTreeSet<ScopeId>,
        final_intents: &BTreeMap<ScopeId, ResourceCommandIntent<C>>,
        primary_scope: Option<ScopeId>,
    ) {
        let mut existing_owner_count = self.resource_owners.get(key).map_or(0, BTreeSet::len);
        for scope in scopes_by_order(final_intents, added_owners) {
            if Some(scope) == primary_scope {
                existing_owner_count += 1;
                continue;
            }
            self.audit
                .pending_resource_coalescences
                .push(ResourceCoalescedTrace {
                    key: key.clone(),
                    scope,
                    existing_owner_count,
                });
            existing_owner_count += 1;
        }
    }

    pub(crate) fn apply_canonical_resource_state(
        &mut self,
        key: ResourceKey,
        final_owners: BTreeSet<ScopeId>,
        final_payload: Option<C>,
    ) {
        let previous_owners = self.resource_owners.get(&key).cloned().unwrap_or_default();
        for removed in previous_owners.difference(&final_owners) {
            self.resource_acquisitions.remove(&(*removed, key.clone()));
        }
        for added in final_owners.difference(&previous_owners) {
            self.record_resource_acquisition(*added, &key);
        }
        if final_owners.is_empty() {
            self.resource_owners.remove(&key);
            self.resource_payloads.remove(&key);
        } else {
            self.resource_owners.insert(key.clone(), final_owners);
            if let Some(payload) = final_payload {
                self.resource_payloads.insert(key, payload);
            }
        }
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
