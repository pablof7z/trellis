use std::collections::BTreeSet;

use trellis_core::{
    DependencyList, Graph, OutputError, ResourceCommand, ResourceKey, ResourcePlan,
    TransactionPhase,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Command {
    Open(String),
}

fn key(value: &str) -> ResourceKey {
    ResourceKey::new(value.to_owned())
}

fn expected_phase_trace() -> Vec<TransactionPhase> {
    vec![
        TransactionPhase::StageOperations,
        TransactionPhase::ValidateTransaction,
        TransactionPhase::CommitCanonicalInputs,
        TransactionPhase::MarkDirtyNodes,
        TransactionPhase::RecomputeDerivedNodes,
        TransactionPhase::RecomputeCollectionNodes,
        TransactionPhase::ComputeStructuralDiffs,
        TransactionPhase::ResolveScopeLifecycle,
        TransactionPhase::ProduceResourcePlans,
        TransactionPhase::ProduceOutputFrames,
        TransactionPhase::CommitGraphRevision,
        TransactionPhase::ReturnTransactionResult,
    ]
}

#[test]
fn same_input_sequence_produces_same_phase_trace() {
    fn run_sequence() -> Vec<TransactionPhase> {
        let mut graph = Graph::new();
        let mut tx = graph.begin_transaction().unwrap();
        let input = tx.input::<u64>("input").unwrap();
        tx.commit().unwrap();
        drop(tx);

        let mut tx = graph.begin_transaction().unwrap();
        tx.set_input(input, 1).unwrap();
        tx.commit().unwrap().phase_trace
    }

    assert_eq!(run_sequence(), expected_phase_trace());
    assert_eq!(run_sequence(), run_sequence());
}

#[test]
fn resource_plans_and_outputs_see_final_derived_state() {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let source = tx.input::<String>("source").unwrap();
    tx.set_input(source, "first".to_owned()).unwrap();
    let derived = tx
        .derived(
            "derived",
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(format!("final:{}", ctx.input(source)?)),
        )
        .unwrap();
    let collection = tx
        .set_collection(
            "resources",
            DependencyList::new([derived.id()]).unwrap(),
            move |ctx| Ok(BTreeSet::from([ctx.derived(derived)?.clone()])),
        )
        .unwrap();
    tx.set_resource_planner(collection, scope, move |ctx| {
        let mut plan = ResourcePlan::new();
        for added in &ctx.diff().added {
            plan.open(
                key(&added.value),
                ctx.scope(),
                Command::Open(added.value.clone()),
            );
        }
        Ok(plan)
    })
    .unwrap();
    tx.materialized_output(
        "output",
        scope,
        DependencyList::new([derived.id()]).unwrap(),
        move |ctx| Ok(ctx.derived(derived)?.clone()),
    )
    .unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert_eq!(
        result.resource_plan.commands(),
        &[ResourceCommand::Open {
            key: key("final:first"),
            scope,
            command: Command::Open("final:first".to_owned()),
        }]
    );
    assert_eq!(
        result.output_frames[0].kind.payload::<String>(),
        Some(&"final:first".to_owned())
    );
}

#[test]
fn failed_transaction_emits_no_partial_plans_or_frames() {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let source = tx.input::<String>("source").unwrap();
    tx.set_input(source, "ok".to_owned()).unwrap();
    let output = tx
        .materialized_output::<String>(
            "output",
            scope,
            DependencyList::new([source.id()]).unwrap(),
            move |_| Err(OutputError::message("output failed")),
        )
        .unwrap();
    let error = tx.commit().unwrap_err();
    drop(tx);

    assert_eq!(
        error,
        trellis_core::GraphError::OutputFailed(output.key(), OutputError::message("output failed"),)
    );
    assert_eq!(graph.revision().get(), 0);
    assert!(graph.output_meta(output.key()).is_none());
}
