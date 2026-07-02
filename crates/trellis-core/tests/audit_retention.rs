use trellis_core::{AuditEvent, AuditExplanationLevel, DependencyList, Graph, TransactionOptions};

fn path_options() -> TransactionOptions {
    TransactionOptions::default().with_audit_explanations(AuditExplanationLevel::DependencyPaths)
}

fn disabled_options() -> TransactionOptions {
    TransactionOptions::default().with_audit_explanations(AuditExplanationLevel::Disabled)
}

#[test]
fn default_audit_explanations_keep_bounded_summaries_without_paths() {
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
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

    assert!(
        result
            .audit_log
            .iter()
            .any(|entry| entry.event == AuditEvent::InputChanged(input.id()))
    );
    let explanation = graph.why_changed(derived).unwrap();
    assert_eq!(explanation.event, AuditEvent::DerivedChanged(derived.id()));
    assert!(explanation.input_causes.is_empty());
    assert!(explanation.dependency_paths.is_empty());
}

#[test]
fn disabled_audit_explanations_clear_latest_graph_explanations() {
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
    tx.commit().unwrap();
    drop(tx);
    assert!(graph.why_changed(derived).is_some());

    let mut tx = graph
        .begin_transaction_with_options(disabled_options())
        .unwrap();
    tx.set_input(input, 2).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert!(
        result
            .audit_log
            .iter()
            .any(|entry| entry.event == AuditEvent::InputChanged(input.id()))
    );
    assert!(graph.why_changed(input).is_none());
    assert!(graph.why_changed(derived).is_none());
}

#[test]
fn dependency_path_returns_shortest_stable_path() {
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let input = tx.input::<usize>("input").unwrap();
    tx.set_input(input, 1).unwrap();
    let long_first = tx
        .derived(
            "long-first",
            DependencyList::new([input.id()]).unwrap(),
            move |ctx| Ok(*ctx.input(input)? + 1),
        )
        .unwrap();
    let long_second = tx
        .derived(
            "long-second",
            DependencyList::new([long_first.id()]).unwrap(),
            move |ctx| Ok(*ctx.derived(long_first)? + 1),
        )
        .unwrap();
    let target = tx
        .derived(
            "target",
            DependencyList::new([input.id(), long_second.id()]).unwrap(),
            move |ctx| Ok(*ctx.input(input)? + *ctx.derived(long_second)?),
        )
        .unwrap();
    tx.commit().unwrap();
    drop(tx);

    assert_eq!(
        graph.dependency_path(input.id(), target.id()),
        Some(vec![input.id(), target.id()])
    );
}
