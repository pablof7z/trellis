use trellis_core::{AuditEvent, AuditExplanationLevel, DependencyList, Graph, TransactionOptions};

fn audit_paths_options() -> TransactionOptions {
    TransactionOptions::default().with_audit_explanations(AuditExplanationLevel::DependencyPaths)
}

#[test]
fn node_explanations_use_only_inputs_that_reach_the_node() {
    let mut graph = Graph::<()>::new();
    let mut tx = graph
        .begin_transaction_with_options(audit_paths_options())
        .unwrap();
    let first = tx.input::<usize>("first").unwrap();
    let second = tx.input::<usize>("second").unwrap();
    tx.set_input(first, 1).unwrap();
    tx.set_input(second, 10).unwrap();
    let first_derived = tx
        .derived(
            "first-derived",
            DependencyList::new([first.id()]).unwrap(),
            move |ctx| Ok(*ctx.input(first)? + 1),
        )
        .unwrap();
    let second_derived = tx
        .derived(
            "second-derived",
            DependencyList::new([second.id()]).unwrap(),
            move |ctx| Ok(*ctx.input(second)? + 1),
        )
        .unwrap();
    tx.commit().unwrap();
    drop(tx);

    assert_eq!(
        graph.why_changed(first_derived).unwrap().input_causes,
        vec![first.id()]
    );
    assert_eq!(
        graph.why_changed(second_derived).unwrap().input_causes,
        vec![second.id()]
    );
}

#[test]
fn equal_input_write_does_not_replace_last_change_explanation() {
    let mut graph = Graph::new();
    let mut tx = graph
        .begin_transaction_with_options(audit_paths_options())
        .unwrap();
    let input = tx.input::<String>("input").unwrap();
    tx.set_input(input, "same".to_owned()).unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph
        .begin_transaction_with_options(audit_paths_options())
        .unwrap();
    tx.set_input(input, "same".to_owned()).unwrap();
    tx.commit().unwrap();
    drop(tx);

    let explanation = graph.why_changed(input).unwrap();
    assert_eq!(explanation.event, AuditEvent::InputChanged(input.id()));
    assert_eq!(explanation.input_causes, vec![input.id()]);
}
