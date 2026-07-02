use std::collections::BTreeSet;

use trellis_core::{
    DependencyList, Graph, HostResourceOutcome, InputNode, ResourceCommandKind,
    ResourceCommandTrace, ResourceKey, ResourcePlan, Revision, ScopeId,
};
use trellis_testing::{
    FakeHost, HostStatusClass, HostStatusEvent, ResourceLedger, ResourceLedgerError,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Command {
    Open(u8),
}

struct TestGraph {
    graph: Graph<Command>,
    source: InputNode<BTreeSet<u8>>,
    status: InputNode<HostStatusEvent>,
    scope: ScopeId,
}

fn members(values: &[u8]) -> BTreeSet<u8> {
    values.iter().copied().collect()
}

fn key(value: u8) -> ResourceKey {
    ResourceKey::new(format!("test:{value}"))
}

fn build_graph(initial: BTreeSet<u8>) -> (TestGraph, trellis_core::TransactionResult<Command>) {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let source = tx.input::<BTreeSet<u8>>("source").unwrap();
    let status = tx.input::<HostStatusEvent>("resource-status").unwrap();
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
    let result = tx.commit().unwrap();
    drop(tx);

    (
        TestGraph {
            graph,
            source,
            status,
            scope,
        },
        result,
    )
}

fn feed_host_status(
    target: &mut TestGraph,
    status: HostStatusEvent,
) -> trellis_core::TransactionResult<Command> {
    let mut tx = target.graph.begin_transaction().unwrap();
    tx.set_input(target.status, status.clone()).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);
    assert_eq!(
        target.graph.input_value(target.status).unwrap(),
        Some(&status)
    );
    result
}

fn set_source(
    target: &mut TestGraph,
    values: BTreeSet<u8>,
) -> trellis_core::TransactionResult<Command> {
    let mut tx = target.graph.begin_transaction().unwrap();
    tx.set_input(target.source, values).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);
    target.graph.assert_incremental_equals_full().unwrap();
    result
}

