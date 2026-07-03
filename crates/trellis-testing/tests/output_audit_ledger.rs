use std::collections::BTreeSet;

use trellis_core::{
    AuditExplanationLevel, CollectionNode, DependencyList, Graph, InputNode, MaterializedOutput,
    OutputFrameKind, OutputKey, ResourceKey, ResourcePlan, ScopeId, TransactionOptions,
};
use trellis_testing::{
    FullRecomputeOracle, OutputLedger, OutputLedgerError, assert_dependency_path_exists,
    assert_incremental_equals_full, assert_no_unexplained_output_frame, assert_no_unexplained_plan,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Command {
    Open(u8),
}

struct TestGraph {
    graph: Graph<Command>,
    source: InputNode<BTreeSet<u8>>,
    collection: CollectionNode<u8, ()>,
    output: MaterializedOutput<BTreeSet<u8>>,
    scope: ScopeId,
}

fn members(values: &[u8]) -> BTreeSet<u8> {
    values.iter().copied().collect()
}

fn key(value: u8) -> ResourceKey {
    ResourceKey::new(format!("test:{value}"))
}

fn audit_paths_options() -> TransactionOptions {
    TransactionOptions::default().with_audit_explanations(AuditExplanationLevel::DependencyPaths)
}

fn build_graph(initial: BTreeSet<u8>) -> (TestGraph, trellis_core::TransactionResult<Command>) {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph
        .begin_transaction_with_options(audit_paths_options())
        .unwrap();
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
            collection,
            output,
            scope,
        },
        result,
    )
}

fn set_source(
    target: &mut TestGraph,
    values: BTreeSet<u8>,
) -> trellis_core::TransactionResult<Command> {
    let mut tx = target
        .graph
        .begin_transaction_with_options(audit_paths_options())
        .unwrap();
    tx.set_input(target.source, values).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);
    target.graph.assert_incremental_equals_full().unwrap();
    result
}

struct LedgerOracle;

impl FullRecomputeOracle<OutputLedger> for LedgerOracle {
    type CanonicalInputs = (OutputKey, BTreeSet<u8>);
    type ExpectedState = BTreeSet<u8>;

    fn recompute(inputs: &Self::CanonicalInputs) -> Self::ExpectedState {
        inputs.1.clone()
    }

    fn observe_incremental(
        ledger: &OutputLedger,
        inputs: &Self::CanonicalInputs,
    ) -> Self::ExpectedState {
        ledger
            .snapshot(inputs.0)
            .and_then(|snapshot| snapshot.state_as::<BTreeSet<u8>>().cloned())
            .unwrap_or_default()
    }
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
    ledger
        .assert_consumer_needs_no_hidden_graph_state()
        .unwrap();
    assert_incremental_equals_full::<_, LedgerOracle>(&ledger, &(output_key, members(&[1, 2])))
        .unwrap();
    assert!(matches!(
        &rebaseline.output_frames[0].kind,
        OutputFrameKind::Rebaseline(value, _)
            if value
                .get::<BTreeSet<u8>>()
                .is_some_and(|value| value == &members(&[1, 2]))
    ));

    let mut tx = target.graph.begin_transaction().unwrap();
    tx.close_scope(target.scope).unwrap();
    let closed = tx.commit().unwrap();
    drop(tx);
    ledger.close_scope(target.scope);
    ledger.apply_result(&closed);
    ledger.assert_cleared(output_key).unwrap();
    ledger.assert_closed_scope_cleared(target.scope).unwrap();
    ledger
        .assert_no_frame_for_closed_scope_except_terminal()
        .unwrap();

    let bad_frame = trellis_core::OutputFrame {
        output_key,
        scope: target.scope,
        transaction_id: closed.transaction_id,
        revision: closed.revision,
        kind: OutputFrameKind::delta(members(&[9])),
    };
    ledger.apply_frame(&bad_frame);
    let error = ledger
        .assert_no_frame_for_closed_scope_except_terminal()
        .unwrap_err();
    assert!(matches!(
        error,
        OutputLedgerError::FrameAfterClosedScope { context }
            if context.output_key == output_key
                && context.scope == target.scope
                && context.transaction_id == closed.transaction_id
                && context.revision == closed.revision
    ));
}

#[test]
fn audit_assertions_explain_plans_and_frames() {
    let (target, initial) = build_graph(members(&[1]));

    assert_no_unexplained_plan(&target.graph, &initial).unwrap();
    assert_no_unexplained_output_frame(&target.graph, &initial).unwrap();
    assert_dependency_path_exists(&target.graph, target.source.id(), target.collection.id())
        .unwrap();
}
