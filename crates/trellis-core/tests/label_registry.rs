use std::collections::BTreeSet;

use trellis_core::{DependencyList, Graph, ResourceKey};

#[derive(Clone, Debug, PartialEq)]
enum Command {
    Open,
}

#[test]
fn graph_exports_stable_label_registry_from_metadata() {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("workspace/session-1").unwrap();
    let source = tx.input::<BTreeSet<u8>>("source").unwrap();
    let duplicate = tx.input::<u8>("source").unwrap();
    tx.set_input(source, [1, 2].into()).unwrap();
    tx.set_input(duplicate, 7).unwrap();
    let demand = tx
        .set_collection(
            "resource-demand",
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(source)?.clone()),
        )
        .unwrap();
    tx.open_close_planner(
        demand,
        scope,
        |value| ResourceKey::new(format!("resource/{value}")),
        |_| Command::Open,
    )
    .unwrap();
    let output = tx
        .materialized_output(
            "visible-output",
            scope,
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(source)?.len()),
        )
        .unwrap();
    tx.commit().unwrap();
    drop(tx);

    let registry = graph.label_registry();

    assert_eq!(
        registry
            .nodes()
            .iter()
            .map(|entry| (entry.id, entry.label.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (source.id(), "source"),
            (duplicate.id(), "source"),
            (demand.id(), "resource-demand"),
        ]
    );
    assert_eq!(
        registry
            .scopes()
            .iter()
            .map(|entry| (entry.id, entry.label.as_str()))
            .collect::<Vec<_>>(),
        vec![(scope, "workspace/session-1")]
    );
    assert_eq!(
        registry
            .resources()
            .iter()
            .map(|entry| (entry.key.as_str(), entry.label.as_str()))
            .collect::<Vec<_>>(),
        vec![("resource/1", "resource/1"), ("resource/2", "resource/2")]
    );
    assert_eq!(
        registry
            .outputs()
            .iter()
            .map(|entry| (entry.key, entry.label.as_str()))
            .collect::<Vec<_>>(),
        vec![(output.key(), "visible-output")]
    );
}

#[test]
fn registry_labels_are_diagnostic_not_identity() {
    let mut registry = trellis_core::GraphLabelRegistry::new();
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let first = tx.input::<u8>("same").unwrap();
    let second = tx.input::<u8>("same").unwrap();
    tx.commit().unwrap();

    registry.label_node(first.id(), "same");
    registry.label_node(second.id(), "same");

    assert_ne!(first.id(), second.id());
    assert_eq!(registry.nodes().len(), 2);
    assert_eq!(registry.nodes()[0].label, registry.nodes()[1].label);
}
