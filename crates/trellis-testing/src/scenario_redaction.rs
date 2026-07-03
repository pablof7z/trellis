use trellis_core::{AuditEvent, TransactionTrace};

use crate::scenario::TraceRedactor;

pub(crate) fn redact_trace(
    trace: &TransactionTrace,
    redactor: &impl TraceRedactor,
) -> TransactionTrace {
    let mut trace = trace.clone();
    for command in &mut trace.resource_commands {
        command.key = redactor.resource_key(&command.key);
    }
    for coalesced in &mut trace.resource_coalescences {
        coalesced.key = redactor.resource_key(&coalesced.key);
    }
    for entry in &mut trace.audit_log {
        redact_audit_event(&mut entry.event, redactor);
    }
    for result in &mut trace.invariant_results {
        result.name = redactor.invariant_name(&result.name);
    }
    trace
}

fn redact_audit_event(event: &mut AuditEvent, redactor: &impl TraceRedactor) {
    match event {
        AuditEvent::InputChanged(_)
        | AuditEvent::InputUnchanged(_)
        | AuditEvent::DerivedChanged(_)
        | AuditEvent::CollectionChanged(_)
        | AuditEvent::ScopeCreated(_)
        | AuditEvent::ScopeClosed(_)
        | AuditEvent::NodeCreated(_)
        | AuditEvent::NodeAttached { .. } => {}
        AuditEvent::ResourceOpenCoalesced { key, .. } => {
            *key = redactor.resource_key(key);
        }
    }
}
