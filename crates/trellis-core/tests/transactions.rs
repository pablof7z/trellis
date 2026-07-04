use trellis_core::{AuditEvent, DependencyList, Graph, GraphError, NodeId, TransactionOptions};

fn input_graph<T>(name: &str) -> (Graph, trellis_core::InputNode<T>)
where
    T: Clone + PartialEq + Send + Sync + 'static,
{
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let input = tx.input::<T>(name).unwrap();
    tx.commit().unwrap();
    drop(tx);
    (graph, input)
}

#[test]
fn single_input_update_increments_revision_once() {
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let input = tx.input::<String>("name").unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(input, "trellis".to_owned()).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert_eq!(result.revision.get(), 2);
    assert_eq!(result.changed_inputs, vec![input.id()]);
    assert_eq!(result.audit_log.len(), 1);
    assert_eq!(
        result.audit_log[0].event,
        AuditEvent::InputChanged(input.id())
    );
    assert_eq!(graph.revision().get(), 2);
    assert_eq!(
        graph.input_value(input).unwrap(),
        Some(&"trellis".to_owned())
    );
    assert_eq!(
        graph.node_meta(input).unwrap().last_changed_revision(),
        result.revision
    );
}

#[test]
fn multiple_input_updates_increment_revision_once() {
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let first = tx.input::<String>("first").unwrap();
    let second = tx.input::<u64>("second").unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(first, "a".to_owned()).unwrap();
    tx.set_input(second, 42).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert_eq!(result.revision.get(), 2);
    assert_eq!(graph.revision().get(), 2);
    assert_eq!(result.changed_inputs, vec![first.id(), second.id()]);
    assert_eq!(graph.input_value(first).unwrap(), Some(&"a".to_owned()));
    assert_eq!(graph.input_value(second).unwrap(), Some(&42));
}

#[test]
fn equal_input_write_is_noop_by_default() {
    let (mut graph, input) = input_graph::<String>("name");

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(input, "same".to_owned()).unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(input, "same".to_owned()).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert_eq!(result.revision.get(), 2);
    assert!(result.changed_inputs.is_empty());
    assert_eq!(result.audit_log.len(), 1);
    assert_eq!(
        result.audit_log[0].event,
        AuditEvent::InputUnchanged(input.id())
    );
    assert_eq!(graph.revision().get(), 2);
}

#[test]
fn equal_input_write_can_be_configured_as_change() {
    let (mut graph, input) = input_graph::<String>("name");

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(input, "same".to_owned()).unwrap();
    tx.commit().unwrap();
    drop(tx);

    let options = TransactionOptions::default().with_skip_equal_inputs(false);
    let mut tx = graph.begin_transaction_with_options(options).unwrap();
    tx.set_input(input, "same".to_owned()).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert_eq!(result.revision.get(), 3);
    assert_eq!(result.changed_inputs, vec![input.id()]);
    assert_eq!(
        result.audit_log[0].event,
        AuditEvent::InputChanged(input.id())
    );
    assert_eq!(graph.revision().get(), 3);
}

#[test]
fn closed_transaction_cannot_be_reused() {
    let (mut graph, input) = input_graph::<String>("name");

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(input, "value".to_owned()).unwrap();
    let transaction_id = tx.id();
    tx.commit().unwrap();

    assert_eq!(
        tx.set_input(input, "again".to_owned()).unwrap_err(),
        GraphError::TransactionClosed(transaction_id)
    );
    assert_eq!(
        tx.commit().unwrap_err(),
        GraphError::TransactionClosed(transaction_id)
    );
}

#[test]
fn failed_transaction_does_not_partially_commit() {
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let first = tx.input::<String>("first").unwrap();
    let second = tx.input::<String>("second").unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(first, "valid".to_owned()).unwrap();
    assert_eq!(
        tx.set_input_by_id(second.id(), 10_u64).unwrap_err(),
        GraphError::WrongInputType(second.id())
    );
    assert_eq!(
        tx.commit().unwrap_err(),
        GraphError::WrongInputType(second.id())
    );
    drop(tx);

    assert_eq!(graph.revision().get(), 1);
    assert_eq!(graph.input_value(first).unwrap(), None);
    assert_eq!(graph.input_value(second).unwrap(), None);
}

