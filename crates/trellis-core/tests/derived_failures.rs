use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use std::collections::BTreeSet;

use trellis_core::{
    DependencyList, DeriveError, FullRecomputeOutputMismatch, FullRecomputeResourceMismatch, Graph,
    GraphError, ResourceKey, ResourcePlan,
};

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
    let runs = Arc::new(AtomicUsize::new(0));
    let runs_for_derive = Arc::clone(&runs);

    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let derived = tx
        .derived::<u64>("nondeterministic", DependencyList::empty(), move |_| {
            let next = runs_for_derive.fetch_add(1, Ordering::Relaxed) + 1;
            Ok(next as u64)
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

#[test]
fn full_recompute_resource_mismatch_names_resource_key() {
    let runs = Arc::new(AtomicUsize::new(0));
    let runs_for_planner = Arc::clone(&runs);

    let mut graph = Graph::<String>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let collection = tx
        .set_collection("resources", DependencyList::empty(), |_| {
            Ok(BTreeSet::from(["member".to_owned()]))
        })
        .unwrap();
    tx.set_resource_planner(collection, scope, move |ctx| {
        let run = runs_for_planner.fetch_add(1, Ordering::Relaxed) + 1;
        let mut plan = ResourcePlan::new();
        for added in &ctx.diff().added {
            let key = ResourceKey::new(format!("{}-{run}", added.value));
            plan.open(key, ctx.scope(), added.value.clone());
        }
        Ok(plan)
    })
    .unwrap();
    tx.commit().unwrap();
    drop(tx);

    assert_eq!(
        graph.full_recompute_check().unwrap_err(),
        GraphError::FullRecomputeResourceMismatch(FullRecomputeResourceMismatch {
            key: ResourceKey::new("member-1"),
            incremental_owners: vec![scope],
            recomputed_owners: Vec::new(),
        })
    );
}

#[test]
fn full_recompute_output_mismatch_names_output_key() {
    let runs = Arc::new(AtomicUsize::new(0));
    let runs_for_output = Arc::clone(&runs);

    let mut graph = Graph::<()>::new();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let output = tx
        .materialized_output("output", scope, DependencyList::empty(), move |_| {
            let next = runs_for_output.fetch_add(1, Ordering::Relaxed) + 1;
            Ok(next as u64)
        })
        .unwrap();
    tx.commit().unwrap();
    drop(tx);

    assert_eq!(
        graph.full_recompute_check().unwrap_err(),
        GraphError::FullRecomputeOutputMismatch(FullRecomputeOutputMismatch {
            key: output.key(),
            incremental_present: true,
            recomputed_present: true,
        })
    );
}
