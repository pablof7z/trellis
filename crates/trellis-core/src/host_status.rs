use crate::{ErrorCategory, ResourceKey, Revision, ScopeId};

/// Host-observed resource outcome carried by a canonical status input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HostResourceOutcome {
    /// The host has not reported a resource outcome.
    Unknown,
    /// The resource is live according to the host.
    Open,
    /// The resource failed outside graph propagation.
    Failed(String),
    /// The resource is closed according to the host.
    Closed,
    /// The host cannot apply the requested transition.
    Unsupported(String),
}

impl HostResourceOutcome {
    /// Returns the model category for host-reported resource status.
    pub const fn category(&self) -> ErrorCategory {
        ErrorCategory::HostResourceStatus
    }
}

/// Canonical host report for a resource command observed outside the graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostResourceStatus<S = HostResourceOutcome> {
    /// Stable graph-visible resource identity.
    pub resource_key: ResourceKey,
    /// Scope associated with the command being reported.
    pub scope: ScopeId,
    /// Graph revision of the resource command the host is reporting for.
    pub command_revision: Revision,
    /// Monotonic host observation revision.
    pub status_revision: Revision,
    /// Application-defined status payload.
    pub status: S,
}

impl<S> HostResourceStatus<S> {
    /// Creates a host resource status input.
    pub fn new(
        resource_key: ResourceKey,
        scope: ScopeId,
        command_revision: Revision,
        status_revision: Revision,
        status: S,
    ) -> Self {
        Self {
            resource_key,
            scope,
            command_revision,
            status_revision,
            status,
        }
    }

    /// Returns the model category for host-reported resource status.
    pub const fn category(&self) -> ErrorCategory {
        ErrorCategory::HostResourceStatus
    }

    /// Maps the status payload while preserving structural identity.
    pub fn map_status<T>(self, map: impl FnOnce(S) -> T) -> HostResourceStatus<T> {
        HostResourceStatus {
            resource_key: self.resource_key,
            scope: self.scope,
            command_revision: self.command_revision,
            status_revision: self.status_revision,
            status: map(self.status),
        }
    }
}
