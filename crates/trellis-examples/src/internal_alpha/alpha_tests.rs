use std::collections::BTreeSet;

#[path = "shared_resource.rs"]
mod shared_resource;

use trellis_core::{
    DependencyList, Graph, InputNode, MaterializedOutput, OutputFrameKind, ResourceCommand,
    ResourceCommandKind, ResourceKey, ResourcePlan, ScopeId, TransactionResult, TransactionTrace,
    assert_transaction_traces_match,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum AlphaCommand {
    Open(u8),
}

struct AlphaApp {
    graph: Graph<AlphaCommand, BTreeSet<u8>>,
    source: InputNode<BTreeSet<u8>>,
    allowed: InputNode<BTreeSet<u8>>,
    scope: ScopeId,
    output: MaterializedOutput<BTreeSet<u8>>,
    initial: TransactionResult<AlphaCommand, BTreeSet<u8>>,
}

fn members(values: &[u8]) -> BTreeSet<u8> {
    values.iter().copied().collect()
}

fn key(value: u8) -> ResourceKey {
    ResourceKey::new(format!("alpha:{value}"))
}

fn command_closes(result: &TransactionResult<AlphaCommand, BTreeSet<u8>>, value: u8) -> bool {
    result.resource_plan.commands().iter().any(|command| {
        matches!(command, ResourceCommand::Close { key: closed, .. } if closed == &key(value))
    })
}

fn apply_frames(
    consumer: &mut Option<BTreeSet<u8>>,
    result: &TransactionResult<AlphaCommand, BTreeSet<u8>>,
) {
    for frame in &result.output_frames {
        match &frame.kind {
            OutputFrameKind::Baseline(value)
            | OutputFrameKind::Delta(value)
            | OutputFrameKind::Rebaseline(value, _) => *consumer = Some(value.clone()),
            OutputFrameKind::Clear(_) => *consumer = None,
        }
    }
}

fn commit_source(
    app: &mut AlphaApp,
    next: BTreeSet<u8>,
) -> TransactionResult<AlphaCommand, BTreeSet<u8>> {
    let mut tx = app.graph.begin_transaction().unwrap();
    tx.set_input(app.source, next).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);
    app.graph.assert_incremental_equals_full().unwrap();
    result
}

fn commit_allowed(
    app: &mut AlphaApp,
    next: BTreeSet<u8>,
) -> TransactionResult<AlphaCommand, BTreeSet<u8>> {
    let mut tx = app.graph.begin_transaction().unwrap();
    tx.set_input(app.allowed, next).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);
    app.graph.assert_incremental_equals_full().unwrap();
    result
}

fn alpha_trace_script() -> Vec<TransactionTrace> {
    let mut app = build_alpha(members(&[1, 2, 3]), members(&[1, 2, 3]));
    let mut traces = vec![app.initial.trace()];

    let result = commit_source(&mut app, members(&[1, 3]));
    assert!(command_closes(&result, 2));
    traces.push(result.trace());

    let result = commit_allowed(&mut app, members(&[3]));
    assert!(command_closes(&result, 1));
    traces.push(result.trace());

    let mut tx = app.graph.begin_transaction().unwrap();
    tx.rebaseline_output(app.output).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);
    app.graph.assert_incremental_equals_full().unwrap();
    traces.push(result.trace());

    let mut tx = app.graph.begin_transaction().unwrap();
    tx.close_scope(app.scope).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);
    assert!(command_closes(&result, 3));
    assert!(
        app.graph
            .scope_resource_inventory(app.scope)
            .unwrap()
            .resources
            .is_empty()
    );
    app.graph.assert_incremental_equals_full().unwrap();
    traces.push(result.trace());

    traces
}

