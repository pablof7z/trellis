use trellis_core::{Graph, InputNode, NodeKind};

#[test]
fn node_identities_are_stable() {
    let mut graph = Graph::new();

    let first = graph.input::<String>("first");
    let second = graph.input::<u64>("second");
    let first_id = first.id();
    let second_id = second.id();

    let _third = graph.input::<bool>("third");

    assert_ne!(first_id, second_id);
    assert_eq!(graph.node_meta(first).unwrap().id(), first_id);
    assert_eq!(graph.node_meta(second).unwrap().id(), second_id);
    assert_eq!(graph.node_meta(first).unwrap().kind(), NodeKind::Input);
}

#[test]
fn duplicate_debug_names_do_not_define_identity() {
    let mut graph = Graph::new();

    let first = graph.input::<String>("same_name");
    let second = graph.input::<String>("same_name");

    assert_ne!(first.id(), second.id());
    assert_eq!(graph.node_meta(first).unwrap().debug_name(), "same_name");
    assert_eq!(graph.node_meta(second).unwrap().debug_name(), "same_name");
}

#[test]
fn typed_handles_carry_distinct_value_types() {
    fn expects_string(_: InputNode<String>) {}
    fn expects_u64(_: InputNode<u64>) {}

    let mut graph = Graph::new();
    let string_node = graph.input::<String>("string");
    let u64_node = graph.input::<u64>("number");

    expects_string(string_node);
    expects_u64(u64_node);
}
