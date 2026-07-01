use std::collections::{BTreeMap, BTreeSet};

use trellis_core::{Added, DependencyList, Graph, Removed, Unchanged, Updated};

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
fn map_collection_detects_added_removed_updated_and_unchanged() {
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let source = tx.input::<BTreeMap<String, u64>>("source").unwrap();
    tx.set_input(source, map(&[("b", 2), ("a", 1)])).unwrap();
    let collection = tx
        .collection(
            "items",
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(source)?.clone()),
        )
        .unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert_eq!(result.changed_collection_nodes, vec![collection.id()]);
    assert_eq!(
        graph.map_diff(collection).unwrap().unwrap().added,
        vec![
            Added {
                value: ("a".to_owned(), 1)
            },
            Added {
                value: ("b".to_owned(), 2)
            },
        ]
    );

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(source, map(&[("a", 1), ("b", 3), ("c", 4)]))
        .unwrap();
    tx.commit().unwrap();
    drop(tx);
    let diff = graph.map_diff(collection).unwrap().unwrap();

    assert_eq!(
        diff.updated,
        vec![Updated {
            key: "b".to_owned(),
            previous: 2,
            current: 3
        }]
    );
    assert_eq!(
        diff.unchanged,
        vec![Unchanged {
            value: ("a".to_owned(), 1)
        }]
    );
    assert_eq!(
        diff.added,
        vec![Added {
            value: ("c".to_owned(), 4)
        }]
    );

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(source, map(&[("a", 1)])).unwrap();
    tx.commit().unwrap();
    drop(tx);
    let diff = graph.map_diff(collection).unwrap().unwrap();

    assert_eq!(
        diff.removed,
        vec![
            Removed {
                value: ("b".to_owned(), 3)
            },
            Removed {
                value: ("c".to_owned(), 4)
            },
        ]
    );
}

#[test]
fn set_collection_detects_structural_diff_and_empty_source() {
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let source = tx.input::<BTreeSet<String>>("source").unwrap();
    tx.set_input(source, set(&["c", "a", "b"])).unwrap();
    let collection = tx
        .set_collection(
            "members",
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(source)?.clone()),
        )
        .unwrap();
    tx.commit().unwrap();
    drop(tx);

    assert_eq!(
        graph.set_diff(collection).unwrap().unwrap().added,
        vec![
            Added {
                value: "a".to_owned()
            },
            Added {
                value: "b".to_owned()
            },
            Added {
                value: "c".to_owned()
            },
        ]
    );

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(source, BTreeSet::new()).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);
    let diff = graph.set_diff(collection).unwrap().unwrap();

    assert_eq!(result.changed_collection_nodes, vec![collection.id()]);
    assert!(diff.added.is_empty());
    assert_eq!(
        diff.removed,
        vec![
            Removed {
                value: "a".to_owned()
            },
            Removed {
                value: "b".to_owned()
            },
            Removed {
                value: "c".to_owned()
            },
        ]
    );
    assert!(
        graph
            .set_collection(collection)
            .unwrap()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn equal_collection_result_produces_empty_diff_and_does_not_propagate() {
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let source = tx.input::<u64>("source").unwrap();
    tx.set_input(source, 1).unwrap();
    let collection = tx
        .set_collection(
            "parity",
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| {
                let label = if *ctx.input(source)? % 2 == 0 {
                    "even"
                } else {
                    "odd"
                };
                Ok(set(&[label]))
            },
        )
        .unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(source, 3).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);
    let diff = graph.set_diff(collection).unwrap().unwrap();

    assert!(result.changed_collection_nodes.is_empty());
    assert!(diff.is_empty());
    assert_eq!(
        diff.unchanged,
        vec![Unchanged {
            value: "odd".to_owned()
        }]
    );
}

#[test]
fn collection_can_depend_on_collection_in_stable_order() {
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let source = tx.input::<BTreeSet<String>>("source").unwrap();
    tx.set_input(source, set(&["bb", "a"])).unwrap();
    let base = tx
        .set_collection(
            "base",
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(source)?.clone()),
        )
        .unwrap();
    let lengths = tx
        .collection(
            "lengths",
            DependencyList::new([base.id()]).unwrap(),
            move |ctx| {
                Ok(ctx
                    .set_collection(base)?
                    .iter()
                    .map(|value| (value.clone(), value.len()))
                    .collect())
            },
        )
        .unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert_eq!(
        result.changed_collection_nodes,
        vec![base.id(), lengths.id()]
    );
    assert_eq!(graph.map_collection(lengths).unwrap().unwrap()["bb"], 2);
}

#[test]
fn large_collection_diff_is_deterministic() {
    let initial: BTreeMap<u64, u64> = (0..10_000).map(|value| (value, value)).collect();
    let next: BTreeMap<u64, u64> = (5_000..15_000).map(|value| (value, value + 1)).collect();

    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let source = tx.input::<BTreeMap<u64, u64>>("source").unwrap();
    tx.set_input(source, initial).unwrap();
    let collection = tx
        .collection(
            "large",
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(source)?.clone()),
        )
        .unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(source, next).unwrap();
    tx.commit().unwrap();
    drop(tx);
    let diff = graph.map_diff(collection).unwrap().unwrap();

    assert_eq!(diff.removed.len(), 5_000);
    assert_eq!(diff.updated.len(), 5_000);
    assert_eq!(diff.added.len(), 5_000);
    assert_eq!(diff.removed.first().unwrap().value, (0, 0));
    assert_eq!(diff.updated.first().unwrap().key, 5_000);
    assert_eq!(diff.added.first().unwrap().value, (10_000, 10_001));
}

#[test]
fn full_recompute_includes_collections() {
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let source = tx.input::<BTreeSet<String>>("source").unwrap();
    tx.set_input(source, set(&["a"])).unwrap();
    let collection = tx
        .set_collection(
            "members",
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(source)?.clone()),
        )
        .unwrap();
    tx.commit().unwrap();
    drop(tx);

    let check = graph.full_recompute_check().unwrap();

    assert!(check.checked_derived.is_empty());
    assert_eq!(check.checked_collections, vec![collection.id()]);
}
