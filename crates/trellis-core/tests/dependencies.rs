use std::collections::BTreeMap;

use trellis_core::{DependencyList, Graph, GraphError, NodeKind};

#[test]
fn dependencies_are_inspectable_and_ordered() {
    let mut graph = Graph::new();

    let mut tx = graph.begin_transaction().unwrap();
    let input = tx.input::<String>("input").unwrap();
    let derived = tx
        .derived::<usize>(
            "derived",
            DependencyList::new([input.id()]).unwrap(),
            |_| Ok(0),
        )
        .unwrap();
    let collection = tx
        .collection::<String, usize>(
            "collection",
            DependencyList::new([input.id(), derived.id()]).unwrap(),
            |_| Ok(BTreeMap::new()),
        )
        .unwrap();
    tx.commit().unwrap();
    drop(tx);

    assert_eq!(
        graph.dependencies(derived).unwrap().as_slice(),
        &[input.id()]
    );
    assert_eq!(
        graph.dependencies(collection).unwrap().as_slice(),
        &[input.id(), derived.id()]
    );
    assert_eq!(graph.node_meta(derived).unwrap().kind(), NodeKind::Derived);
    assert_eq!(
        graph.node_meta(collection).unwrap().kind(),
        NodeKind::Collection
    );
}

#[test]
fn dependency_list_rejects_duplicate_nodes() {
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let input = tx.input::<String>("input").unwrap();

    let error = DependencyList::new([input.id(), input.id()]).unwrap_err();
    tx.commit().unwrap();

    assert_eq!(error, GraphError::DuplicateDependency(input.id()));
}

#[test]
fn graph_rejects_unknown_dependency() {
    let mut other_graph = Graph::new();
    let mut other_tx = other_graph.begin_transaction().unwrap();
    let unknown = other_tx.input::<String>("foreign").unwrap();
    other_tx.commit().unwrap();
    drop(other_tx);

    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let error = tx
        .derived::<usize>(
            "derived",
            DependencyList::new([unknown.id()]).expect("foreign id is still typed"),
            |_| Ok(0),
        )
        .unwrap_err();

    assert_eq!(error, GraphError::SelfDependency(unknown.id()));

    let known = tx.input::<String>("known").unwrap();
    let mut other_tx = other_graph.begin_transaction().unwrap();
    let _foreign_two = other_tx.input::<String>("foreign_two").unwrap();
    let _foreign_three = other_tx.input::<String>("foreign_three").unwrap();
    let unknown = other_tx.input::<String>("foreign_four").unwrap();
    other_tx.commit().unwrap();
    drop(other_tx);
    let error = tx
        .derived::<usize>(
            "derived",
            DependencyList::new([known.id(), unknown.id()]).unwrap(),
            |_| Ok(0),
        )
        .unwrap_err();

    assert_eq!(error, GraphError::UnknownNode(unknown.id()));
}

#[test]
fn dependency_validation_handles_dense_shared_diamond_paths() {
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let input = tx.input::<usize>("input").unwrap();
    tx.set_input(input, 1).unwrap();

    let mut previous = input.id();
    for layer in 0..22 {
        let left = tx
            .derived::<usize>(
                format!("left-{layer}"),
                DependencyList::new([previous]).unwrap(),
                |_| Ok(1),
            )
            .unwrap();
        let right = tx
            .derived::<usize>(
                format!("right-{layer}"),
                DependencyList::new([previous]).unwrap(),
                |_| Ok(1),
            )
            .unwrap();
        let join = tx
            .derived::<usize>(
                format!("join-{layer}"),
                DependencyList::new([left.id(), right.id()]).unwrap(),
                |_| Ok(1),
            )
            .unwrap();
        previous = join.id();
    }

    tx.derived::<usize>("consumer", DependencyList::new([previous]).unwrap(), |_| {
        Ok(1)
    })
    .unwrap();
    tx.commit().unwrap();
}
