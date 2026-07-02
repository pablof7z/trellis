use std::collections::BTreeSet;

use trellis_core::{
    AuditEvent, DependencyList, DeriveError, ErrorCategory, ErrorTarget, Graph, GraphError,
    HostResourceOutcome, HostResourceStatus, OutputError, PlanError, ResourceKey, ResourcePlan,
    Revision,
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

#[test]
fn plan_error_does_not_emit_partial_plan_or_mutate_ownership() {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let collection = tx
        .set_collection("resources", DependencyList::empty(), |_| Ok(set(&["a"])))
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
        Err(PlanError::message("planner failed"))
    })
    .unwrap();
    let error = tx.commit().unwrap_err();
    drop(tx);

    assert_eq!(
        error,
        GraphError::PlanFailed(scope, PlanError::message("planner failed"))
    );
    assert_eq!(error.category(), ErrorCategory::PlanError);
    assert_eq!(error.audit_event().target, ErrorTarget::Scope(scope));
    assert!(graph.resource_owners(&key("a")).is_none());
    assert_eq!(graph.revision().get(), 0);
}

#[test]
fn output_error_does_not_corrupt_graph_state() {
    let mut graph = Graph::<()>::new();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let source = tx.input::<String>("source").unwrap();
    tx.set_input(source, "ok".to_owned()).unwrap();
    let output = tx
        .materialized_output::<String>(
            "output",
            scope,
            DependencyList::new([source.id()]).unwrap(),
            |_| Err(OutputError::message("output failed")),
        )
        .unwrap();
    let error = tx.commit().unwrap_err();
    drop(tx);

    assert_eq!(
        error,
        GraphError::OutputFailed(output.key(), OutputError::message("output failed"))
    );
    assert_eq!(error.category(), ErrorCategory::OutputError);
    assert_eq!(
        error.audit_event().target,
        ErrorTarget::Output(output.key())
    );
    assert_eq!(graph.revision().get(), 0);
    assert_eq!(
        graph.input_value(source).unwrap_err(),
        GraphError::UnknownNode(source.id())
    );
    assert!(graph.output_meta(output.key()).is_none());
}

#[test]
fn host_resource_failure_is_modeled_as_canonical_input() {
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let status = tx.input::<HostResourceStatus>("resource-status").unwrap();
    let failed = HostResourceStatus::new(
        key("a"),
        scope,
        Revision::new(1),
        Revision::new(2),
        HostResourceOutcome::Failed("connection refused".to_owned()),
    );
    tx.set_input(status, failed.clone()).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert_eq!(result.changed_inputs, vec![status.id()]);
    assert_eq!(graph.input_value(status).unwrap(), Some(&failed));
    assert_eq!(failed.category(), ErrorCategory::HostResourceStatus);
    assert_eq!(
        HostResourceOutcome::Failed("connection refused".to_owned()).category(),
        ErrorCategory::HostResourceStatus
    );
}

#[test]
fn duplicate_host_status_is_an_unchanged_canonical_input() {
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let status = tx.input::<HostResourceStatus>("resource-status").unwrap();
    let current = HostResourceStatus::new(
        key("a"),
        scope,
        Revision::new(1),
        Revision::new(2),
        HostResourceOutcome::Open,
    );
    tx.set_input(status, current.clone()).unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(status, current).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert!(result.changed_inputs.is_empty());
    assert_eq!(
        result.audit_log[0].event,
        AuditEvent::InputUnchanged(status.id())
    );
}

#[test]
fn unsupported_resource_transition_is_host_status_not_graph_failure() {
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let status = tx.input::<HostResourceStatus>("resource-status").unwrap();
    let unsupported = HostResourceStatus::new(
        key("a"),
        scope,
        Revision::new(1),
        Revision::new(2),
        HostResourceOutcome::Unsupported("replace unsupported".to_owned()),
    );
    tx.set_input(status, unsupported.clone()).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert_eq!(result.changed_inputs, vec![status.id()]);
    assert_eq!(graph.revision(), result.revision);
    assert_eq!(graph.input_value(status).unwrap(), Some(&unsupported));
}

#[test]
fn error_categories_and_audit_events_are_deterministic() {
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let source = tx.input::<u64>("source").unwrap();
    tx.set_input(source, 0).unwrap();
    let derived = tx
        .derived(
            "derived",
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| {
                if *ctx.input(source)? == 0 {
                    Err(DeriveError::message("zero"))
                } else {
                    Ok(1)
                }
            },
        )
        .unwrap();
    let first = tx.commit().unwrap_err();
    drop(tx);

    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let source = tx.input::<u64>("source").unwrap();
    tx.set_input(source, 0).unwrap();
    let derived_again = tx
        .derived(
            "derived",
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| {
                if *ctx.input(source)? == 0 {
                    Err(DeriveError::message("zero"))
                } else {
                    Ok(1)
                }
            },
        )
        .unwrap();
    let second = tx.commit().unwrap_err();

    assert_eq!(derived.id(), derived_again.id());
    assert_eq!(first, second);
    assert_eq!(first.category(), ErrorCategory::DeriveError);
    assert_eq!(first.audit_event(), second.audit_event());
    assert_eq!(first.audit_event().target, ErrorTarget::Node(derived.id()));
}
