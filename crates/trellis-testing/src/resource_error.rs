use std::collections::BTreeSet;

use trellis_core::{
    ResourceCommandKind, ResourceCommandTrace, ResourceKey, Revision, ScopeId, TransactionId,
};

use crate::{HostStatusClass, HostStatusEvent};

/// Structural context for a resource command observed by the ledger.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceCommandContext {
    /// Resource key targeted by the command.
    pub key: ResourceKey,
    /// Scope associated with the command.
    pub scope: ScopeId,
    /// Transaction that emitted the command.
    pub transaction_id: TransactionId,
    /// Graph revision that emitted the command.
    pub revision: Revision,
    /// Ledger-assigned command generation for this resource key.
    pub generation: u64,
    /// Command operation without application payload.
    pub kind: ResourceCommandKind,
}

/// Structural context for a host status classification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceStatusContext {
    /// Status supplied by the host simulator.
    pub status: HostStatusEvent,
    /// Classification assigned by the ledger.
    pub class: HostStatusClass,
    /// Last command transaction known for this resource key, if any.
    pub last_transaction_id: Option<TransactionId>,
    /// Last command revision known for this resource key, if any.
    pub last_command_revision: Option<Revision>,
}

/// Resource ledger assertion failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResourceLedgerError {
    /// Resource has no owner.
    Orphan {
        /// Resource key.
        key: ResourceKey,
        /// Last known command context for the resource.
        context: Option<ResourceCommandContext>,
    },
    /// Resource was closed without a matching owner.
    DuplicateClose {
        /// Resource key.
        key: ResourceKey,
        /// Command context that attempted the duplicate close.
        context: ResourceCommandContext,
    },
    /// Forbidden resource demand was opened.
    ForbiddenOpen {
        /// Resource key.
        key: ResourceKey,
        /// Command context that opened the forbidden resource.
        context: Option<ResourceCommandContext>,
    },
    /// Resource is still open.
    StillOpen {
        /// Resource key.
        key: ResourceKey,
        /// Last known command context for the resource.
        context: Option<ResourceCommandContext>,
    },
    /// A closed scope still owns resources.
    ClosedScopeOwnsResources {
        /// Closed scope.
        scope: ScopeId,
        /// Resources still owned by the closed scope.
        resources: Vec<ResourceKey>,
        /// Last command contexts for the resources.
        contexts: Vec<ResourceCommandContext>,
    },
    /// Resource command count differed from expectation.
    CountMismatch {
        /// Resource key.
        key: ResourceKey,
        /// Count that differed.
        field: &'static str,
        /// Expected count.
        expected: usize,
        /// Actual count.
        actual: usize,
        /// Last known command context for the resource.
        context: Option<ResourceCommandContext>,
    },
    /// Resource generation differed from expectation.
    GenerationMismatch {
        /// Resource key.
        key: ResourceKey,
        /// Expected generation.
        expected: u64,
        /// Actual generation.
        actual: u64,
        /// Last known command context for the resource.
        context: Option<ResourceCommandContext>,
    },
    /// Resource does not have the expected owners.
    OwnerMismatch {
        /// Resource key.
        key: ResourceKey,
        /// Expected owner set.
        expected: BTreeSet<ScopeId>,
        /// Actual owner set.
        actual: BTreeSet<ScopeId>,
        /// Last known command context for the resource.
        context: Option<ResourceCommandContext>,
    },
    /// Resource command order did not match the expected structural trace.
    CommandOrderMismatch {
        /// Expected command trace.
        expected: Vec<ResourceCommandTrace>,
        /// Actual command trace.
        actual: Vec<ResourceCommandTrace>,
    },
    /// No matching status classification was recorded.
    MissingStatus {
        /// Resource key.
        key: ResourceKey,
        /// Command revision expected in the status record.
        command_revision: Revision,
    },
    /// A status was classified differently than expected.
    StatusClassMismatch {
        /// Recorded status context.
        context: ResourceStatusContext,
        /// Expected classification.
        expected: HostStatusClass,
    },
    /// A host status appeared to mutate ownership for a closed scope.
    StatusMutatedClosedScope {
        /// Scope that should remain closed.
        scope: ScopeId,
        /// Status context that caused the failure.
        context: ResourceStatusContext,
    },
}
