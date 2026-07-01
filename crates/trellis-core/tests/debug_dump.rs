use trellis_core::{DependencyList, Graph};

#[test]
fn debug_dump_is_deterministic() {
    fn build_graph() -> Graph {
        let mut graph = Graph::new();
        let mut tx = graph.begin_transaction().unwrap();
        let root = tx.create_scope("root").unwrap();
        let workspace = tx
            .create_scope_with_parent("workspace", Some(root))
            .unwrap();
        let active = tx.input::<String>("active_workspace").unwrap();
        let visible = tx
            .derived::<Vec<String>>(
                "visible_projects",
                DependencyList::new([active.id()]).unwrap(),
            )
            .unwrap();
        let windows = tx
            .collection::<String, String>(
                "sync_windows",
                DependencyList::new([active.id(), visible.id()]).unwrap(),
            )
            .unwrap();

        tx.attach_node_to_scope(visible, workspace).unwrap();
        tx.attach_node_to_scope(windows, workspace).unwrap();
        tx.commit().unwrap();
        drop(tx);
        graph
    }

    let first = build_graph().debug_dump();
    let second = build_graph().debug_dump();

    assert_eq!(first, second);
    assert_eq!(
        first,
        concat!(
            "Graph(revision=1)\n",
            "Scopes:\n",
            "  ScopeId(1) name=\"root\" parent=None closed=false\n",
            "  ScopeId(2) name=\"workspace\" parent=Some(ScopeId(1)) closed=false\n",
            "Nodes:\n",
            "  NodeId(1) kind=Input name=\"active_workspace\" scope=None deps=[]\n",
            "  NodeId(2) kind=Derived name=\"visible_projects\" scope=Some(ScopeId(2)) deps=[NodeId(1)]\n",
            "  NodeId(3) kind=Collection name=\"sync_windows\" scope=Some(ScopeId(2)) deps=[NodeId(1), NodeId(2)]\n",
        )
    );
}
