use std::collections::{BTreeMap, BTreeSet};

use trellis_core::{DependencyList, Graph, ResourceCommand, ResourceKey, ResourcePlan};

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

fn map(entries: &[(&str, u64)]) -> BTreeMap<String, u64> {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_owned(), *value))
        .collect()
}

#[test]
fn added_set_members_produce_open_commands_in_deterministic_order() {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let source = tx.input::<BTreeSet<String>>("source").unwrap();
    tx.set_input(source, set(&["c", "a"])).unwrap();
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
        Ok(plan)
    })
    .unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert_eq!(
        result.resource_plan.commands(),
        &[
            ResourceCommand::Open {
                key: key("a"),
                scope,
                command: Command::Open("a".to_owned()),
            },
            ResourceCommand::Open {
                key: key("c"),
                scope,
                command: Command::Open("c".to_owned()),
            },
        ]
    );
}

#[test]
fn empty_collection_produces_no_open_commands() {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let collection = tx
        .set_collection::<String>(
            "resources",
            DependencyList::empty(),
            |_| Ok(BTreeSet::new()),
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
        Ok(plan)
    })
    .unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert!(result.resource_plan.commands().is_empty());
}

#[test]
fn removed_set_members_produce_close_commands() {
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
    tx.set_input(source, set(&["a"])).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert_eq!(
        result.resource_plan.commands(),
        &[ResourceCommand::Close {
            key: key("b"),
            scope,
        }]
    );
}

#[test]
fn updated_map_members_produce_replace_commands() {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let source = tx.input::<BTreeMap<String, u64>>("source").unwrap();
    tx.set_input(source, map(&[("a", 1)])).unwrap();
    let collection = tx
        .collection(
            "resources",
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(source)?.clone()),
        )
        .unwrap();
    tx.map_resource_planner(collection, scope, move |ctx| {
        let mut plan = ResourcePlan::new();
        for added in &ctx.diff().added {
            plan.open(
                key(&added.value.0),
                ctx.scope(),
                Command::Open(added.value.0.clone()),
            );
        }
        for updated in &ctx.diff().updated {
            plan.replace(
                key(&updated.key),
                ctx.scope(),
                Command::Replace(updated.key.clone(), updated.current),
            );
        }
        Ok(plan)
    })
    .unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(source, map(&[("a", 2)])).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert_eq!(
        result.resource_plan.commands(),
        &[ResourceCommand::Replace {
            key: key("a"),
            scope,
            command: Command::Replace("a".to_owned(), 2),
        }]
    );
}

#[test]
fn scope_close_closes_owned_resources() {
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
    assert!(graph.resource_owners(&key("a")).is_none());
}

#[test]
fn plan_debug_includes_command_payload_when_payload_supports_debug() {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut plan = ResourcePlan::new();
    plan.open(key("a"), scope, Command::Open("a".to_owned()));

    let debug = format!("{plan:?}");

    assert!(debug.contains("ResourcePlan"));
    assert!(debug.contains("Open(\"a\")"));
}
