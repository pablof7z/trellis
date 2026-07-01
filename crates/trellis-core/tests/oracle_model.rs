use std::collections::{BTreeMap, BTreeSet};

use trellis_core::{
    DependencyList, Graph, ModelGenerator, ModelScript, ModelStep, ModelTopology, OutputFrameKind,
    ResourceCommand, ResourceKey, ResourcePlan, TransactionTrace, assert_transaction_traces_match,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Command {
    Open(u8),
}

fn members(values: &[u8]) -> BTreeSet<u8> {
    values.iter().copied().collect()
}

fn key(value: u8) -> ResourceKey {
    ResourceKey::new(format!("n:{value}"))
}

fn assert_no_duplicate_closes(result: &trellis_core::TransactionResult<Command, BTreeSet<u8>>) {
    let mut closed = BTreeSet::new();
    for command in result.resource_plan.commands() {
        if let ResourceCommand::Close { key, .. } = command {
            assert!(closed.insert(key.clone()), "duplicate close for {key:?}");
        }
    }
}

fn apply_set_frame(state: &mut Option<BTreeSet<u8>>, kind: &OutputFrameKind<BTreeSet<u8>>) {
    match kind {
        OutputFrameKind::Baseline(value)
        | OutputFrameKind::Delta(value)
        | OutputFrameKind::Rebaseline(value, _) => {
            *state = Some(value.clone());
        }
        OutputFrameKind::Clear(_) => *state = None,
    }
}

fn run_set_resource_script(script: &ModelScript) -> Vec<TransactionTrace> {
    let mut graph = Graph::<Command, BTreeSet<u8>>::new_with_command_type();
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
    let result = tx.commit().unwrap();
    drop(tx);

    let mut output_live = true;
    let mut scope_live = true;
    let mut last_revision = BTreeMap::new();
    let mut traces = vec![TransactionTrace::from_result(&result)];
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

        for frame in &result.output_frames {
            let previous = last_revision
                .insert(frame.output_key, frame.revision)
                .unwrap_or(frame.revision);
            assert!(frame.revision >= previous);
        }
        assert_no_duplicate_closes(&result);
        assert!(graph.orphan_resources().is_empty());
        graph.assert_incremental_equals_full().unwrap();
        traces.push(TransactionTrace::from_result(&result));
    }
    traces
}

fn run_scalar_chain_script(script: &ModelScript) -> Vec<TransactionTrace> {
    let mut graph = Graph::<(), usize>::new_with_output_type();
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
    let result = tx.commit().unwrap();
    drop(tx);

    let mut output_live = true;
    let mut scope_live = true;
    let mut traces = vec![TransactionTrace::from_result(&result)];
    graph.assert_incremental_equals_full().unwrap();

    for step in &script.steps {
        let mut tx = graph.begin_transaction().unwrap();
        match step {
            ModelStep::SetMembers(next) => tx.set_input(source, next.clone()).unwrap(),
            ModelStep::RebaselineOutput if output_live => tx.rebaseline_output(output).unwrap(),
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

#[test]
fn full_recompute_includes_resources_and_outputs() {
    let script = ModelScript {
        topology: ModelTopology::SetResourceOutput,
        steps: vec![
            ModelStep::SetMembers(members(&[1, 2, 3])),
            ModelStep::SetMembers(members(&[1, 3])),
            ModelStep::SetMembers(BTreeSet::new()),
            ModelStep::ClosePrimaryScope,
        ],
    };

    let traces = run_set_resource_script(&script);

    assert!(traces.iter().any(|trace| {
        trace
            .resource_commands
            .iter()
            .any(|command| command.key == key(2))
    }));
}

#[test]
fn generated_model_replay_is_deterministic() {
    for seed in 0..12 {
        let mut generator = ModelGenerator::new(seed);
        let script = generator.script(8);
        let first = match script.topology {
            ModelTopology::ScalarChain => run_scalar_chain_script(&script),
            ModelTopology::SetResourceOutput => run_set_resource_script(&script),
        };
        let second = match script.topology {
            ModelTopology::ScalarChain => run_scalar_chain_script(&script),
            ModelTopology::SetResourceOutput => run_set_resource_script(&script),
        };

        assert_transaction_traces_match(&first, &second).unwrap();
    }
}

#[test]
fn output_delta_sequence_matches_later_rebaseline() {
    let mut graph = Graph::<Command, BTreeSet<u8>>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let source = tx.input::<BTreeSet<u8>>("source").unwrap();
    tx.set_input(source, members(&[1])).unwrap();
    let collection = tx
        .set_collection(
            "resources",
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(source)?.clone()),
        )
        .unwrap();
    let output = tx
        .materialized_output(
            "output",
            scope,
            DependencyList::new([collection.id()]).unwrap(),
            move |ctx| Ok(ctx.set_collection(collection)?.clone()),
        )
        .unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    let mut consumer = None;
    for frame in &result.output_frames {
        apply_set_frame(&mut consumer, &frame.kind);
    }
    graph.assert_incremental_equals_full().unwrap();

    for next in [members(&[1, 2]), members(&[2]), BTreeSet::new()] {
        let mut tx = graph.begin_transaction().unwrap();
        tx.set_input(source, next).unwrap();
        let result = tx.commit().unwrap();
        drop(tx);
        for frame in &result.output_frames {
            apply_set_frame(&mut consumer, &frame.kind);
        }
        graph.assert_incremental_equals_full().unwrap();
    }

    let mut tx = graph.begin_transaction().unwrap();
    tx.rebaseline_output(output).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    let OutputFrameKind::Rebaseline(final_state, _) = &result.output_frames[0].kind else {
        panic!("expected rebaseline");
    };
    assert_eq!(consumer.as_ref(), Some(final_state));
}
