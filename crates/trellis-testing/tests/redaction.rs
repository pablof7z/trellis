#![cfg(feature = "serde")]

use trellis_core::{
    AuditEntry, AuditEvent, Graph, ResourceCoalescedTrace, ResourceCommandKind,
    ResourceCommandTrace, ResourceKey, Revision, ScopeId, TransactionId, TransactionTrace,
};
use trellis_testing::{Scenario, TraceRedactor};

struct MaskResourceKeys;

impl TraceRedactor for MaskResourceKeys {
    fn resource_key(&self, _key: &ResourceKey) -> ResourceKey {
        ResourceKey::new("redacted-issue-161")
    }
}

#[test]
fn redacted_trace_json_removes_resource_keys_from_audit_log() {
    let scope = create_scope();
    let command_key = ResourceKey::new("secret-command-key-issue-161");
    let coalesced_key = ResourceKey::new("secret-coalesced-trace-key-issue-161");
    let audit_key = ResourceKey::new("secret-audit-log-key-issue-161");
    let transaction_id = TransactionId::new(161);
    let revision = Revision::new(9);
    let trace = TransactionTrace {
        transaction_id,
        revision,
        staged_input_changes: Vec::new(),
        changed_inputs: Vec::new(),
        dirty_roots: Vec::new(),
        recomputed_derived_nodes: Vec::new(),
        changed_derived_nodes: Vec::new(),
        recomputed_collection_nodes: Vec::new(),
        changed_collection_nodes: Vec::new(),
        collection_diffs: Vec::new(),
        resource_commands: vec![ResourceCommandTrace {
            key: command_key.clone(),
            scope,
            kind: ResourceCommandKind::Open,
        }],
        resource_coalescences: vec![ResourceCoalescedTrace {
            key: coalesced_key.clone(),
            scope,
            existing_owner_count: 1,
        }],
        output_frames: Vec::new(),
        scope_events: Vec::new(),
        audit_log: vec![AuditEntry {
            transaction_id,
            revision,
            event: AuditEvent::ResourceOpenCoalesced {
                key: audit_key.clone(),
                scope,
                existing_owner_count: 2,
            },
        }],
        phase_trace: Vec::new(),
        invariant_results: Vec::new(),
    };
    let mut scenario = Scenario::new();
    scenario.record_trace("distinctive secrets", trace).unwrap();

    let redacted_trace = scenario
        .redacted(&MaskResourceKeys)
        .traces()
        .into_iter()
        .next()
        .unwrap();
    let json = serde_json::to_string(&redacted_trace).unwrap();

    assert!(json.contains("redacted-issue-161"));
    for original in [
        command_key.as_str(),
        coalesced_key.as_str(),
        audit_key.as_str(),
    ] {
        assert!(
            !json.contains(original),
            "redacted trace JSON leaked {original}: {json}"
        );
    }
}

fn create_scope() -> ScopeId {
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("redaction-regression").unwrap();
    tx.commit().unwrap();
    scope
}
