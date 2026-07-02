use std::collections::BTreeSet;

use trellis_core::{DependencyList, Graph, ResourceCommand, ResourceKey};

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
fn open_close_planner_closes_removed_set_members() {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let source = tx.input::<BTreeSet<String>>("source").unwrap();
    tx.set_input(source, set(&["a", "b"])).unwrap();
    let collection = tx
        .set_collection(
            "resources",
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(source)?.clone()),
        )
        .unwrap();
    tx.open_close_planner(
        collection,
        scope,
        |value| key(value),
        |value| Command::Open(value.clone()),
    )
    .unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(source, set(&["a"])).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert_eq!(
        result.resource_plan.commands(),
        &[ResourceCommand::Close {
            key: key("b"),
            scope,
        }]
    );
    assert!(graph.resource_owners(&key("b")).is_none());
}
