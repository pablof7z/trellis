#![allow(dead_code)]

use std::collections::BTreeSet;

use trellis_core::{
    DependencyList, Graph, InputNode, MaterializedOutput, ResourceKey, ResourcePlan, ScopeId,
};
use trellis_testing::{ResourceLedger, Scenario};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Command {
    Open(u8),
}

struct Target {
    graph: Graph<Command, BTreeSet<u8>>,
    source: InputNode<BTreeSet<u8>>,
    output: MaterializedOutput<BTreeSet<u8>>,
    scope: ScopeId,
}

pub(crate) fn run_resource_lifecycle(data: &[u8]) {
    let (mut target, initial) = build_graph(members(data.first().copied().unwrap_or_default()));
    let mut ledger = ResourceLedger::new();
    ledger.apply_result(&initial);
    ledger.assert_no_orphan_resources().unwrap();

    for value in data.iter().copied().skip(1).take(32) {
        let result = set_source(&mut target, members(value));
        ledger.apply_result(&result);
        ledger.assert_no_duplicate_close().unwrap();
        ledger.assert_no_orphan_resources().unwrap();
    }

    let closed = close_scope(&mut target);
    ledger.apply_result(&closed);
    let _ = target.output.key();
    ledger
        .assert_closed_scope_owns_no_resources(target.scope)
        .unwrap();
}

pub(crate) fn run_trace_replay(data: &[u8]) {
    let first = run_scenario(data);
    let second = run_scenario(data);
    first.assert_replay_matches(&second).unwrap();
}

fn run_scenario(data: &[u8]) -> Scenario {
    let (mut target, initial) = build_graph(members(data.first().copied().unwrap_or_default()));
    let mut scenario = Scenario::new();
    scenario.record("initial", &initial);
    for (index, value) in data.iter().copied().skip(1).take(32).enumerate() {
        let result = set_source(&mut target, members(value));
        scenario.record(format!("step-{index}"), &result);
    }
    scenario
}

fn build_graph(
    initial: BTreeSet<u8>,
) -> (
    Target,
    trellis_core::TransactionResult<Command, BTreeSet<u8>>,
) {
    let mut graph = Graph::<Command, BTreeSet<u8>>::new_with_command_type();
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
) -> trellis_core::TransactionResult<Command, BTreeSet<u8>> {
    let mut tx = target.graph.begin_transaction().unwrap();
    tx.set_input(target.source, values).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);
    target.graph.assert_incremental_equals_full().unwrap();
    result
}

fn close_scope(target: &mut Target) -> trellis_core::TransactionResult<Command, BTreeSet<u8>> {
    let mut tx = target.graph.begin_transaction().unwrap();
    tx.close_scope(target.scope).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);
    result
}

fn key(value: u8) -> ResourceKey {
    ResourceKey::new(format!("fuzz:{value}"))
}

fn members(value: u8) -> BTreeSet<u8> {
    (0..6).filter(|bit| value & (1 << bit) != 0).collect()
}
