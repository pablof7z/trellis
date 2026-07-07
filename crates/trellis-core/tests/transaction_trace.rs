use std::collections::{BTreeMap, BTreeSet};

use trellis_core::{
    AuditEvent, AuditExplanationLevel, CollectionDiffKind, DependencyList, Graph,
    OutputFrameKindTrace, ResourceCommandKind, ResourceKey, ResourcePlan, ResourceTransitionPolicy,
    StagedInputOutcome, TransactionOptions,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Command {
    Open(String),
    Replace,
}

fn members(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn key(value: &str) -> ResourceKey {
    ResourceKey::new(format!("resource:{value}"))
}

fn path_options() -> TransactionOptions {
    TransactionOptions::default().with_audit_explanations(AuditExplanationLevel::DependencyPaths)
}

#[test]
fn transaction_trace_records_stable_structural_facts() {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let source = tx.input::<BTreeSet<String>>("source").unwrap();
    let mode = tx.input::<String>("mode").unwrap();
    tx.set_input(mode, "initial".to_owned()).unwrap();
    tx.set_input(source, members(&["b", "a"])).unwrap();
    let count = tx
        .derived(
            "count",
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(source)?.len()),
        )
        .unwrap();
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
            plan.open(
                key(&added.value),
                ctx.scope(),
                Command::Open(added.value.clone()),
            );
        }
        Ok(plan)
    })
    .unwrap();
    let first_output = tx
        .materialized_output(
            "first",
            scope,
            DependencyList::new([collection.id(), mode.id()]).unwrap(),
            move |ctx| {
                Ok(format!(
                    "{}:{}",
                    ctx.input(mode)?,
                    ctx.set_collection(collection)?.len()
                ))
            },
        )
        .unwrap();
    let second_output = tx
        .materialized_output(
            "second",
            scope,
            DependencyList::new([count.id()]).unwrap(),
            move |ctx| Ok(format!("count:{}", ctx.derived(count)?)),
        )
        .unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    let trace = result.trace();
    assert_eq!(
        trace
            .staged_input_changes
            .iter()
            .map(|change| (change.node, change.outcome))
            .collect::<Vec<_>>(),
        vec![
            (source.id(), StagedInputOutcome::Changed),
            (mode.id(), StagedInputOutcome::Changed),
        ]
    );
    assert_eq!(
        trace.dirty_roots,
        vec![source.id(), mode.id(), count.id(), collection.id()]
    );
    assert_eq!(trace.recomputed_derived_nodes, vec![count.id()]);
    assert_eq!(trace.changed_derived_nodes, vec![count.id()]);
    assert_eq!(trace.recomputed_collection_nodes, vec![collection.id()]);
    assert_eq!(trace.changed_collection_nodes, vec![collection.id()]);

    assert_eq!(trace.collection_diffs.len(), 1);
    assert_eq!(trace.collection_diffs[0].node, collection.id());
    assert_eq!(trace.collection_diffs[0].kind, CollectionDiffKind::Set);
    assert_eq!(trace.collection_diffs[0].added, 2);
    assert_eq!(trace.collection_diffs[0].removed, 0);
    assert_eq!(trace.collection_diffs[0].unchanged, 0);

    assert_eq!(
        trace
            .resource_commands
            .iter()
            .map(|command| (&command.key, command.kind, command.transition_policy))
            .collect::<Vec<_>>(),
        vec![
            (
                &key("a"),
                ResourceCommandKind::Open,
                ResourceTransitionPolicy::Open,
            ),
            (
                &key("b"),
                ResourceCommandKind::Open,
                ResourceTransitionPolicy::Open,
            ),
        ]
    );
    assert_eq!(
        trace
            .output_frames
            .iter()
            .map(|frame| (frame.output_key, frame.kind))
            .collect::<Vec<_>>(),
        vec![
            (first_output.key(), OutputFrameKindTrace::Baseline),
            (second_output.key(), OutputFrameKindTrace::Baseline),
        ]
    );
    assert_eq!(trace.scope_events.len(), 1);
    assert_eq!(
        trace
            .audit_log
            .iter()
            .filter_map(|entry| match entry.event {
                AuditEvent::InputChanged(node) => Some(node),
                _ => None,
            })
            .collect::<Vec<_>>(),
        vec![source.id(), mode.id()]
    );
}

#[test]
fn transaction_trace_records_transition_policy_separately_from_operation() {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let source = tx.input::<BTreeMap<String, u64>>("source").unwrap();
    tx.set_input(source, BTreeMap::from([("a".to_owned(), 1)]))
        .unwrap();
    let collection = tx
        .collection(
            "demand",
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(source)?.clone()),
        )
        .unwrap();
    tx.map_resource_planner(collection, scope, move |ctx| {
        let mut plan = ResourcePlan::new();
        for added in &ctx.diff().added {
            plan.open(
                key(&added.value.0),
                ctx.scope(),
                Command::Open(added.value.0.clone()),
            );
        }
        for updated in &ctx.diff().updated {
            plan.replace(key(&updated.key), ctx.scope(), Command::Replace);
        }
        Ok(plan)
    })
    .unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(source, BTreeMap::from([("a".to_owned(), 2)]))
        .unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert_eq!(result.trace().resource_commands.len(), 1);
    let command = &result.trace().resource_commands[0];
    assert_eq!(command.key, key("a"));
    assert_eq!(command.kind, ResourceCommandKind::Replace);
    assert_eq!(
        command.transition_policy,
        ResourceTransitionPolicy::ReplaceAtomically
    );
}

#[test]
fn transaction_trace_carries_audit_explanations() {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph
        .begin_transaction_with_options(path_options())
        .unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let source = tx.input::<BTreeSet<String>>("source").unwrap();
    tx.set_input(source, members(&["a"])).unwrap();
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
            plan.open(
                key(&added.value),
                ctx.scope(),
                Command::Open(added.value.clone()),
            );
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

    let trace = result.trace();
    assert_eq!(
        trace.audit_explanations.level,
        AuditExplanationLevel::DependencyPaths
    );
    assert!(
        trace
            .audit_explanations
            .node_changes
            .iter()
            .any(|explanation| {
                explanation.node == collection.id()
                    && explanation.input_causes == vec![source.id()]
                    && explanation.dependency_paths == vec![vec![source.id(), collection.id()]]
            })
    );
    assert!(
        trace
            .audit_explanations
            .resource_commands
            .iter()
            .any(|explanation| explanation.key == key("a")
                && explanation.input_causes == vec![source.id()])
    );
    assert!(trace.audit_explanations.output_frames.iter().any(
        |explanation| explanation.output_key == output.key()
            && explanation.changed_dependencies == vec![collection.id()]
            && explanation.dependency_paths == vec![vec![source.id(), collection.id()]]
    ));
}