#[test]
fn resource_ledger_detects_lifecycle_and_status_classes() {
    let (mut target, initial) = build_graph(members(&[1, 2]));
    let broad_key = ResourceKey::wildcard("all-devices");
    assert_eq!(
        broad_key.segments().collect::<Vec<_>>(),
        vec!["wildcard", "all-devices"]
    );

    let mut forbidden = ResourceLedger::new();
    forbidden.mark_forbidden_unless_explicit(key(1));
    forbidden.apply_result(&initial);
    let error = forbidden.assert_no_wildcard_resource_opened().unwrap_err();
    assert!(matches!(
        error,
        ResourceLedgerError::ForbiddenOpen {
            key: failed_key,
            context: Some(context),
        } if failed_key == key(1)
            && context.scope == target.scope
            && context.transaction_id == initial.transaction_id
            && context.revision == initial.revision
    ));

    let mut ledger = ResourceLedger::new();
    ledger.mark_forbidden_unless_explicit(broad_key);
    ledger.apply_result(&initial);
    ledger.assert_all_resources_have_owner().unwrap();
    ledger.assert_no_wildcard_resource_opened().unwrap();
    ledger.assert_resource_opened_once(&key(1)).unwrap();
    let first = ledger.snapshot(&key(1)).unwrap();
    assert!(first.is_open);
    assert_eq!(first.last_transaction_id, initial.transaction_id);
    assert_eq!(first.command_revision, initial.revision);
    assert_eq!(first.current_command.as_ref(), Some(&Command::Open(1)));

    let shrink = set_source(&mut target, members(&[1]));
    ledger.apply_result(&shrink);
    ledger.assert_resource_not_open(&key(2)).unwrap();
    ledger.assert_resource_closed_once(&key(2)).unwrap();
    ledger.assert_resource_generation(&key(2), 2).unwrap();
    ledger.assert_no_duplicate_close().unwrap();
    ledger
        .assert_command_order(&[
            ResourceCommandTrace {
                key: key(1),
                scope: target.scope,
                kind: ResourceCommandKind::Open,
            },
            ResourceCommandTrace {
                key: key(2),
                scope: target.scope,
                kind: ResourceCommandKind::Open,
            },
            ResourceCommandTrace {
                key: key(2),
                scope: target.scope,
                kind: ResourceCommandKind::Close,
            },
        ])
        .unwrap();

    let status = HostStatusEvent {
        resource_key: key(1),
        scope: target.scope,
        command_revision: Revision::new(0),
        status_revision: Revision::new(100),
        status: HostResourceOutcome::Open,
    };
    assert_eq!(ledger.classify_status(status), HostStatusClass::Stale);
    ledger
        .assert_status_is_stale(&key(1), Revision::new(0))
        .unwrap();

    let current = HostStatusEvent {
        resource_key: key(1),
        scope: target.scope,
        command_revision: initial.revision,
        status_revision: Revision::new(101),
        status: HostResourceOutcome::Open,
    };
    assert_eq!(
        ledger.classify_status(current.clone()),
        HostStatusClass::Current
    );
    assert_eq!(ledger.classify_status(current), HostStatusClass::Duplicate);

    let failed = HostStatusEvent {
        resource_key: key(1),
        scope: target.scope,
        command_revision: initial.revision,
        status_revision: Revision::new(102),
        status: HostResourceOutcome::Failed("host failed".to_owned()),
    };
    assert_eq!(ledger.classify_status(failed), HostStatusClass::Current);

    let future = HostStatusEvent {
        resource_key: key(1),
        scope: target.scope,
        command_revision: Revision::new(10),
        status_revision: Revision::new(103),
        status: HostResourceOutcome::Open,
    };
    assert_eq!(ledger.classify_status(future), HostStatusClass::Future);

    let closed_ack = HostStatusEvent {
        resource_key: key(2),
        scope: target.scope,
        command_revision: shrink.revision,
        status_revision: Revision::new(104),
        status: HostResourceOutcome::Closed,
    };
    assert_eq!(ledger.classify_status(closed_ack), HostStatusClass::Current);

    let mut host = FakeHost::new();
    let host_result = set_source(&mut target, members(&[1, 3]));
    let events = host.apply_result(&mut ledger, &host_result);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].class, HostStatusClass::Current);
    let status_result = feed_host_status(&mut target, events[0].clone().into_status());
    assert_eq!(status_result.changed_inputs, vec![target.status.id()]);
    assert_eq!(
        host.open_failed(
            &mut ledger,
            key(3),
            target.scope,
            host_result.revision,
            "host failed"
        )
        .class,
        HostStatusClass::Current
    );
    let recovered = host.open_succeeded(&mut ledger, key(3), target.scope, host_result.revision);
    assert_eq!(recovered.class, HostStatusClass::Current);
    assert_eq!(
        host.duplicate_status(&mut ledger, &recovered).class,
        HostStatusClass::Duplicate
    );

    let mut tx = target.graph.begin_transaction().unwrap();
    tx.close_scope(target.scope).unwrap();
    let closed = tx.commit().unwrap();
    drop(tx);
    ledger.apply_result(&closed);
    assert_eq!(
        host.open_succeeded(&mut ledger, key(1), target.scope, initial.revision)
            .class,
        HostStatusClass::Late
    );
    ledger
        .assert_status_did_not_resurrect_closed_scope(target.scope)
        .unwrap();
}

#[test]
fn fake_host_close_status_is_current_for_the_close_command() {
    let (mut target, initial) = build_graph(members(&[9]));
    let mut ledger = ResourceLedger::new();
    let mut host = FakeHost::new();
    let opened = host.apply_result(&mut ledger, &initial);
    assert_eq!(opened.len(), 1);
    assert_eq!(opened[0].class, HostStatusClass::Current);

    let closed = set_source(&mut target, BTreeSet::new());
    let closed_statuses = host.apply_result(&mut ledger, &closed);
    assert_eq!(closed_statuses.len(), 1);
    assert_eq!(closed_statuses[0].status.resource_key, key(9));
    assert_eq!(closed_statuses[0].status.command_revision, closed.revision);
    assert_eq!(
        closed_statuses[0].status.status,
        HostResourceOutcome::Closed
    );
    assert_eq!(closed_statuses[0].class, HostStatusClass::Current);
    ledger.assert_resource_not_open(&key(9)).unwrap();
}
