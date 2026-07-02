use trellis_core::{
    Graph, HostResourceCommandState, HostResourceOutcome, HostResourceStatus, HostStatusClass,
    ResourceKey, Revision, ScopeId, classify_host_resource_status,
};

fn scopes() -> (ScopeId, ScopeId) {
    let mut graph = Graph::<()>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let primary = tx.create_scope("primary").unwrap();
    let other = tx.create_scope("other").unwrap();
    tx.commit().unwrap();
    (primary, other)
}

fn status(
    scope: ScopeId,
    command_revision: u64,
    status: HostResourceOutcome,
) -> HostResourceStatus {
    HostResourceStatus::new(
        ResourceKey::new("device:1"),
        scope,
        Revision::new(command_revision),
        Revision::new(100),
        status,
    )
}

fn live_state(
    scope: ScopeId,
    command_revision: u64,
    scope_owns_resource: bool,
) -> HostResourceCommandState {
    HostResourceCommandState {
        scope,
        command_revision: Revision::new(command_revision),
        resource_is_live: true,
        scope_owns_resource,
    }
}

fn closed_state(scope: ScopeId, command_revision: u64) -> HostResourceCommandState {
    HostResourceCommandState {
        scope,
        command_revision: Revision::new(command_revision),
        resource_is_live: false,
        scope_owns_resource: false,
    }
}

#[test]
fn live_resource_status_classifies_current_and_duplicate_revisions() {
    let (scope, _) = scopes();
    let status = status(scope, 3, HostResourceOutcome::Open);
    let state = live_state(scope, 3, true);

    assert_eq!(
        classify_host_resource_status(&status, Some(state), false),
        HostStatusClass::Current
    );
    assert_eq!(
        classify_host_resource_status(&status, Some(state), true),
        HostStatusClass::Duplicate
    );
}

#[test]
fn live_resource_status_classifies_stale_and_future_revisions() {
    let (scope, _) = scopes();
    let state = live_state(scope, 3, true);

    assert_eq!(
        classify_host_resource_status(
            &status(scope, 2, HostResourceOutcome::Failed("lost".to_owned())),
            Some(state),
            false
        ),
        HostStatusClass::Stale
    );
    assert_eq!(
        classify_host_resource_status(
            &status(scope, 4, HostResourceOutcome::Open),
            Some(state),
            false
        ),
        HostStatusClass::Future
    );
}

#[test]
fn live_resource_status_is_late_for_unknown_resource_or_non_owner_scope() {
    let (scope, other) = scopes();

    assert_eq!(
        classify_host_resource_status(&status(scope, 3, HostResourceOutcome::Open), None, false),
        HostStatusClass::Late
    );
    assert_eq!(
        classify_host_resource_status(
            &status(other, 3, HostResourceOutcome::Open),
            Some(live_state(scope, 3, false)),
            false
        ),
        HostStatusClass::Late
    );
}

#[test]
fn closed_resource_accepts_only_matching_scope_closed_ack() {
    let (scope, other) = scopes();
    let state = closed_state(scope, 3);

    assert_eq!(
        classify_host_resource_status(
            &status(scope, 3, HostResourceOutcome::Closed),
            Some(state),
            false
        ),
        HostStatusClass::Current
    );
    assert_eq!(
        classify_host_resource_status(
            &status(scope, 3, HostResourceOutcome::Open),
            Some(state),
            false
        ),
        HostStatusClass::Late
    );
    assert_eq!(
        classify_host_resource_status(
            &status(other, 3, HostResourceOutcome::Closed),
            Some(state),
            false
        ),
        HostStatusClass::Late
    );
}
