use crate::{AuditEvent, NodeId, ScopeLifecycleKind, ScopeLifecycleTrace};
use std::collections::BTreeSet;

pub(crate) fn stable_node_union(nodes: impl IntoIterator<Item = NodeId>) -> Vec<NodeId> {
    nodes
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub(crate) fn scope_events(events: &[AuditEvent]) -> Vec<ScopeLifecycleTrace> {
    events
        .iter()
        .filter_map(|event| match event {
            AuditEvent::ScopeCreated(scope) => Some(ScopeLifecycleTrace {
                scope: *scope,
                kind: ScopeLifecycleKind::Created,
            }),
            AuditEvent::ScopeClosed(scope) => Some(ScopeLifecycleTrace {
                scope: *scope,
                kind: ScopeLifecycleKind::Closed,
            }),
            _ => None,
        })
        .collect()
}
