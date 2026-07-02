#![cfg(feature = "proptest")]

use std::collections::{BTreeMap, BTreeSet};

use proptest::prelude::*;
use trellis_core::{
    DependencyList, Graph, OutputFrame, ResourceCommand, ResourceKey, ResourcePlan,
    TransactionTrace, assert_transaction_traces_match,
    testing::{ModelScript, ModelStep, ModelTopology},
};
use trellis_testing::{
    ResourceLedger,
    proptest::{model_script_replay_debug, model_script_strategy},
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Command {
    Open(u8),
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn generated_model_scripts_drive_real_graphs(script in model_script_strategy(16)) {
        let first = run_script(&script);
        let second = run_script(&script);
        prop_assert!(
            assert_transaction_traces_match(&first, &second).is_ok(),
            "{}",
            model_script_replay_debug(&script)
        );
    }
}

fn run_script(script: &ModelScript) -> Vec<TransactionTrace> {
    match script.topology {
        ModelTopology::ScalarChain => run_scalar_chain_script(script),
        ModelTopology::SetResourceOutput => run_set_resource_script(script),
    }
}

fn run_set_resource_script(script: &ModelScript) -> Vec<TransactionTrace> {
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
    let output = tx
        .materialized_output(
            "output",
            scope,
            DependencyList::new([collection.id()]).unwrap(),
            move |ctx| Ok(ctx.set_collection(collection)?.clone()),
        )
        .unwrap();
    let initial = tx.commit().unwrap();
    drop(tx);

    let mut output_live = true;
    let mut scope_live = true;
    let mut ledger = ResourceLedger::new();
    let mut output_revisions = BTreeMap::new();
    let mut traces = vec![TransactionTrace::from_result(&initial)];
    apply_resource_result(&mut graph, &mut ledger, &mut output_revisions, &initial);

    for step in &script.steps {
        let mut tx = graph.begin_transaction().unwrap();
        match step {
            ModelStep::SetMembers(next) => tx.set_input(source, next.clone()).unwrap(),
            ModelStep::RebaselineOutput if output_live => {
                tx.rebaseline_output(output.clone()).unwrap();
            }
            ModelStep::ClosePrimaryScope if scope_live => {
                tx.close_scope(scope).unwrap();
                scope_live = false;
                output_live = false;
            }
            ModelStep::RebaselineOutput | ModelStep::ClosePrimaryScope => {}
        }
        let result = tx.commit().unwrap();
        drop(tx);

        apply_resource_result(&mut graph, &mut ledger, &mut output_revisions, &result);
        traces.push(TransactionTrace::from_result(&result));
    }
    traces
}

fn apply_resource_result(
    graph: &mut Graph<Command>,
    ledger: &mut ResourceLedger<Command>,
    output_revisions: &mut BTreeMap<trellis_core::OutputKey, trellis_core::Revision>,
    result: &trellis_core::TransactionResult<Command>,
) {
    ledger.apply_result(result);
    ledger.assert_no_duplicate_close().unwrap();
    ledger.assert_no_orphan_resources().unwrap();
    ledger.assert_graph_has_no_orphan_resources(graph).unwrap();
    assert_no_duplicate_closes(result);
    assert_output_revisions_monotonic(output_revisions, &result.output_frames);
    graph.assert_incremental_equals_full().unwrap();
}

fn run_scalar_chain_script(script: &ModelScript) -> Vec<TransactionTrace> {
    let mut graph = Graph::<()>::new();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let source = tx.input::<BTreeSet<u8>>("source").unwrap();
    tx.set_input(source, BTreeSet::new()).unwrap();
    let count = tx
        .derived(
            "count",
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(source)?.len()),
        )
        .unwrap();
    let output = tx
        .materialized_output(
            "output",
            scope,
            DependencyList::new([count.id()]).unwrap(),
            move |ctx| Ok(*ctx.derived(count)?),
        )
        .unwrap();
    let initial = tx.commit().unwrap();
    drop(tx);

    let mut output_live = true;
    let mut scope_live = true;
    let mut traces = vec![TransactionTrace::from_result(&initial)];
    graph.assert_incremental_equals_full().unwrap();

    for step in &script.steps {
        let mut tx = graph.begin_transaction().unwrap();
        match step {
            ModelStep::SetMembers(next) => tx.set_input(source, next.clone()).unwrap(),
            ModelStep::RebaselineOutput if output_live => {
                tx.rebaseline_output(output.clone()).unwrap();
            }
            ModelStep::ClosePrimaryScope if scope_live => {
                tx.close_scope(scope).unwrap();
                scope_live = false;
                output_live = false;
            }
            ModelStep::RebaselineOutput | ModelStep::ClosePrimaryScope => {}
        }
        let result = tx.commit().unwrap();
        drop(tx);
        graph.assert_incremental_equals_full().unwrap();
        traces.push(TransactionTrace::from_result(&result));
    }
    traces
}

fn assert_no_duplicate_closes(result: &trellis_core::TransactionResult<Command>) {
    let mut closed = BTreeSet::new();
    for command in result.resource_plan.commands() {
        if let ResourceCommand::Close { key, .. } = command {
            assert!(closed.insert(key.clone()), "duplicate close for {key:?}");
        }
    }
}

fn assert_output_revisions_monotonic(
    output_revisions: &mut BTreeMap<trellis_core::OutputKey, trellis_core::Revision>,
    frames: &[OutputFrame],
) {
    for frame in frames {
        let previous = output_revisions
            .insert(frame.output_key, frame.revision)
            .unwrap_or(frame.revision);
        assert!(frame.revision >= previous);
    }
}

fn key(value: u8) -> ResourceKey {
    ResourceKey::new(format!("n:{value}"))
}
