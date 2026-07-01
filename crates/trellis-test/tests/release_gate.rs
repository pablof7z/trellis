use std::collections::BTreeSet;

use trellis_core::{
    DependencyList, Graph, InputNode, MaterializedOutput, OutputFrameKind, ResourceKey,
    ResourcePlan, Revision, ScopeId,
};
use trellis_test::{
    ConformanceLevel, ConformanceReport, HostStatusClass, HostStatusEvent, OutputLedger,
    ResourceLedger, Scenario,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Command {
    Open(u8),
}

struct TestGraph {
    graph: Graph<Command, BTreeSet<u8>>,
    source: InputNode<BTreeSet<u8>>,
    output: MaterializedOutput<BTreeSet<u8>>,
    scope: ScopeId,
}

fn members(values: &[u8]) -> BTreeSet<u8> {
    values.iter().copied().collect()
}

fn key(value: u8) -> ResourceKey {
    ResourceKey::new(format!("test:{value}"))
}

fn build_graph(
    initial: BTreeSet<u8>,
) -> (
    TestGraph,
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
        TestGraph {
            graph,
            source,
            output,
            scope,
        },
        result,
    )
}

fn set_source(
    target: &mut TestGraph,
    values: BTreeSet<u8>,
) -> trellis_core::TransactionResult<Command, BTreeSet<u8>> {
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
}

#[test]
fn resource_ledger_detects_lifecycle_and_status_classes() {
    let (mut target, initial) = build_graph(members(&[1, 2]));
    let mut ledger = ResourceLedger::new();
    ledger.mark_forbidden_unless_explicit(ResourceKey::new("test:*"));
    ledger.apply_result(&initial);
    ledger.assert_all_resources_have_owner().unwrap();
    ledger.assert_no_forbidden_opened().unwrap();

    let shrink = set_source(&mut target, members(&[1]));
    ledger.apply_result(&shrink);
    ledger.assert_resource_not_open(&key(2)).unwrap();
    ledger.assert_no_duplicate_close().unwrap();

    let status = HostStatusEvent {
        key: key(1),
        scope: target.scope,
        command_revision: Revision::new(0),
        status_revision: Revision::new(100),
    };
    assert_eq!(ledger.classify_status(status), HostStatusClass::Stale);

    let current = HostStatusEvent {
        key: key(1),
        scope: target.scope,
        command_revision: initial.revision,
        status_revision: Revision::new(101),
    };
    assert_eq!(
        ledger.classify_status(current.clone()),
        HostStatusClass::Current
    );
    assert_eq!(ledger.classify_status(current), HostStatusClass::Duplicate);

    let future = HostStatusEvent {
        key: key(1),
        scope: target.scope,
        command_revision: Revision::new(10),
        status_revision: Revision::new(102),
    };
    assert_eq!(ledger.classify_status(future), HostStatusClass::Future);
}

#[test]
fn output_ledger_checks_revision_and_rebaseline_coherence() {
    let (mut target, initial) = build_graph(members(&[1]));
    let mut ledger = OutputLedger::new();
    ledger.apply_result(&initial);
    ledger
        .assert_current_equals(target.output.key(), &members(&[1]))
        .unwrap();

    let next = set_source(&mut target, members(&[1, 2]));
    ledger.apply_result(&next);
    ledger
        .assert_current_equals(target.output.key(), &members(&[1, 2]))
        .unwrap();

    let output_key = target.output.key();
    let mut tx = target.graph.begin_transaction().unwrap();
    tx.rebaseline_output(target.output).unwrap();
    let rebaseline = tx.commit().unwrap();
    drop(tx);
    ledger.apply_result(&rebaseline);
    ledger.assert_revision_monotonic().unwrap();
    ledger
        .assert_current_equals(output_key, &members(&[1, 2]))
        .unwrap();

    assert!(matches!(
        &rebaseline.output_frames[0].kind,
        OutputFrameKind::Rebaseline(value, _) if value == &members(&[1, 2])
    ));

    let mut tx = target.graph.begin_transaction().unwrap();
    tx.close_scope(target.scope).unwrap();
    let closed = tx.commit().unwrap();
    drop(tx);
    ledger.close_scope(target.scope);
    ledger.apply_result(&closed);
    ledger.assert_cleared(output_key).unwrap();
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
}
