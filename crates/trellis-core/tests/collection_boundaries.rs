use std::collections::{BTreeMap, BTreeSet};

use trellis_core::{DependencyList, DeriveError, Graph, GraphError, Removed};

fn map(entries: &[(&str, u64)]) -> BTreeMap<String, u64> {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_owned(), *value))
        .collect()
}

fn set(entries: &[&str]) -> BTreeSet<String> {
    entries.iter().map(|value| (*value).to_owned()).collect()
}

#[test]
fn scalar_derived_node_cannot_depend_on_collection() {
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let collection = tx
        .set_collection("members", DependencyList::empty(), |_| Ok(set(&["a"])))
        .unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    let error = tx
        .derived::<usize>(
            "count",
            DependencyList::new([collection.id()]).unwrap(),
            |_| Ok(1),
        )
        .unwrap_err();

    assert_eq!(
        error,
        GraphError::CollectionDependencyNotAllowed(collection.id())
    );
}

#[test]
fn set_and_unit_map_shapes_are_type_distinct() {
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let set_node = tx
        .set_collection("set", DependencyList::empty(), |_| Ok(set(&["a"])))
        .unwrap();
    let map_node = tx
        .collection::<String, ()>("map", DependencyList::empty(), |_| {
            Ok([("a".to_owned(), ())].into_iter().collect())
        })
        .unwrap();
    tx.commit().unwrap();
    drop(tx);

    assert_eq!(
        graph.map_collection(set_node).unwrap_err(),
        GraphError::WrongCollectionType(set_node.id())
    );
    assert_eq!(
        graph.set_collection(map_node).unwrap_err(),
        GraphError::WrongCollectionType(map_node.id())
    );
}

#[test]
fn collection_context_reports_wrong_collection_shape() {
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let set_node = tx
        .set_collection("set", DependencyList::empty(), |_| Ok(set(&["a"])))
        .unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    let bad_reader = tx
        .collection::<String, ()>(
            "bad_reader",
            DependencyList::new([set_node.id()]).unwrap(),
            move |ctx| Ok(ctx.map_collection(set_node)?.clone()),
        )
        .unwrap();
    let error = tx.commit().unwrap_err();
    drop(tx);

    assert_eq!(
        error,
        GraphError::CollectionFailed(
            bad_reader.id(),
            DeriveError::WrongCollectionType(set_node.id())
        )
    );
}

#[test]
fn unrelated_transaction_clears_collection_diff_without_stale_previous_state() {
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let source = tx.input::<BTreeMap<String, u64>>("source").unwrap();
    let unrelated = tx.input::<u64>("unrelated").unwrap();
    tx.set_input(source, map(&[("a", 1)])).unwrap();
    tx.set_input(unrelated, 0).unwrap();
    let collection = tx
        .collection(
            "items",
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(source)?.clone()),
        )
        .unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(source, map(&[("a", 1), ("b", 2)])).unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(unrelated, 1).unwrap();
    tx.commit().unwrap();
    drop(tx);

    assert!(graph.map_diff(collection).unwrap().is_none());

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(source, map(&[("a", 1)])).unwrap();
    tx.commit().unwrap();
    drop(tx);
    let diff = graph.map_diff(collection).unwrap().unwrap();

    assert_eq!(
        diff.removed,
        vec![Removed {
            value: ("b".to_owned(), 2)
        }]
    );
}