#[test]
fn audit_log_order_is_stable_by_node_id() {
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let first = tx.input::<String>("first").unwrap();
    let second = tx.input::<String>("second").unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(second, "b".to_owned()).unwrap();
    tx.set_input(first, "a".to_owned()).unwrap();
    let result = tx.commit().unwrap();

    let audit_nodes: Vec<NodeId> = result
        .audit_log
        .iter()
        .filter_map(|entry| match entry.event {
            AuditEvent::InputChanged(node) | AuditEvent::InputUnchanged(node) => Some(node),
            _ => None,
        })
        .collect();
    assert_eq!(audit_nodes, vec![first.id(), second.id()]);
}

#[test]
fn non_input_node_cannot_be_set_as_input() {
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let input = tx.input::<String>("input").unwrap();
    let derived = tx
        .derived::<String>(
            "derived",
            DependencyList::new([input.id()]).unwrap(),
            |_| Ok(String::new()),
        )
        .unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    assert_eq!(
        tx.set_input_by_id(derived.id(), "value".to_owned())
            .unwrap_err(),
        GraphError::NotInputNode(derived.id())
    );
}

#[test]
fn preview_does_not_mutate_graph() {
    let (mut graph, input) = input_graph::<String>("name");

    let before_dump = graph.debug_dump();
    let before_revision = graph.revision();

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(input, "changed".to_owned()).unwrap();
    let result = tx.preview().unwrap();

    // preview predicts the next revision but must not apply it.
    assert_eq!(result.revision.get(), before_revision.get() + 1);
    assert_eq!(result.changed_inputs, vec![input.id()]);

    // The real graph is untouched: same dump, same revision, input unchanged.
    assert_eq!(graph.debug_dump(), before_dump);
    assert_eq!(graph.revision(), before_revision);
    assert_eq!(graph.input_value(input).unwrap(), None);
}

#[test]
fn preview_matches_commit() {
    fn build() -> (Graph, trellis_core::InputNode<String>) {
        input_graph::<String>("name")
    }

    let (mut preview_graph, preview_input) = build();
    let (mut commit_graph, commit_input) = build();
    assert_eq!(preview_input.id(), commit_input.id());

    let mut preview_tx = preview_graph.begin_transaction().unwrap();
    preview_tx
        .set_input(preview_input, "value".to_owned())
        .unwrap();
    let previewed = preview_tx.preview().unwrap();

    let mut commit_tx = commit_graph.begin_transaction().unwrap();
    commit_tx
        .set_input(commit_input, "value".to_owned())
        .unwrap();
    let committed = commit_tx.commit().unwrap();
    drop(commit_tx);

    // preview predicts commit exactly: same commands, frames, changed sets,
    // revision, audit — the whole TransactionResult.
    assert_eq!(previewed, committed);
    // And the commit really landed while the preview did not.
    assert_eq!(
        commit_graph.input_value(commit_input).unwrap(),
        Some(&"value".to_owned())
    );
    assert_eq!(preview_graph.input_value(preview_input).unwrap(), None);
}

#[test]
fn preview_can_be_followed_by_a_real_commit() {
    let (mut graph, input) = input_graph::<String>("name");

    // A dry run leaves nothing behind...
    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(input, "value".to_owned()).unwrap();
    let previewed = tx.preview().unwrap();

    // ...so a fresh transaction staging the same change commits cleanly and
    // produces exactly what the preview predicted.
    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(input, "value".to_owned()).unwrap();
    let committed = tx.commit().unwrap();
    drop(tx);

    // The follow-up commit lands the expected state. (Its transaction id is one
    // higher than the preview's, since `begin_transaction` advances the graph's
    // tx counter each time it is called — a begin-time bookkeeping step that is
    // independent of whether a tx previews or commits.)
    assert_eq!(committed.revision.get(), 2);
    assert_eq!(committed.changed_inputs, vec![input.id()]);
    assert_eq!(graph.revision().get(), 2);
    assert_eq!(graph.input_value(input).unwrap(), Some(&"value".to_owned()));

    // Everything the preview predicted holds, except the transaction id that
    // begin-time bookkeeping advanced.
    assert_eq!(previewed.revision, committed.revision);
    assert_eq!(previewed.changed_inputs, committed.changed_inputs);
    assert_eq!(previewed.output_frames, committed.output_frames);
    assert_eq!(previewed.resource_plan, committed.resource_plan);
}

#[test]
fn handles_from_aborted_transactions_do_not_alias_future_nodes() {
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let stale = tx.input::<String>("stale").unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    let committed = tx.input::<String>("committed").unwrap();
    tx.commit().unwrap();
    drop(tx);

    assert_ne!(stale.id(), committed.id());

    let mut tx = graph.begin_transaction().unwrap();
    assert_eq!(
        tx.set_input(stale, "wrong".to_owned()).unwrap_err(),
        GraphError::UnknownNode(stale.id())
    );
}
