use trellis_core::{Graph, GraphError};

#[test]
fn scopes_can_be_created_and_inspected() {
    let mut graph = Graph::new();

    let root = graph.create_scope("root");
    let child = graph
        .create_scope_with_parent("child", Some(root))
        .expect("parent exists");

    let root_meta = graph.scope_meta(root).unwrap();
    let child_meta = graph.scope_meta(child).unwrap();

    assert_eq!(root_meta.id(), root);
    assert_eq!(root_meta.debug_name(), "root");
    assert_eq!(root_meta.parent(), None);
    assert!(!root_meta.is_closed());

    assert_eq!(child_meta.id(), child);
    assert_eq!(child_meta.debug_name(), "child");
    assert_eq!(child_meta.parent(), Some(root));
}

#[test]
fn scope_parent_must_exist() {
    let mut other_graph = Graph::new();
    let unknown_parent = other_graph.create_scope("foreign");
    let mut graph = Graph::new();

    let error = graph
        .create_scope_with_parent("child", Some(unknown_parent))
        .unwrap_err();

    assert_eq!(error, GraphError::UnknownScope(unknown_parent));
}

#[test]
fn nodes_can_be_attached_to_scopes() {
    let mut graph = Graph::new();
    let node = graph.input::<String>("input");
    let scope = graph.create_scope("scope");

    assert_eq!(graph.node_meta(node).unwrap().owning_scope(), None);

    graph.attach_node_to_scope(node, scope).unwrap();

    assert_eq!(graph.node_meta(node).unwrap().owning_scope(), Some(scope));
}

#[test]
fn attaching_to_unknown_scope_is_rejected() {
    let mut other_graph = Graph::new();
    let unknown_scope = other_graph.create_scope("foreign");

    let mut graph = Graph::new();
    let node = graph.input::<String>("input");

    let error = graph.attach_node_to_scope(node, unknown_scope).unwrap_err();

    assert_eq!(error, GraphError::UnknownScope(unknown_scope));
}

#[test]
fn attaching_unknown_node_is_rejected() {
    let mut other_graph = Graph::new();
    let unknown_node = other_graph.input::<String>("foreign");

    let mut graph = Graph::new();
    let scope = graph.create_scope("scope");

    let error = graph.attach_node_to_scope(unknown_node, scope).unwrap_err();

    assert_eq!(error, GraphError::UnknownNode(unknown_node.id()));
}

#[test]
fn node_can_only_be_attached_once() {
    let mut graph = Graph::new();
    let node = graph.input::<String>("input");
    let first_scope = graph.create_scope("first");
    let second_scope = graph.create_scope("second");

    graph.attach_node_to_scope(node, first_scope).unwrap();
    let error = graph.attach_node_to_scope(node, second_scope).unwrap_err();

    assert_eq!(error, GraphError::NodeAlreadyAttached(node.id()));
}
