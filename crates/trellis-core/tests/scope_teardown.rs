use std::collections::BTreeSet;

use trellis_core::{
    AuditEvent, DependencyList, Graph, GraphError, ResourceCommand, ResourceKey, ResourcePlan,
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

#[test]
fn closing_parent_closes_children_first_and_detaches_nodes() {
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let root = tx.create_scope("root").unwrap();
    let child = tx.create_scope_with_parent("child", Some(root)).unwrap();
    let grandchild = tx
        .create_scope_with_parent("grandchild", Some(child))
        .unwrap();
    let root_node = tx.input::<String>("root-node").unwrap();
    let child_node = tx.input::<String>("child-node").unwrap();
    tx.attach_node_to_scope(root_node, root).unwrap();
    tx.attach_node_to_scope(child_node, child).unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.close_scope(root).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    let closed_scopes: Vec<_> = result
        .audit_log
        .iter()
        .filter_map(|entry| match entry.event {
            AuditEvent::ScopeClosed(scope) => Some(scope),
            _ => None,
        })
        .collect();
    assert_eq!(closed_scopes, vec![grandchild, child, root]);
    assert!(graph.scope_meta(root).unwrap().is_closed());
    assert!(graph.scope_meta(child).unwrap().is_closed());
    assert!(graph.scope_meta(grandchild).unwrap().is_closed());
    assert_eq!(graph.node_meta(root_node).unwrap().owning_scope(), None);
    assert_eq!(graph.node_meta(child_node).unwrap().owning_scope(), None);
}

#[test]
fn closed_scope_rejects_new_children_nodes_and_resources() {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let collection = tx
        .set_collection("resources", DependencyList::empty(), |_| Ok(set(&["a"])))
        .unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.close_scope(scope).unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    assert_eq!(
        tx.create_scope_with_parent("child", Some(scope))
            .unwrap_err(),
        GraphError::ScopeAlreadyClosed(scope)
    );
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    let node = tx.input::<String>("node").unwrap();
    assert_eq!(
        tx.attach_node_to_scope(node, scope).unwrap_err(),
        GraphError::ScopeAlreadyClosed(scope)
    );
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    assert_eq!(
        tx.set_resource_planner(collection, scope, |_| Ok(ResourcePlan::new()))
            .unwrap_err(),
        GraphError::ScopeAlreadyClosed(scope)
    );
}

#[test]
fn closing_parent_closes_child_resources_without_orphans() {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let parent = tx.create_scope("parent").unwrap();
    let child = tx.create_scope_with_parent("child", Some(parent)).unwrap();
    let parent_collection = tx
        .set_collection("parent-resources", DependencyList::empty(), |_| {
            Ok(set(&["parent"]))
        })
        .unwrap();
    let child_collection = tx
        .set_collection("child-resources", DependencyList::empty(), |_| {
            Ok(set(&["child"]))
        })
        .unwrap();
    for (collection, scope) in [(parent_collection, parent), (child_collection, child)] {
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
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.close_scope(parent).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert_eq!(
        result.resource_plan.commands(),
        &[
            ResourceCommand::Close {
                key: key("child"),
                scope: child,
            },
            ResourceCommand::Close {
                key: key("parent"),
                scope: parent,
            },
        ]
    );
    assert!(graph.resource_owners(&key("child")).is_none());
    assert!(graph.resource_owners(&key("parent")).is_none());
    assert!(graph.orphan_resources().is_empty());
}

#[test]
fn shared_parent_child_resource_closes_once_after_last_owner() {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let parent = tx.create_scope("parent").unwrap();
    let child = tx.create_scope_with_parent("child", Some(parent)).unwrap();
    let parent_collection = tx
        .set_collection("parent-resources", DependencyList::empty(), |_| {
            Ok(set(&["shared"]))
        })
        .unwrap();
    let child_collection = tx
        .set_collection("child-resources", DependencyList::empty(), |_| {
            Ok(set(&["shared"]))
        })
        .unwrap();
    for (collection, scope) in [(parent_collection, parent), (child_collection, child)] {
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
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.close_scope(parent).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert_eq!(
        result.resource_plan.commands(),
        &[ResourceCommand::Close {
            key: key("shared"),
            scope: parent,
        }]
    );
    assert!(graph.resource_owners(&key("shared")).is_none());
    assert!(graph.orphan_resources().is_empty());
}
