use std::collections::BTreeSet;

use trellis_core::{
    DependencyList, Graph, GraphError, ResourceCommand, ResourceCommandKind, ResourceKey,
    ResourcePlan,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Command {
    Open(String),
    Replace(String, u64),
}

fn key(value: &str) -> ResourceKey {
    ResourceKey::new(value.to_owned())
}

fn set(entries: &[&str]) -> BTreeSet<String> {
    entries.iter().map(|value| (*value).to_owned()).collect()
}

#[test]
fn late_planner_registration_opens_existing_collection_members() {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let collection = tx
        .set_collection("resources", DependencyList::empty(), |_| Ok(set(&["a"])))
        .unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_resource_planner(collection, scope, move |ctx| {
        let mut plan = ResourcePlan::new();
        for added in &ctx.diff().added {
            plan.open(
                key(&added.value),
                ctx.scope(),
                Command::Open(added.value.clone()),
            );
        }
        Ok(plan)
    })
    .unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert_eq!(
        result.resource_plan.commands(),
        &[ResourceCommand::Open {
            key: key("a"),
            scope,
            command: Command::Open("a".to_owned()),
        }]
    );
}

#[test]
fn planner_cannot_emit_commands_for_another_scope() {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope_a = tx.create_scope("a").unwrap();
    let scope_b = tx.create_scope("b").unwrap();
    let collection = tx
        .set_collection("resources", DependencyList::empty(), |_| Ok(set(&["a"])))
        .unwrap();
    tx.set_resource_planner(collection, scope_a, move |ctx| {
        let mut plan = ResourcePlan::new();
        for added in &ctx.diff().added {
            plan.open(
                key(&added.value),
                scope_b,
                Command::Open(added.value.clone()),
            );
        }
        Ok(plan)
    })
    .unwrap();
    let error = tx.commit().unwrap_err();
    drop(tx);

    assert_eq!(error, GraphError::ResourceScopeMismatch(scope_b));
    assert!(graph.resource_owners(&key("a")).is_none());
}

#[test]
fn replace_without_existing_owner_fails_atomically() {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let collection = tx
        .set_collection("resources", DependencyList::empty(), |_| Ok(set(&["a"])))
        .unwrap();
    tx.set_resource_planner(collection, scope, move |ctx| {
        let mut plan = ResourcePlan::new();
        for added in &ctx.diff().added {
            plan.replace(
                key(&added.value),
                ctx.scope(),
                Command::Replace(added.value.clone(), 1),
            );
        }
        Ok(plan)
    })
    .unwrap();
    let error = tx.commit().unwrap_err();
    drop(tx);

    assert_eq!(
        error,
        GraphError::ResourceNotOwned {
            key: key("a"),
            scope,
            command_kind: ResourceCommandKind::Replace,
        }
    );
    assert!(graph.resource_owners(&key("a")).is_none());
}

#[test]
fn shared_resource_closes_only_after_last_owner() {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope_a = tx.create_scope("a").unwrap();
    let scope_b = tx.create_scope("b").unwrap();
    let collection_a = tx
        .set_collection("a_resources", DependencyList::empty(), |_| {
            Ok(set(&["shared"]))
        })
        .unwrap();
    let collection_b = tx
        .set_collection("b_resources", DependencyList::empty(), |_| {
            Ok(set(&["shared"]))
        })
        .unwrap();
    for (collection, scope) in [(collection_a, scope_a), (collection_b, scope_b)] {
        tx.set_resource_planner(collection, scope, move |ctx| {
            let mut plan = ResourcePlan::new();
            for added in &ctx.diff().added {
                plan.open(
                    key(&added.value),
                    ctx.scope(),
                    Command::Open(added.value.clone()),
                );
            }
            Ok(plan)
        })
        .unwrap();
    }
    let result = tx.commit().unwrap();
    drop(tx);

    assert_eq!(result.resource_plan.commands().len(), 1);

    let mut tx = graph.begin_transaction().unwrap();
    tx.close_scope(scope_a).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert!(result.resource_plan.commands().is_empty());
    assert_eq!(
        graph
            .resource_owners(&key("shared"))
            .unwrap()
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![scope_b]
    );

    let mut tx = graph.begin_transaction().unwrap();
    tx.close_scope(scope_b).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert_eq!(
        result.resource_plan.commands(),
        &[ResourceCommand::Close {
            key: key("shared"),
            scope: scope_b,
        }]
    );
}

#[test]
fn closing_scope_twice_is_idempotent_for_resource_plans() {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.close_scope(scope).unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.close_scope(scope).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert!(result.resource_plan.commands().is_empty());
}

#[test]
fn closed_scope_planner_does_not_run_on_later_collection_diffs() {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let source = tx.input::<BTreeSet<String>>("source").unwrap();
    tx.set_input(source, set(&["a"])).unwrap();
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
        result.resource_plan.commands(),
        &[ResourceCommand::Close {
            key: key("a"),
            scope,
        }]
    );

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(source, set(&["b"])).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert!(result.resource_plan.commands().is_empty());
    assert!(graph.resource_owners(&key("a")).is_none());
    assert!(graph.resource_owners(&key("b")).is_none());
}
