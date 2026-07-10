use crate::{
    GraphError, GraphResult, ResourceCommand, ResourceCommandCause, ResourceCommandKind,
    ResourceKey, ResourcePayloadConflict, ResourcePlan, ScopeId,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone)]
pub(crate) struct PlannedResourceCommand<C> {
    pub(crate) command: ResourceCommand<C>,
    pub(crate) cause: ResourceCommandCause,
    pub(crate) order: usize,
}

pub(crate) struct ResourceCommandIntent<C> {
    pub(crate) payload: Option<C>,
    pub(crate) operation: Option<ResourceCommandKind>,
    pub(crate) cause: ResourceCommandCause,
    pub(crate) order: usize,
    pub(crate) previous_owner: bool,
}

pub(crate) struct EmittedResourceCommand<C> {
    pub(crate) command: ResourceCommand<C>,
    pub(crate) cause: ResourceCommandCause,
    order: usize,
    phase: u8,
}

pub(crate) fn shared_payload<C: Clone + PartialEq>(
    key: &ResourceKey,
    intents: &BTreeMap<ScopeId, ResourceCommandIntent<C>>,
) -> GraphResult<Option<C>> {
    let mut payloads: Vec<_> = intents
        .iter()
        .filter_map(|(scope, intent)| {
            intent
                .payload
                .as_ref()
                .map(|payload| (*scope, intent, payload))
        })
        .collect();
    if payloads.is_empty() {
        return Ok(None);
    };
    payloads.sort_by_key(|(scope, intent, _)| {
        let current_owner_order = if intent.previous_owner { 0 } else { 1 };
        (current_owner_order, intent.order, *scope)
    });
    let (_, _, first_payload) = payloads[0];
    let mut existing_owners = Vec::new();
    for (scope, _, payload) in payloads {
        if payload != first_payload {
            return Err(GraphError::ResourcePayloadConflict(
                ResourcePayloadConflict {
                    key: key.clone(),
                    joining_scope: scope,
                    existing_owners,
                },
            ));
        }
        existing_owners.push(scope);
    }
    Ok(Some(first_payload.clone()))
}

pub(crate) fn first_scope_by_order<C>(
    intents: &BTreeMap<ScopeId, ResourceCommandIntent<C>>,
    scopes: &BTreeSet<ScopeId>,
) -> Option<ScopeId> {
    scopes_by_order(intents, scopes).into_iter().next()
}

pub(crate) fn scopes_by_order<C>(
    intents: &BTreeMap<ScopeId, ResourceCommandIntent<C>>,
    scopes: &BTreeSet<ScopeId>,
) -> Vec<ScopeId> {
    let mut ordered: Vec<_> = scopes
        .iter()
        .map(|scope| {
            let order = intents.get(scope).map_or(usize::MAX, |intent| intent.order);
            (order, *scope)
        })
        .collect();
    ordered.sort_by_key(|(order, scope)| (*order, *scope));
    ordered.into_iter().map(|(_, scope)| scope).collect()
}

pub(crate) fn first_update_operation<C>(
    intents: &BTreeMap<ScopeId, ResourceCommandIntent<C>>,
    retained_owners: &BTreeSet<ScopeId>,
) -> Option<(ScopeId, ResourceCommandKind)> {
    let mut candidates: Vec<_> = retained_owners
        .iter()
        .filter_map(|scope| {
            let intent = intents.get(scope)?;
            let operation = intent.operation?;
            if matches!(
                operation,
                ResourceCommandKind::Replace | ResourceCommandKind::Refresh
            ) {
                Some((intent.order, *scope, operation))
            } else {
                None
            }
        })
        .collect();
    candidates.sort_by_key(|(order, scope, _)| (*order, *scope));
    candidates
        .into_iter()
        .map(|(_, scope, operation)| (scope, operation))
        .next()
}

