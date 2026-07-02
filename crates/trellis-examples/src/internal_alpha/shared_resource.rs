use std::collections::BTreeSet;

use super::{AlphaCommand, command_closes, key, members};
use trellis_core::{DependencyList, Graph};

#[test]
fn alpha_catches_shared_resource_closing_before_last_owner() {
    let mut graph = Graph::<AlphaCommand, BTreeSet<u8>>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let first = tx.create_scope("first").unwrap();
    let second = tx.create_scope("second").unwrap();
    let source = tx.input::<BTreeSet<u8>>("source").unwrap();
    tx.set_input(source, members(&[9])).unwrap();
    let demand = tx
        .set_collection(
            "demand",
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(source)?.clone()),
        )
        .unwrap();
    for scope in [first, second] {
        tx.open_close_planner(
            demand,
            scope,
            |value| key(*value),
            |value| AlphaCommand::Open(*value),
        )
        .unwrap();
    }
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.close_scope(first).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);
    assert!(!command_closes(&result, 9));
    assert!(
        graph
            .resource_owners(&key(9))
            .is_some_and(|owners| owners.contains(&second))
    );

    let mut tx = graph.begin_transaction().unwrap();
    tx.close_scope(second).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);
    assert!(command_closes(&result, 9));
    assert!(graph.resource_owners(&key(9)).is_none());
    graph.assert_incremental_equals_full().unwrap();
}
