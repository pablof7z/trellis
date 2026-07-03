use std::collections::BTreeSet;

use trellis_core::{
    AuditEvent, DependencyList, Graph, GraphError, ResourceCoalescedTrace, ResourceCommand,
    ResourceCommandKind, ResourceKey, ResourcePayloadConflict, ResourcePlan, ScopeId,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Command {
    Open(String),
}

fn key(value: &str) -> ResourceKey {
    ResourceKey::new(value.to_owned())
}

fn set(entries: &[&str]) -> BTreeSet<String> {
    entries.iter().map(|value| (*value).to_owned()).collect()
}

fn register_set_planner(
    tx: &mut trellis_core::Transaction<'_, Command>,
    scope: ScopeId,
    source_name: &str,
    command_payload: &'static str,
) -> trellis_core::InputNode<BTreeSet<String>> {
    let source = tx.input::<BTreeSet<String>>(source_name).unwrap();
    tx.set_input(source, set(&["shared"])).unwrap();
    let collection = tx
        .set_collection(
            format!("{source_name}-resources"),
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(source)?.clone()),
        )
        .unwrap();
    tx.set_resource_planner(collection, scope, move |ctx| {
        let mut plan = ResourcePlan::new();
        for added in &ctx.diff().added {
            plan.open(
                key(&added.value),
                ctx.scope(),
                Command::Open(command_payload.to_owned()),
            );
        }
        for removed in &ctx.diff().removed {
            plan.close(key(&removed.value), ctx.scope());
        }
        Ok(plan)
    })
    .unwrap();
    source
}

#[test]
fn equal_payload_open_coalesces_with_trace_and_audit() {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let first = tx.create_scope("first").unwrap();
    let second = tx.create_scope("second").unwrap();
    register_set_planner(&mut tx, first, "first-source", "same");
    register_set_planner(&mut tx, second, "second-source", "same");
    let result = tx.commit().unwrap();
    drop(tx);

    assert_eq!(
        result.resource_plan.commands(),
        &[ResourceCommand::Open {
            key: key("shared"),
            scope: first,
            command: Command::Open("same".to_owned()),
        }]
    );
    assert_eq!(
        result.resource_coalescences,
        vec![ResourceCoalescedTrace {
            key: key("shared"),
            scope: second,
            existing_owner_count: 1,
        }]
    );
    assert_eq!(
        result.trace().resource_coalescences,
        result.resource_coalescences
    );
    assert!(result.audit_log.iter().any(|entry| {
        entry.event
            == AuditEvent::ResourceOpenCoalesced {
                key: key("shared"),
                scope: second,
                existing_owner_count: 1,
            }
    }));
    assert_eq!(
        graph.resource_owners(&key("shared")).cloned(),
        Some(BTreeSet::from([first, second]))
    );
}

#[test]
fn divergent_payload_open_fails_without_partial_owner_state() {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let first = tx.create_scope("first").unwrap();
    let second = tx.create_scope("second").unwrap();
    register_set_planner(&mut tx, first, "first-source", "same");
    let second_source = tx.input::<BTreeSet<String>>("second-source").unwrap();
    tx.set_input(second_source, set(&["shared"])).unwrap();
    let second_collection = tx
        .set_collection(
            "second-resources",
            DependencyList::new([second_source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(second_source)?.clone()),
        )
        .unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_resource_planner(second_collection, second, move |ctx| {
        let mut plan = ResourcePlan::new();
        for added in &ctx.diff().added {
            plan.open(
                key(&added.value),
                ctx.scope(),
                Command::Open("different".to_owned()),
            );
        }
        Ok(plan)
    })
    .unwrap();
    let error = tx.commit().unwrap_err();
    drop(tx);

    assert_eq!(
        error,
        GraphError::ResourcePayloadConflict(ResourcePayloadConflict {
            key: key("shared"),
            joining_scope: second,
            existing_owners: vec![first],
        })
    );
    assert_eq!(
        graph.resource_owners(&key("shared")).cloned(),
        Some(BTreeSet::from([first]))
    );
}

#[test]
fn coalesced_owner_close_waits_for_last_owner() {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let first = tx.create_scope("first").unwrap();
    let second = tx.create_scope("second").unwrap();
    let first_source = register_set_planner(&mut tx, first, "first-source", "same");
    let second_source = register_set_planner(&mut tx, second, "second-source", "same");
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(second_source, BTreeSet::new()).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);
    assert!(result.resource_plan.commands().is_empty());
    assert_eq!(
        graph.resource_owners(&key("shared")).cloned(),
        Some(BTreeSet::from([first]))
    );

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(first_source, BTreeSet::new()).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);
    assert_eq!(
        result.resource_plan.commands(),
        &[ResourceCommand::Close {
            key: key("shared"),
            scope: first,
        }]
    );
    assert!(graph.resource_owners(&key("shared")).is_none());
}

#[test]
fn scope_close_uses_reverse_acquisition_order() {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let source = tx.input::<BTreeSet<String>>("source").unwrap();
    tx.set_input(source, set(&["a", "b", "c"])).unwrap();
    let collection = tx
        .set_collection(
            "resources",
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(source)?.clone()),
        )
        .unwrap();
    tx.set_resource_planner(collection, scope, move |ctx| {
        let mut plan = ResourcePlan::new();
        for added in &ctx.diff().added {
            plan.open(
                key(&added.value),
                ctx.scope(),
                Command::Open(added.value.clone()),
            );
        }
        for removed in &ctx.diff().removed {
            plan.close(key(&removed.value), ctx.scope());
        }
        Ok(plan)
    })
    .unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.close_scope(scope).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert_eq!(
        result
            .trace()
            .resource_commands
            .iter()
            .map(|command| (&command.key, command.kind))
            .collect::<Vec<_>>(),
        vec![
            (&key("c"), ResourceCommandKind::Close),
            (&key("b"), ResourceCommandKind::Close),
            (&key("a"), ResourceCommandKind::Close),
        ]
    );
}

#[test]
fn release_then_reacquire_moves_key_to_top_of_scope_stack() {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let source = tx.input::<BTreeSet<String>>("source").unwrap();
    tx.set_input(source, set(&["a", "b"])).unwrap();
    let collection = tx
        .set_collection(
            "resources",
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(source)?.clone()),
        )
        .unwrap();
    tx.set_resource_planner(collection, scope, move |ctx| {
        let mut plan = ResourcePlan::new();
        for added in &ctx.diff().added {
            plan.open(
                key(&added.value),
                ctx.scope(),
                Command::Open(added.value.clone()),
            );
        }
        for removed in &ctx.diff().removed {
            plan.close(key(&removed.value), ctx.scope());
        }
        Ok(plan)
    })
    .unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(source, set(&["b"])).unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(source, set(&["a", "b"])).unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.close_scope(scope).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert_eq!(
        result.resource_plan.commands(),
        &[
            ResourceCommand::Close {
                key: key("a"),
                scope,
            },
            ResourceCommand::Close {
                key: key("b"),
                scope,
            },
        ]
    );
}
