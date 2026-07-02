#![allow(dead_code)]

#[path = "scalar_support.rs"]
mod scalar_support;

use std::collections::BTreeSet;

use trellis_core::{
    DependencyList, Graph, InputNode, MaterializedOutput, ResourceKey, ResourcePlan, ScopeId,
    TransactionTrace, assert_transaction_traces_match,
    testing::{ModelScript, ModelStep, ModelTopology},
};
use trellis_testing::ResourceLedger;

use scalar_support::run_scalar_chain_script;

#[derive(Clone, Debug, Eq, PartialEq)]
enum Command {
    Open(u8),
}

struct Target {
    graph: Graph<Command>,
    source: InputNode<BTreeSet<u8>>,
    output: MaterializedOutput<BTreeSet<u8>>,
    scope: ScopeId,
}

pub(crate) fn run_resource_lifecycle(data: &[u8]) {
    let script = ModelScript {
        topology: ModelTopology::SetResourceOutput,
        steps: steps_from_bytes(data),
    };
    run_set_resource_script(&script);
}

pub(crate) fn run_trace_replay(data: &[u8]) {
    let script = script_from_bytes(data);
    let first = run_script(&script);
    let second = run_script(&script);
    assert_transaction_traces_match(&first, &second).unwrap();
}

fn script_from_bytes(data: &[u8]) -> ModelScript {
    let topology = if data.first().copied().unwrap_or_default().is_multiple_of(2) {
        ModelTopology::ScalarChain
    } else {
        ModelTopology::SetResourceOutput
    };
    ModelScript {
        topology,
        steps: steps_from_bytes(data.get(1..).unwrap_or_default()),
    }
}

fn steps_from_bytes(data: &[u8]) -> Vec<ModelStep> {
    data.iter().copied().take(64).map(step_from_byte).collect()
}

fn step_from_byte(value: u8) -> ModelStep {
    if value.is_multiple_of(13) {
        ModelStep::ClosePrimaryScope
    } else if value.is_multiple_of(7) {
        ModelStep::RebaselineOutput
    } else {
        ModelStep::SetMembers(members(value))
    }
}

fn run_script(script: &ModelScript) -> Vec<TransactionTrace> {
    match script.topology {
        ModelTopology::ScalarChain => run_scalar_chain_script(script),
        ModelTopology::SetResourceOutput => run_set_resource_script(script),
    }
}

fn run_set_resource_script(script: &ModelScript) -> Vec<TransactionTrace> {
    let (mut target, initial) = build_graph(BTreeSet::new());
    let mut ledger = ResourceLedger::new();
    let mut traces = vec![TransactionTrace::from_result(&initial)];
    let mut scope_live = true;
    let mut output_live = true;
    apply_resource_result(&target, &mut ledger, &initial);

    for step in &script.steps {
        let result = match step {
            ModelStep::SetMembers(next) => set_source(&mut target, next.clone()),
            ModelStep::RebaselineOutput if output_live => rebaseline_output(&mut target),
            ModelStep::ClosePrimaryScope if scope_live => {
                scope_live = false;
                output_live = false;
                close_scope(&mut target)
            }
            ModelStep::RebaselineOutput | ModelStep::ClosePrimaryScope => commit_noop(&mut target),
        };
        apply_resource_result(&target, &mut ledger, &result);
        traces.push(TransactionTrace::from_result(&result));
    }
    traces
}

fn apply_resource_result(
    target: &Target,
    ledger: &mut ResourceLedger<Command>,
    result: &trellis_core::TransactionResult<Command>,
) {
    ledger.apply_result(result);
    ledger.assert_no_duplicate_close().unwrap();
    ledger.assert_no_orphan_resources().unwrap();
    ledger
        .assert_graph_has_no_orphan_resources(&target.graph)
        .unwrap();
}

fn build_graph(
    initial: BTreeSet<u8>,
) -> (Target, trellis_core::TransactionResult<Command>) {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let source = tx.input::<BTreeSet<u8>>("source").unwrap();
    tx.set_input(source, initial).unwrap();
    let collection = tx
        .set_collection(
            "demand",
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
    let output = tx
        .materialized_output(
            "rows",
            scope,
            DependencyList::new([collection.id()]).unwrap(),
            move |ctx| Ok(ctx.set_collection(collection)?.clone()),
        )
        .unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    (
        Target {
            graph,
            source,
            output,
            scope,
        },
        result,
    )
}

fn set_source(
    target: &mut Target,
    values: BTreeSet<u8>,
) -> trellis_core::TransactionResult<Command> {
    let mut tx = target.graph.begin_transaction().unwrap();
    tx.set_input(target.source, values).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);
    target.graph.assert_incremental_equals_full().unwrap();
    result
}

fn rebaseline_output(target: &mut Target) -> trellis_core::TransactionResult<Command> {
    let mut tx = target.graph.begin_transaction().unwrap();
    tx.rebaseline_output(target.output.clone()).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);
    target.graph.assert_incremental_equals_full().unwrap();
    result
}

fn close_scope(target: &mut Target) -> trellis_core::TransactionResult<Command> {
    let mut tx = target.graph.begin_transaction().unwrap();
    tx.close_scope(target.scope).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);
    result
}

fn commit_noop(target: &mut Target) -> trellis_core::TransactionResult<Command> {
    let mut tx = target.graph.begin_transaction().unwrap();
    let result = tx.commit().unwrap();
    drop(tx);
    target.graph.assert_incremental_equals_full().unwrap();
    result
}

fn key(value: u8) -> ResourceKey {
    ResourceKey::new(format!("fuzz:{value}"))
}

fn members(value: u8) -> BTreeSet<u8> {
    (0..6).filter(|bit| value & (1 << bit) != 0).collect()
}
