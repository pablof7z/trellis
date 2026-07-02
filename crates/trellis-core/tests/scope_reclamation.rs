use std::collections::BTreeSet;

use trellis_core::{DependencyList, Graph, GraphError, ResourceCommand, ResourceKey};

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
fn closing_scope_reclaims_owned_node_values_specs_and_planners() {
    let mut graph = Graph::<Command, usize>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let source = tx.input::<BTreeSet<String>>("source").unwrap();
    tx.set_input(source, set(&["a"])).unwrap();
    let count = tx
        .derived(
            "count",
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(source)?.len()),
        )
        .unwrap();
    let collection = tx
        .set_collection(
            "resources",
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(source)?.clone()),
        )
        .unwrap();
    tx.attach_node_to_scope(source, scope).unwrap();
    tx.attach_node_to_scope(count, scope).unwrap();
    tx.attach_node_to_scope(collection, scope).unwrap();
    tx.open_close_planner(
        collection,
        scope,
        |value| key(value),
        |_| Command::Open("a".into()),
    )
    .unwrap();
    tx.materialized_output(
        "count-output",
        scope,
        DependencyList::new([count.id()]).unwrap(),
        move |ctx| Ok(*ctx.derived(count)?),
    )
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
    assert_eq!(result.output_frames.len(), 1);
    assert!(graph.scope_meta(scope).is_none());
    assert!(graph.node_meta(source).is_none());
    assert!(graph.node_meta(count).is_none());
    assert!(graph.node_meta(collection).is_none());
    assert_eq!(
        graph.input_value(source).unwrap_err(),
        GraphError::UnknownNode(source.id())
    );
    assert_eq!(
        graph.derived_value(count).unwrap_err(),
        GraphError::UnknownNode(count.id())
    );
    assert_eq!(
        graph.set_collection(collection).unwrap_err(),
        GraphError::UnknownNode(collection.id())
    );

    let mut tx = graph.begin_transaction().unwrap();
    assert_eq!(
        tx.set_input(source, set(&["b"])).unwrap_err(),
        GraphError::UnknownNode(source.id())
    );
    drop(tx);
    graph.assert_incremental_equals_full().unwrap();
}
