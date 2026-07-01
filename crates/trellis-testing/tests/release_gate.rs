use std::collections::BTreeSet;

use trellis_core::{DependencyList, Graph, InputNode, ResourceKey, ResourcePlan};
use trellis_testing::{
    ConformanceLevel, ConformanceReport, ConformanceSuite, NoRedaction, Scenario,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Command {
    Open(u8),
}

struct ScenarioGraph {
    graph: Graph<Command>,
    source: InputNode<BTreeSet<u8>>,
}

fn members(values: &[u8]) -> BTreeSet<u8> {
    values.iter().copied().collect()
}

fn key(value: u8) -> ResourceKey {
    ResourceKey::new(format!("test:{value}"))
}

fn build_graph(initial: BTreeSet<u8>) -> (ScenarioGraph, trellis_core::TransactionResult<Command>) {
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
    let result = tx.commit().unwrap();
    drop(tx);

    (ScenarioGraph { graph, source }, result)
}

fn set_source(
    target: &mut ScenarioGraph,
    values: BTreeSet<u8>,
) -> trellis_core::TransactionResult<Command> {
    let mut tx = target.graph.begin_transaction().unwrap();
    tx.set_input(target.source, values).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);
    target.graph.assert_incremental_equals_full().unwrap();
    result
}

fn run_scenario() -> Scenario {
    let (mut target, initial) = build_graph(members(&[1, 2]));
    let mut scenario = Scenario::new();
    scenario.record("initial", &initial);
    let shrink = set_source(&mut target, members(&[1]));
    scenario.record("shrink", &shrink);
    let empty = set_source(&mut target, BTreeSet::new());
    scenario.record("empty", &empty);
    scenario
}

#[test]
fn scenario_replay_is_structural_and_deterministic() {
    let first = run_scenario();
    let second = run_scenario();

    first.assert_replay_matches(&second).unwrap();
    assert_eq!(
        first.step("shrink").unwrap().trace.resource_commands.len(),
        1
    );
    first
        .assert_step_resource_commands(
            "shrink",
            &first.step("shrink").unwrap().trace.resource_commands,
        )
        .unwrap();
    assert_eq!(
        first.to_redacted_debug_string(&NoRedaction),
        second.to_redacted_debug_string(&NoRedaction)
    );
}

#[test]
fn conformance_levels_report_unsupported_explicitly() {
    let report = ConformanceReport::new()
        .support(ConformanceLevel::DeterministicTrace)
        .support(ConformanceLevel::ScopeResourceLifecycle)
        .support(ConformanceLevel::MaterializedOutput)
        .support(ConformanceLevel::FullRecomputeOracle)
        .unsupported(ConformanceLevel::GeneratedModelSequences);

    assert!(report.supports(ConformanceLevel::DeterministicTrace));
    assert!(
        report
            .unsupported_levels()
            .contains(&ConformanceLevel::GeneratedModelSequences)
    );

    let suite = ConformanceSuite::all();
    let report = suite.report(&[
        ConformanceLevel::DeterministicTrace,
        ConformanceLevel::ScopeResourceLifecycle,
    ]);
    assert!(report.supports(ConformanceLevel::DeterministicTrace));
    assert!(
        report
            .unsupported_levels()
            .contains(&ConformanceLevel::MaterializedOutput)
    );
}
