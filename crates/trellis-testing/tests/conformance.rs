use trellis_core::{Graph, TransactionPhase};
use trellis_testing::{
    ConformanceCheckResult, ConformanceLevel, ConformanceSuite, Scenario, conformance,
};

fn deterministic_graph_dump() -> String {
    let mut graph = Graph::<()>::new();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("screen").unwrap();
    let input = tx.input::<usize>("count").unwrap();
    tx.set_input(input, 1).unwrap();
    tx.attach_node_to_scope(input, scope).unwrap();
    tx.commit().unwrap();
    drop(tx);
    graph.debug_dump()
}

#[test]
fn executable_conformance_runs_app_owned_checks() {
    let report = conformance()
        .check(
            ConformanceLevel::DeterministicTrace,
            "debug dump is deterministic",
            || {
                if deterministic_graph_dump() == deterministic_graph_dump() {
                    ConformanceCheckResult::passed()
                } else {
                    ConformanceCheckResult::failed("debug dump differed between fresh graphs")
                }
            },
        )
        .unsupported(
            ConformanceLevel::GeneratedModelSequences,
            "app has not opted into generated sequence hooks",
        )
        .run()
        .unwrap();

    assert!(report.supports(ConformanceLevel::DeterministicTrace));
    assert!(
        report
            .unsupported_levels()
            .contains(&ConformanceLevel::GeneratedModelSequences)
    );
    assert_eq!(report.check_results().len(), 1);
}

#[test]
fn required_levels_without_hooks_are_reported_unsupported() {
    let report = ConformanceSuite::all()
        .runner()
        .check(
            ConformanceLevel::DeterministicTrace,
            "phase order is stable",
            || {
                let mut graph = Graph::<()>::new();
                let mut tx = graph.begin_transaction().unwrap();
                let result = tx.commit().unwrap();
                drop(tx);
                if result
                    .phase_trace
                    .contains(&TransactionPhase::ReturnTransactionResult)
                {
                    ConformanceCheckResult::passed()
                } else {
                    ConformanceCheckResult::failed("missing return phase")
                }
            },
        )
        .run()
        .unwrap();

    assert!(report.supports(ConformanceLevel::DeterministicTrace));
    assert!(
        report
            .unsupported_levels()
            .contains(&ConformanceLevel::MaterializedOutput)
    );
    assert!(
        report
            .unsupported_reasons()
            .get(&ConformanceLevel::MaterializedOutput)
            .unwrap()
            .iter()
            .any(|reason| reason.contains("no conformance check registered"))
    );
}

#[test]
fn failures_include_invariant_and_trace_context() {
    let mut scenario = Scenario::new();
    let mut graph = Graph::<()>::new();
    let mut tx = graph.begin_transaction().unwrap();
    let result = tx.commit().unwrap();
    drop(tx);
    scenario.record("empty", &result).unwrap();

    let failure = conformance()
        .check(
            ConformanceLevel::DeterministicTrace,
            "same input sequence produces same trace",
            move || {
                ConformanceCheckResult::failed(format!(
                    "scenario empty trace: {:#?}",
                    scenario.step("empty").unwrap().trace
                ))
            },
        )
        .run()
        .unwrap_err();

    assert_eq!(
        failure.invariant,
        "same input sequence produces same trace".to_owned()
    );
    assert!(failure.detail.contains("scenario empty trace"));
}
