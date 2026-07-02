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

/// Classification for host status delivery relative to graph command state.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum HostStatusClass {
    /// Status matches the current resource/scope/revision.
    Current,
    /// Status duplicates an already accepted status revision.
    Duplicate,
    /// Status targets an old command revision.
    Stale,
    /// Status targets a command revision newer than the graph has observed.
    Future,
    /// Status targets a resource or scope that is no longer current.
    Late,
}

/// Graph command state needed to classify a host resource status.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct HostResourceCommandState {
    /// Scope associated with the latest command for this resource.
    pub scope: ScopeId,
    /// Latest command revision known for this resource.
    pub command_revision: Revision,
    /// Whether the resource is still live in graph state.
    pub resource_is_live: bool,
    /// Whether the status scope currently owns the live resource.
    pub scope_owns_resource: bool,
}

/// Classifies a host resource status against graph command state.
pub fn classify_host_resource_status(
    status: &HostResourceStatus,
    state: Option<HostResourceCommandState>,
    duplicate: bool,
) -> HostStatusClass {
    let Some(state) = state else {
        return HostStatusClass::Late;
    };

    if !state.resource_is_live {
        if matches!(status.status, HostResourceOutcome::Closed) && state.scope == status.scope {
            return classify_revision(status.command_revision, state.command_revision, duplicate);
        }
        return HostStatusClass::Late;
    }

    if !state.scope_owns_resource {
        return HostStatusClass::Late;
    }

    classify_revision(status.command_revision, state.command_revision, duplicate)
}

fn classify_revision(
    status_revision: Revision,
    command_revision: Revision,
    duplicate: bool,
) -> HostStatusClass {
    if status_revision < command_revision {
        HostStatusClass::Stale
    } else if status_revision > command_revision {
        HostStatusClass::Future
    } else if duplicate {
        HostStatusClass::Duplicate
    } else {
        HostStatusClass::Current
    }
}
