use crate::{NodeId, Revision, ScopeId, TransactionId};

/// Configuration for committing input changes.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct TransactionOptions {
    /// When true, setting an input to an equal value does not advance revision.
    pub skip_equal_inputs: bool,
}

impl Default for TransactionOptions {
    fn default() -> Self {
        Self {
            skip_equal_inputs: true,
        }
    }
}

/// Deterministic audit record for an input transaction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditEntry {
    /// Transaction that produced this audit entry.
    pub transaction_id: TransactionId,
    /// Graph revision after the transaction committed.
    pub revision: Revision,
    /// Audited event.
    pub event: AuditEvent,
}

/// Deterministic transaction event emitted in the audit log.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum AuditEvent {
    /// An input value changed or was configured to count as changed.
    InputChanged(NodeId),
    /// An equal input write was skipped by transaction options.
    InputUnchanged(NodeId),
    /// A derived value changed.
    DerivedChanged(NodeId),
    /// A collection value changed and produced a structural diff.
    CollectionChanged(NodeId),
    /// A scope was created.
    ScopeCreated(ScopeId),
    /// A node was created.
    NodeCreated(NodeId),
    /// A node was attached to a scope.
    NodeAttached {
        /// Attached node.
        node: NodeId,
        /// Owning scope.
        scope: ScopeId,
    },
}

/// Result returned by a committed input transaction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionResult {
    /// Committed transaction id.
    pub transaction_id: TransactionId,
    /// Graph revision after commit.
    pub revision: Revision,
    /// Input nodes that changed in stable node-id order.
    pub changed_inputs: Vec<NodeId>,
    /// Derived nodes that changed in deterministic topological order.
    pub changed_derived_nodes: Vec<NodeId>,
    /// Collection nodes that changed in deterministic topological order.
    pub changed_collection_nodes: Vec<NodeId>,
    /// Deterministic audit entries for staged input writes.
    pub audit_log: Vec<AuditEntry>,
}
