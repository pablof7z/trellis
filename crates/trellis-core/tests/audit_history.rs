use std::collections::BTreeSet;

use trellis_core::{
    AuditExplanationLevel, AuditHistory, AuditHistoryError, DependencyList, Graph,
    OutputFrameKindTrace, ResourceCommandKind, ResourceKey, ResourcePlan, Revision,
    TransactionOptions,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Command {
    Open(String),
}

fn key(value: &str) -> ResourceKey {
    ResourceKey::new(value.to_owned())
}

fn set(entries: &[&str]) -> BTreeSet<String> {
    entries.iter().map(|value| (*value).to_owned()).collect()
}

fn path_options() -> TransactionOptions {
    TransactionOptions::default().with_audit_explanations(AuditExplanationLevel::DependencyPaths)
}

fn disabled_options() -> TransactionOptions {
    TransactionOptions::default().with_audit_explanations(AuditExplanationLevel::Disabled)
}

fn summary_options() -> TransactionOptions {
    TransactionOptions::default().with_audit_explanations(AuditExplanationLevel::Summary)
}

#[test]
fn retained_history_answers_revision_after_graph_latest_moves() {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph
        .begin_transaction_with_options(path_options())
        .unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let source = tx.input::<BTreeSet<String>>("source").unwrap();
    tx.set_input(source, set(&["a", "b"])).unwrap();
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
            plan.open(
                key(&added.value),
                ctx.scope(),
                Command::Open(added.value.clone()),
            );
        }
        for removed in &ctx.diff().removed {
            plan.close(key(&removed.value), ctx.scope());
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
    let first = tx.commit().unwrap();
    drop(tx);

    let mut history = AuditHistory::new();
    history.retain(&first);

    let mut tx = graph
        .begin_transaction_with_options(path_options())
        .unwrap();
    tx.set_input(source, set(&["a"])).unwrap();
    let second = tx.commit().unwrap();
    drop(tx);

    assert_eq!(
        graph.why_resource_command(&key("b")).unwrap().kind,
        ResourceCommandKind::Close
    );
    assert_eq!(
        graph.why_resource_command(&key("b")).unwrap().revision,
        second.revision
    );

    let retained_node = history.why_at(first.revision, collection).unwrap();
    assert_eq!(retained_node.revision, first.revision);
    assert_eq!(retained_node.input_causes, vec![source.id()]);
    assert_eq!(
        history.dependency_path_at(first.revision, source.id(), collection.id()),
        Ok(vec![source.id(), collection.id()])
    );

    let retained_command = history
        .why_resource_command_at(first.revision, &key("b"))
        .unwrap();
    assert_eq!(retained_command.kind, ResourceCommandKind::Open);
    assert_eq!(retained_command.revision, first.revision);

    let retained_frame = history
        .why_output_frame_at(first.revision, output.key())
        .unwrap();
    assert_eq!(retained_frame.kind, OutputFrameKindTrace::Baseline);
    assert_eq!(retained_frame.revision, first.revision);

    assert_eq!(
        history.why_resource_command_at(first.revision, &key("missing")),
        Err(AuditHistoryError::ResourceCommandNotFound {
            revision: first.revision,
            key: key("missing"),
        })
    );
}

#[test]
fn historical_queries_distinguish_disabled_summary_and_removed_revisions() {
    let mut graph = Graph::new();
    let mut tx = graph
        .begin_transaction_with_options(disabled_options())
        .unwrap();
    let input = tx.input::<usize>("input").unwrap();
    tx.set_input(input, 1).unwrap();
    let derived = tx
        .derived(
            "derived",
            DependencyList::new([input.id()]).unwrap(),
            move |ctx| Ok(*ctx.input(input)? + 1),
        )
        .unwrap();
    let disabled = tx.commit().unwrap();
    drop(tx);

    let mut history = AuditHistory::new();
    history.retain(&disabled);
    assert_eq!(
        history.why_at(disabled.revision, derived),
        Err(AuditHistoryError::ExplanationsDisabled {
            revision: disabled.revision,
        })
    );

    let mut tx = graph
        .begin_transaction_with_options(summary_options())
        .unwrap();
    tx.set_input(input, 2).unwrap();
    let summary = tx.commit().unwrap();
    drop(tx);

    history.retain(&summary);
    let explanation = history.why_at(summary.revision, derived).unwrap();
    assert_eq!(explanation.revision, summary.revision);
    assert!(explanation.dependency_paths.is_empty());
    assert_eq!(
        history.dependency_path_at(summary.revision, input.id(), derived.id()),
        Err(AuditHistoryError::DependencyPathsNotRetained {
            revision: summary.revision,
        })
    );

    history.remove_revision(disabled.revision);
    assert_eq!(
        history.why_at(disabled.revision, derived),
        Err(AuditHistoryError::RevisionNotRetained {
            revision: disabled.revision,
        })
    );
}

#[test]
fn retained_dependency_path_reports_missing_paths_after_path_enabled_receipt() {
    let mut graph = Graph::new();
    let mut tx = graph
        .begin_transaction_with_options(path_options())
        .unwrap();
    let input = tx.input::<usize>("input").unwrap();
    tx.set_input(input, 1).unwrap();
    let derived = tx
        .derived(
            "derived",
            DependencyList::new([input.id()]).unwrap(),
            move |ctx| Ok(*ctx.input(input)? + 1),
        )
        .unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    let mut history = AuditHistory::new();
    history.retain(&result);

    assert_eq!(
        history.dependency_path_at(result.revision, derived.id(), input.id()),
        Err(AuditHistoryError::DependencyPathNotFound {
            revision: result.revision,
            from: derived.id(),
            to: input.id(),
        })
    );
    assert_eq!(
        history.why_at(Revision::new(999), derived),
        Err(AuditHistoryError::RevisionNotRetained {
            revision: Revision::new(999),
        })
    );
}