fn build_alpha(source_members: BTreeSet<u8>, allowed_members: BTreeSet<u8>) -> AlphaApp {
    let mut graph = Graph::<AlphaCommand, BTreeSet<u8>>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("alpha-session").unwrap();
    let source = tx.input::<BTreeSet<u8>>("source").unwrap();
    let allowed = tx.input::<BTreeSet<u8>>("allowed").unwrap();
    tx.set_input(source, source_members).unwrap();
    tx.set_input(allowed, allowed_members).unwrap();
    let visible = tx
        .derived(
            "visible",
            DependencyList::new([source.id(), allowed.id()]).unwrap(),
            move |ctx| {
                let source = ctx.input(source)?;
                let allowed = ctx.input(allowed)?;
                Ok(source
                    .intersection(allowed)
                    .copied()
                    .collect::<BTreeSet<u8>>())
            },
        )
        .unwrap();
    let demand = tx
        .set_collection(
            "demand",
            DependencyList::new([visible.id()]).unwrap(),
            move |ctx| Ok(ctx.derived(visible)?.clone()),
        )
        .unwrap();
    tx.set_resource_planner(demand, scope, move |ctx| {
        let mut plan = ResourcePlan::new();
        for added in &ctx.diff().added {
            plan.open(
                key(added.value),
                ctx.scope(),
                AlphaCommand::Open(added.value),
            );
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
            DependencyList::new([demand.id()]).unwrap(),
            move |ctx| Ok(ctx.set_collection(demand)?.clone()),
        )
        .unwrap();
    let initial = tx.commit().unwrap();
    drop(tx);

    AlphaApp {
        graph,
        source,
        allowed,
        scope,
        output,
        initial,
    }
}

#[test]
fn alpha_replay_trace_is_deterministic() {
    let first = alpha_trace_script();
    let second = alpha_trace_script();

    assert_transaction_traces_match(&first, &second).unwrap();
}

#[test]
fn alpha_catches_source_shrink_missing_resource_close() {
    let mut app = build_alpha(members(&[1, 2, 3]), members(&[1, 2, 3]));

    let result = commit_source(&mut app, members(&[1, 3]));

    assert!(command_closes(&result, 2));
    assert!(app.graph.resource_owners(&key(2)).is_none());
    assert!(app.graph.orphan_resources().is_empty());
}

#[test]
fn alpha_catches_empty_source_broadening_resource_demand() {
    let mut app = build_alpha(members(&[7]), members(&[7]));

    let result = commit_source(&mut app, BTreeSet::new());

    assert!(command_closes(&result, 7));
    assert!(
        !result
            .resource_plan
            .commands()
            .iter()
            .any(|command| matches!(command, ResourceCommand::Open { .. }))
    );
    assert!(app.graph.resource_owners(&key(7)).is_none());
}

#[test]
fn alpha_catches_stale_derived_visibility_after_permission_shrink() {
    let mut app = build_alpha(members(&[1, 2]), members(&[1, 2]));

    let result = commit_allowed(&mut app, members(&[1]));

    assert!(command_closes(&result, 2));
    assert!(matches!(
        &result.output_frames[0].kind,
        OutputFrameKind::Delta(rows) if rows == &members(&[1])
    ));
    let explanation = app.graph.why_resource_command(&key(2)).unwrap();
    assert_eq!(explanation.key, key(2));
    assert_eq!(
        explanation.scope,
        result.resource_plan.commands()[0].scope()
    );
    assert_eq!(explanation.transaction_id, result.transaction_id);
    assert_eq!(explanation.revision, result.revision);
    assert_eq!(explanation.kind, ResourceCommandKind::Close);
    assert_eq!(explanation.input_causes, vec![app.allowed.id()]);
    assert!(explanation.changed_nodes.contains(&app.allowed.id()));
    assert!(
        explanation
            .dependency_paths
            .iter()
            .any(|path| path.first() == Some(&app.allowed.id()))
    );
}

#[test]
fn alpha_catches_scope_close_leaking_resources_or_output() {
    let mut app = build_alpha(members(&[1, 2]), members(&[1, 2]));

    let mut tx = app.graph.begin_transaction().unwrap();
    tx.close_scope(app.scope).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert!(command_closes(&result, 1));
    assert!(command_closes(&result, 2));
    assert!(
        result
            .output_frames
            .iter()
            .any(|frame| matches!(frame.kind, OutputFrameKind::Clear(_)))
    );
    assert!(
        app.graph
            .scope_resource_inventory(app.scope)
            .unwrap()
            .resources
            .is_empty()
    );
    assert!(app.graph.orphan_resources().is_empty());
    app.graph.assert_incremental_equals_full().unwrap();
}

#[test]
fn alpha_catches_output_delta_sequence_that_disagrees_with_rebaseline() {
    let mut app = build_alpha(members(&[1]), members(&[1, 2]));
    let mut consumer = None;
    apply_frames(&mut consumer, &app.initial);

    for next in [members(&[1, 2]), members(&[2])] {
        let mut tx = app.graph.begin_transaction().unwrap();
        tx.set_input(app.source, next).unwrap();
        let result = tx.commit().unwrap();
        drop(tx);
        apply_frames(&mut consumer, &result);
        app.graph.assert_incremental_equals_full().unwrap();
    }

    let mut tx = app.graph.begin_transaction().unwrap();
    tx.rebaseline_output(app.output).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    let OutputFrameKind::Rebaseline(baseline, _) = &result.output_frames[0].kind else {
        panic!("expected rebaseline frame");
    };
    assert_eq!(consumer.as_ref(), Some(baseline));
}
