use std::cell::Cell;
use std::rc::Rc;

use trellis_core::{DependencyList, DeriveError, Graph, GraphError};

#[test]
fn undeclared_dependency_read_fails_transaction() {
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let declared = tx.input::<u64>("declared").unwrap();
    let undeclared = tx.input::<u64>("undeclared").unwrap();
    tx.set_input(declared, 1).unwrap();
    tx.set_input(undeclared, 2).unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    let derived = tx
        .derived::<u64>(
            "bad_read",
            DependencyList::new([declared.id()]).unwrap(),
            move |ctx| Ok(*ctx.input(undeclared)?),
        )
        .unwrap();
    let derived_id = derived.id();
    let tx_id = tx.id();

    assert_eq!(
        tx.commit().unwrap_err(),
        GraphError::DeriveFailed(
            derived_id,
            DeriveError::UndeclaredDependency(undeclared.id())
        )
    );
    assert_eq!(
        tx.commit().unwrap_err(),
        GraphError::TransactionClosed(tx_id)
    );
    drop(tx);

    assert!(graph.node_meta_by_id(derived_id).is_none());
}

#[test]
fn full_recompute_check_detects_mismatch() {
    let runs = Rc::new(Cell::new(0));
    let runs_for_derive = Rc::clone(&runs);

    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let derived = tx
        .derived::<u64>("nondeterministic", DependencyList::empty(), move |_| {
            let next = runs_for_derive.get() + 1;
            runs_for_derive.set(next);
            Ok(next)
        })
        .unwrap();
    tx.commit().unwrap();
    drop(tx);

    assert_eq!(graph.derived_value(derived).unwrap(), Some(&1));
    assert_eq!(
        graph.full_recompute_check().unwrap_err(),
        GraphError::FullRecomputeMismatch(derived.id())
    );
    assert_eq!(graph.derived_value(derived).unwrap(), Some(&1));
}