pub(crate) fn close_scope_for_removed_owners<C>(
    removed_owners: &BTreeSet<ScopeId>,
    intents: &BTreeMap<ScopeId, ResourceCommandIntent<C>>,
    acquisitions: &BTreeMap<(ScopeId, ResourceKey), u64>,
    key: &ResourceKey,
) -> Option<ScopeId> {
    let mut candidates: Vec<_> = removed_owners
        .iter()
        .map(|scope| {
            let close_order = intents.get(scope).and_then(|intent| {
                matches!(intent.operation, Some(ResourceCommandKind::Close)).then_some(intent.order)
            });
            let sequence = acquisitions
                .get(&(*scope, key.clone()))
                .copied()
                .unwrap_or_default();
            (close_order, sequence, *scope)
        })
        .collect();
    candidates.sort_by(|left, right| {
        compare_optional_order_desc(left.0, right.0)
            .then_with(|| right.1.cmp(&left.1))
            .then_with(|| left.2.cmp(&right.2))
    });
    candidates.into_iter().map(|(_, _, scope)| scope).next()
}

fn compare_optional_order_desc(left: Option<usize>, right: Option<usize>) -> std::cmp::Ordering {
    match (left, right) {
        (Some(left), Some(right)) => right.cmp(&left),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

pub(crate) fn push_emitted_command<C>(
    emitted_commands: &mut Vec<EmittedResourceCommand<C>>,
    intents: &BTreeMap<ScopeId, ResourceCommandIntent<C>>,
    scope: ScopeId,
    phase: u8,
    command: ResourceCommand<C>,
) {
    let order = intents
        .get(&scope)
        .map_or(usize::MAX, |intent| intent.order);
    push_emitted_command_at(emitted_commands, intents, scope, order, phase, command);
}

pub(crate) fn push_emitted_command_at<C>(
    emitted_commands: &mut Vec<EmittedResourceCommand<C>>,
    intents: &BTreeMap<ScopeId, ResourceCommandIntent<C>>,
    scope: ScopeId,
    order: usize,
    phase: u8,
    command: ResourceCommand<C>,
) {
    let cause = intents
        .get(&scope)
        .map_or(ResourceCommandCause::ScopeClosed { scope }, |intent| {
            intent.cause
        });
    emitted_commands.push(EmittedResourceCommand {
        command,
        cause,
        order,
        phase,
    });
}

pub(crate) fn first_planned_order<C>(
    intents: &BTreeMap<ScopeId, ResourceCommandIntent<C>>,
) -> usize {
    intents
        .values()
        .map(|intent| intent.order)
        .min()
        .unwrap_or(usize::MAX)
}

pub(crate) fn compare_emitted_commands<C>(
    left: &EmittedResourceCommand<C>,
    right: &EmittedResourceCommand<C>,
) -> std::cmp::Ordering {
    left.order
        .cmp(&right.order)
        .then_with(|| left.phase.cmp(&right.phase))
        .then_with(|| left.command.key().cmp(right.command.key()))
        .then_with(|| left.command.scope().cmp(&right.command.scope()))
        .then_with(|| command_kind_order(&left.command).cmp(&command_kind_order(&right.command)))
}

pub(crate) fn push_plan_command<C>(plan: &mut ResourcePlan<C>, command: ResourceCommand<C>) {
    match command {
        ResourceCommand::Open {
            key,
            scope,
            command,
        } => plan.open(key, scope, command),
        ResourceCommand::Close { key, scope } => plan.close(key, scope),
        ResourceCommand::Replace {
            key,
            scope,
            command,
        } => plan.replace(key, scope, command),
        ResourceCommand::Refresh {
            key,
            scope,
            command,
        } => plan.refresh(key, scope, command),
    }
}

fn command_kind_order<C>(command: &ResourceCommand<C>) -> u8 {
    match command {
        ResourceCommand::Open { .. } => 0,
        ResourceCommand::Close { .. } => 1,
        ResourceCommand::Replace { .. } => 2,
        ResourceCommand::Refresh { .. } => 3,
    }
}
