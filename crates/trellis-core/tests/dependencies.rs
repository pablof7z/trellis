use trellis_core::{DependencyList, Graph, GraphError, NodeKind};

#[test]
fn dependencies_are_inspectable_and_ordered() {
    let mut graph = Graph::new();

    let input = graph.input::<String>("input");
    let derived = graph
        .derived::<usize>("derived", DependencyList::new([input.id()]).unwrap())
        .unwrap();
    let collection = graph
        .collection::<String, usize>(
            "collection",
            DependencyList::new([input.id(), derived.id()]).unwrap(),
        )
        .unwrap();

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
    let input = graph.input::<String>("input");

    let error = DependencyList::new([input.id(), input.id()]).unwrap_err();

    assert_eq!(error, GraphError::DuplicateDependency(input.id()));
}

#[test]
fn graph_rejects_unknown_dependency() {
    let mut other_graph = Graph::new();
    let unknown = other_graph.input::<String>("foreign");

    let mut graph = Graph::new();
    let error = graph
        .derived::<usize>(
            "derived",
            DependencyList::new([unknown.id()]).expect("foreign id is still typed"),
        )
        .unwrap_err();

    assert_eq!(error, GraphError::SelfDependency(unknown.id()));

    let known = graph.input::<String>("known");
    let _foreign_two = other_graph.input::<String>("foreign_two");
    let unknown = other_graph.input::<String>("foreign_three");
    let error = graph
        .derived::<usize>(
            "derived",
            DependencyList::new([known.id(), unknown.id()]).unwrap(),
        )
        .unwrap_err();

    assert_eq!(error, GraphError::UnknownNode(unknown.id()));
}
