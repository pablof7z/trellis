use trellis_core::{Graph, GraphError};

#[test]
fn scopes_can_be_created_and_inspected() {
    let mut graph = Graph::new();

    let mut tx = graph.begin_transaction().unwrap();
    let root = tx.create_scope("root").unwrap();
    let child = tx
        .create_scope_with_parent("child", Some(root))
        .expect("parent exists");
    tx.commit().unwrap();
    drop(tx);

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
    let mut other_tx = other_graph.begin_transaction().unwrap();
    let unknown_parent = other_tx.create_scope("foreign").unwrap();
    other_tx.commit().unwrap();
    drop(other_tx);
    let mut graph = Graph::new();

    let mut tx = graph.begin_transaction().unwrap();
    let error = tx
        .create_scope_with_parent("child", Some(unknown_parent))
        .unwrap_err();

    assert_eq!(error, GraphError::UnknownScope(unknown_parent));
}

#[test]
fn nodes_can_be_attached_to_scopes() {
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let node = tx.input::<String>("input").unwrap();
    let scope = tx.create_scope("scope").unwrap();
    tx.commit().unwrap();
    drop(tx);

    assert_eq!(graph.node_meta(node).unwrap().owning_scope(), None);

    let mut tx = graph.begin_transaction().unwrap();
    tx.attach_node_to_scope(node, scope).unwrap();
    tx.commit().unwrap();
    drop(tx);

    assert_eq!(graph.node_meta(node).unwrap().owning_scope(), Some(scope));
}

#[test]
fn attaching_to_unknown_scope_is_rejected() {
    let mut other_graph = Graph::new();
    let mut other_tx = other_graph.begin_transaction().unwrap();
    let unknown_scope = other_tx.create_scope("foreign").unwrap();
    other_tx.commit().unwrap();
    drop(other_tx);

    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let node = tx.input::<String>("input").unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    let error = tx.attach_node_to_scope(node, unknown_scope).unwrap_err();

    assert_eq!(error, GraphError::UnknownScope(unknown_scope));
}

#[test]
fn attaching_unknown_node_is_rejected() {
    let mut other_graph = Graph::new();
    let mut other_tx = other_graph.begin_transaction().unwrap();
    let unknown_node = other_tx.input::<String>("foreign").unwrap();
    other_tx.commit().unwrap();
    drop(other_tx);

    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    let error = tx.attach_node_to_scope(unknown_node, scope).unwrap_err();

    assert_eq!(error, GraphError::UnknownNode(unknown_node.id()));
}

#[test]
fn node_can_only_be_attached_once() {
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let node = tx.input::<String>("input").unwrap();
    let first_scope = tx.create_scope("first").unwrap();
    let second_scope = tx.create_scope("second").unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.attach_node_to_scope(node, first_scope).unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    let error = tx.attach_node_to_scope(node, second_scope).unwrap_err();

    assert_eq!(error, GraphError::NodeAlreadyAttached(node.id()));
}
