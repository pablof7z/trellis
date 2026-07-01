use std::collections::BTreeSet;

use trellis_core::{ResourceCommandTrace, ResourceKey, ScopeId};

/// Resource ledger assertion failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResourceLedgerError {
    /// Resource has no owner.
    Orphan(ResourceKey),
    /// Resource was closed without a matching owner.
    DuplicateClose(ResourceKey),
    /// Forbidden resource demand was opened.
    ForbiddenOpen(ResourceKey),
    /// Resource is still open.
    StillOpen(ResourceKey),
    /// A closed scope still owns resources.
    ClosedScopeOwnsResources {
        /// Closed scope.
        scope: ScopeId,
        /// Resources still owned by the closed scope.
        resources: Vec<ResourceKey>,
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
    },
    /// Resource generation differed from expectation.
    GenerationMismatch {
        /// Resource key.
        key: ResourceKey,
        /// Expected generation.
        expected: u64,
        /// Actual generation.
        actual: u64,
    },
    /// Resource does not have the expected owners.
    OwnerMismatch {
        /// Resource key.
        key: ResourceKey,
        /// Expected owner set.
        expected: BTreeSet<ScopeId>,
        /// Actual owner set.
        actual: BTreeSet<ScopeId>,
    },
    /// Resource command order did not match the expected structural trace.
    CommandOrderMismatch {
        /// Expected command trace.
        expected: Vec<ResourceCommandTrace>,
        /// Actual command trace.
        actual: Vec<ResourceCommandTrace>,
    },
}
