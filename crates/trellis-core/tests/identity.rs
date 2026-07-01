use trellis_core::{Graph, InputNode, NodeKind};

#[test]
fn node_identities_are_stable() {
    let mut graph = Graph::new();

    let mut tx = graph.begin_transaction().unwrap();
    let first = tx.input::<String>("first").unwrap();
    let second = tx.input::<u64>("second").unwrap();
    let first_id = first.id();
    let second_id = second.id();
    let _third = tx.input::<bool>("third").unwrap();
    tx.commit().unwrap();
    drop(tx);

    assert_ne!(first_id, second_id);
    assert_eq!(graph.node_meta(first).unwrap().id(), first_id);
    assert_eq!(graph.node_meta(second).unwrap().id(), second_id);
    assert_eq!(graph.node_meta(first).unwrap().kind(), NodeKind::Input);
    assert_eq!(graph.node_meta(first).unwrap().created_revision().get(), 1);
    assert_eq!(
        graph
            .node_meta(first)
            .unwrap()
            .last_changed_revision()
            .get(),
        1
    );
}

#[test]
fn duplicate_debug_names_do_not_define_identity() {
    let mut graph = Graph::new();

    let mut tx = graph.begin_transaction().unwrap();
    let first = tx.input::<String>("same_name").unwrap();
    let second = tx.input::<String>("same_name").unwrap();
    tx.commit().unwrap();
    drop(tx);

    assert_ne!(first.id(), second.id());
    assert_eq!(graph.node_meta(first).unwrap().debug_name(), "same_name");
    assert_eq!(graph.node_meta(second).unwrap().debug_name(), "same_name");
}

#[test]
fn typed_handles_carry_distinct_value_types() {
    fn expects_string(_: InputNode<String>) {}
    fn expects_u64(_: InputNode<u64>) {}

    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let string_node = tx.input::<String>("string").unwrap();
    let u64_node = tx.input::<u64>("number").unwrap();
    tx.commit().unwrap();

    expects_string(string_node);
    expects_u64(u64_node);
}
