use std::collections::BTreeSet;

use trellis_adapter::RecordingAdapter;
use trellis_core::{
    DependencyList, Graph, OutputFrame, OutputFrameKind, ResourceCommand, ResourceKey,
    ResourcePlan, TransactionResult,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Command {
    Open(u8),
}

type ResultLog = Vec<TransactionResult<Command>>;
type TestAdapter = RecordingAdapter<Command>;

fn members(values: &[u8]) -> BTreeSet<u8> {
    values.iter().copied().collect()
}

fn key(value: u8) -> ResourceKey {
    ResourceKey::new(format!("n:{value}"))
}

fn build_graph() -> (Graph<Command>, trellis_core::InputNode<BTreeSet<u8>>) {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let source = tx.input::<BTreeSet<u8>>("source").unwrap();
    tx.set_input(source, BTreeSet::new()).unwrap();
    let collection = tx
        .set_collection(
            "resources",
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(source)?.clone()),
        )
        .unwrap();
    tx.set_resource_planner(collection, scope, move |ctx| {
        let mut plan = ResourcePlan::new();
        for added in &ctx.diff().added {
            plan.open(key(added.value), ctx.scope(), Command::Open(added.value));
        }
        for removed in &ctx.diff().removed {
            plan.close(key(removed.value), ctx.scope());
        }
        Ok(plan)
    })
    .unwrap();
    tx.materialized_output(
        "output",
        scope,
        DependencyList::new([collection.id()]).unwrap(),
        move |ctx| Ok(ctx.set_collection(collection)?.clone()),
    )
    .unwrap();
    tx.commit().unwrap();
    drop(tx);
    (graph, source)
}

fn commit_members(
    graph: &mut Graph<Command>,
    source: trellis_core::InputNode<BTreeSet<u8>>,
    values: &[u8],
) -> TransactionResult<Command> {
    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(source, members(values)).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);
    result
}

fn run_sequence(use_adapter: bool) -> (ResultLog, Option<TestAdapter>) {
    let (mut graph, source) = build_graph();
    let mut adapter = use_adapter.then(RecordingAdapter::default);
    let mut results = Vec::new();

    for values in [&[1, 2][..], &[2][..]] {
        let result = commit_members(&mut graph, source, values);
        if let Some(adapter) = &mut adapter {
            let receipt = adapter.apply_transaction(result.clone()).unwrap();
            assert_eq!(receipt.trace, result.trace());
        }
        results.push(result);
    }

    (results, adapter)
}

#[test]
fn adapter_does_not_change_full_transaction_results() {
    let (plain_results, _) = run_sequence(false);
    let (adapted_results, _) = run_sequence(true);

    assert_eq!(plain_results, adapted_results);
    assert!(plain_results.iter().any(|result| {
        result
            .resource_plan
            .commands()
            .iter()
            .any(|command| matches!(command, ResourceCommand::Open { .. }))
    }));
    assert!(plain_results.iter().any(|result| {
        result
            .resource_plan
            .commands()
            .iter()
            .any(|command| matches!(command, ResourceCommand::Close { .. }))
    }));
    assert!(plain_results.iter().any(|result| {
        result
            .output_frames
            .iter()
            .any(|frame| matches!(frame.kind, OutputFrameKind::Delta(_)))
    }));
}

#[test]
fn recording_adapter_consumes_the_cloned_result_payloads() {
    let (results, adapter) = run_sequence(true);
    let adapter = adapter.unwrap();
    let expected_commands = results
        .iter()
        .flat_map(|result| result.resource_plan.commands().iter().cloned())
        .collect::<Vec<_>>();
    let expected_frames = results
        .iter()
        .flat_map(|result| result.output_frames.iter().cloned())
        .collect::<Vec<OutputFrame>>();

    assert_eq!(adapter.resource_sink().commands(), expected_commands);
    assert_eq!(adapter.output_sink().frames(), expected_frames);
}
