use std::cell::Cell;
use std::rc::Rc;

use trellis_core::{DependencyList, DeriveError, Graph, GraphError};

#[test]
fn derived_node_recomputes_when_input_changes() {
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let input = tx.input::<String>("name").unwrap();
    tx.set_input(input, "trellis".to_owned()).unwrap();
    let len = tx
        .derived::<usize>(
            "name_len",
            DependencyList::new([input.id()]).unwrap(),
            move |ctx| Ok(ctx.input(input)?.len()),
        )
        .unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert_eq!(result.changed_derived_nodes, vec![len.id()]);
    assert_eq!(graph.derived_value(len).unwrap(), Some(&7));

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(input, "graph".to_owned()).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert_eq!(result.changed_inputs, vec![input.id()]);
    assert_eq!(result.changed_derived_nodes, vec![len.id()]);
    assert_eq!(graph.derived_value(len).unwrap(), Some(&5));
}

#[test]
fn unaffected_derived_node_does_not_recompute() {
    let left_runs = Rc::new(Cell::new(0));
    let right_runs = Rc::new(Cell::new(0));

    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let left_input = tx.input::<u64>("left_input").unwrap();
    let right_input = tx.input::<u64>("right_input").unwrap();
    tx.set_input(left_input, 1).unwrap();
    tx.set_input(right_input, 10).unwrap();

    let left_runs_for_derive = Rc::clone(&left_runs);
    let left = tx
        .derived::<u64>(
            "left",
            DependencyList::new([left_input.id()]).unwrap(),
            move |ctx| {
                left_runs_for_derive.set(left_runs_for_derive.get() + 1);
                Ok(*ctx.input(left_input)? + 1)
            },
        )
        .unwrap();

    let right_runs_for_derive = Rc::clone(&right_runs);
    let right = tx
        .derived::<u64>(
            "right",
            DependencyList::new([right_input.id()]).unwrap(),
            move |ctx| {
                right_runs_for_derive.set(right_runs_for_derive.get() + 1);
                Ok(*ctx.input(right_input)? + 1)
            },
        )
        .unwrap();
    tx.commit().unwrap();
    drop(tx);

    assert_eq!(left_runs.get(), 1);
    assert_eq!(right_runs.get(), 1);

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(left_input, 2).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert_eq!(result.changed_derived_nodes, vec![left.id()]);
    assert_eq!(graph.derived_value(left).unwrap(), Some(&3));
    assert_eq!(graph.derived_value(right).unwrap(), Some(&11));
    assert_eq!(left_runs.get(), 2);
    assert_eq!(right_runs.get(), 1);
}

#[test]
fn derived_node_can_depend_on_another_derived_node() {
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let input = tx.input::<u64>("input").unwrap();
    tx.set_input(input, 2).unwrap();
    let doubled = tx
        .derived::<u64>(
            "doubled",
            DependencyList::new([input.id()]).unwrap(),
            move |ctx| Ok(*ctx.input(input)? * 2),
        )
        .unwrap();
    let plus_one = tx
        .derived::<u64>(
            "plus_one",
            DependencyList::new([doubled.id()]).unwrap(),
            move |ctx| Ok(*ctx.derived(doubled)? + 1),
        )
        .unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert_eq!(
        result.changed_derived_nodes,
        vec![doubled.id(), plus_one.id()]
    );
    assert_eq!(graph.derived_value(plus_one).unwrap(), Some(&5));
}

#[test]
fn derived_self_cycle_is_rejected() {
    let mut other_graph = Graph::new();
    let mut other_tx = other_graph.begin_transaction().unwrap();
    let foreign = other_tx.input::<u64>("foreign").unwrap();
    other_tx.commit().unwrap();
    drop(other_tx);

    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let error = tx
        .derived::<u64>(
            "self_cycle",
            DependencyList::new([foreign.id()]).unwrap(),
            |_| Ok(0),
        )
        .unwrap_err();

    assert_eq!(error, GraphError::SelfDependency(foreign.id()));
}

#[test]
fn equal_recompute_does_not_propagate_by_default() {
    let downstream_runs = Rc::new(Cell::new(0));

    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let input = tx.input::<u64>("input").unwrap();
    tx.set_input(input, 1).unwrap();
    let parity = tx
        .derived::<u64>(
            "parity",
            DependencyList::new([input.id()]).unwrap(),
            move |ctx| Ok(*ctx.input(input)? % 2),
        )
        .unwrap();
    let downstream_runs_for_derive = Rc::clone(&downstream_runs);
    let downstream = tx
        .derived::<String>(
            "downstream",
            DependencyList::new([parity.id()]).unwrap(),
            move |ctx| {
                downstream_runs_for_derive.set(downstream_runs_for_derive.get() + 1);
                Ok(format!("parity={}", ctx.derived(parity)?))
            },
        )
        .unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(input, 3).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert!(result.changed_inputs.contains(&input.id()));
    assert!(result.changed_derived_nodes.is_empty());
    assert_eq!(
        graph.derived_value(downstream).unwrap(),
        Some(&"parity=1".to_owned())
    );
    assert_eq!(downstream_runs.get(), 1);
}

#[test]
fn derive_error_does_not_corrupt_committed_value() {
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let input = tx.input::<u64>("input").unwrap();
    tx.set_input(input, 1).unwrap();
    let derived = tx
        .derived::<u64>(
            "nonzero",
            DependencyList::new([input.id()]).unwrap(),
            move |ctx| {
                let value = *ctx.input(input)?;
                if value == 0 {
                    Err(DeriveError::message("zero"))
                } else {
                    Ok(value)
                }
            },
        )
        .unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(input, 0).unwrap();
    let tx_id = tx.id();
    let error = tx.commit().unwrap_err();
    assert_eq!(
        tx.commit().unwrap_err(),
        GraphError::TransactionClosed(tx_id)
    );
    drop(tx);

    assert_eq!(
        error,
        GraphError::DeriveFailed(derived.id(), DeriveError::message("zero"))
    );
    assert_eq!(graph.input_value(input).unwrap(), Some(&1));
    assert_eq!(graph.derived_value(derived).unwrap(), Some(&1));
}

#[test]
fn full_recompute_matches_incremental_state() {
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let input = tx.input::<u64>("input").unwrap();
    tx.set_input(input, 4).unwrap();
    let doubled = tx
        .derived::<u64>(
            "doubled",
            DependencyList::new([input.id()]).unwrap(),
            move |ctx| Ok(*ctx.input(input)? * 2),
        )
        .unwrap();
    tx.commit().unwrap();
    drop(tx);

    let check = graph.full_recompute_check().unwrap();

    assert_eq!(check.checked_derived, vec![doubled.id()]);
}
