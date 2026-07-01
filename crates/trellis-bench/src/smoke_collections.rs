use std::collections::{BTreeMap, BTreeSet};

use trellis_core::{
    DependencyList, Graph, InputNode, ResourceKey, ResourcePlan, ScopeId, TransactionTrace,
    assert_transaction_traces_match,
};

const LARGE: u32 = 512;
const OWNERS: usize = 16;

#[derive(Clone, Debug, Eq, PartialEq)]
enum Command {
    Open(u32),
}

type SetGraph = Graph<Command, BTreeSet<u32>>;

fn key(value: u32) -> ResourceKey {
    ResourceKey::new(format!("n:{value}"))
}

fn set(end: u32) -> BTreeSet<u32> {
    (0..end).collect()
}

fn map(size: u32, bump: u32) -> BTreeMap<u32, u32> {
    (0..size).map(|key| (key, key + bump)).collect()
}

fn build_set_graph(
    initial: BTreeSet<u32>,
    owner_count: usize,
    output: bool,
) -> (SetGraph, InputNode<BTreeSet<u32>>, Vec<ScopeId>) {
    let mut graph = Graph::<Command, BTreeSet<u32>>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope_count = owner_count.max(1);
    let scopes = (0..scope_count)
        .map(|index| tx.create_scope(format!("scope-{index}")).unwrap())
        .collect::<Vec<_>>();
    let source = tx.input::<BTreeSet<u32>>("source").unwrap();
    tx.set_input(source, initial).unwrap();
    let collection = tx
        .set_collection(
            "resources",
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(source)?.clone()),
        )
        .unwrap();
    for scope in scopes.iter().take(owner_count).copied() {
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
    }
    if output {
        tx.materialized_output(
            "output",
            scopes[0],
            DependencyList::new([collection.id()]).unwrap(),
            move |ctx| Ok(ctx.set_collection(collection)?.clone()),
        )
        .unwrap();
    }
    tx.commit().unwrap();
    drop(tx);
    (graph, source, scopes)
}

pub(crate) fn large_set_growth() -> usize {
    let (mut graph, source, _) = build_set_graph(BTreeSet::new(), 1, false);
    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(source, set(LARGE)).unwrap();
    tx.commit().unwrap().resource_plan.commands().len()
}

pub(crate) fn large_set_shrink() -> usize {
    let (mut graph, source, _) = build_set_graph(set(LARGE), 1, false);
    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(source, BTreeSet::new()).unwrap();
    tx.commit().unwrap().resource_plan.commands().len()
}

pub(crate) fn large_map_update() -> usize {
    let mut graph = Graph::<(), ()>::new();
    let mut tx = graph.begin_transaction().unwrap();
    let source = tx.input::<BTreeMap<u32, u32>>("source").unwrap();
    tx.set_input(source, map(LARGE, 0)).unwrap();
    tx.map_collection(
        "map",
        DependencyList::new([source.id()]).unwrap(),
        move |ctx| Ok(ctx.input(source)?.clone()),
    )
    .unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(source, map(LARGE, 1)).unwrap();
    tx.commit().unwrap().changed_collection_nodes.len()
}

pub(crate) fn scope_close_many_resources() -> usize {
    let (mut graph, _, scopes) = build_set_graph(set(LARGE), 1, false);
    let mut tx = graph.begin_transaction().unwrap();
    tx.close_scope(scopes[0]).unwrap();
    tx.commit().unwrap().resource_plan.commands().len()
}

pub(crate) fn shared_resource_many_owners() -> usize {
    let (mut graph, _, scopes) = build_set_graph(set(LARGE / 4), OWNERS, false);
    let mut tx = graph.begin_transaction().unwrap();
    for scope in scopes {
        tx.close_scope(scope).unwrap();
    }
    tx.commit().unwrap().resource_plan.commands().len()
}

pub(crate) fn output_baseline_then_delta() -> usize {
    let (mut graph, source, _) = build_set_graph(BTreeSet::new(), 0, true);
    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(source, set(32)).unwrap();
    tx.commit().unwrap().output_frames.len()
}

pub(crate) fn full_recompute_oracle() -> usize {
    let (mut graph, source, _) = build_set_graph(BTreeSet::new(), 1, true);
    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(source, set(64)).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);
    graph.assert_incremental_equals_full().unwrap();
    result.resource_plan.commands().len()
}

pub(crate) fn trace_replay_compare() -> usize {
    let first = run_trace_script();
    let second = run_trace_script();
    assert_transaction_traces_match(&first, &second).unwrap();
    first.len()
}

fn run_trace_script() -> Vec<TransactionTrace> {
    let (mut graph, source, _) = build_set_graph(BTreeSet::new(), 1, true);
    let mut traces = Vec::new();
    for step in 0..8 {
        let mut tx = graph.begin_transaction().unwrap();
        tx.set_input(source, set(step * 8)).unwrap();
        traces.push(tx.commit().unwrap().trace());
    }
    traces
}
