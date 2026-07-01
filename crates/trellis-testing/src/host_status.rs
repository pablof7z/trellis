use trellis_core::{HostResourceStatus, ResourceKey, Revision, ScopeId};

/// Explicit host status event fed to tests after plan application.
pub type HostStatusEvent = HostResourceStatus;

/// Recorded classification for a host status event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostStatusRecord {
    /// Status supplied by the fake host.
    pub status: HostStatusEvent,
    /// Deterministic classification assigned by the ledger.
    pub class: HostStatusClass,
}

/// Classification for host status delivery.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum HostStatusClass {
    /// Status matches the current resource/scope/revision.
    Current,
    /// Status duplicates the last accepted status revision.
    Duplicate,
    /// Status targets an old command revision.
    Stale,
    /// Status targets a command revision newer than the ledger has observed.
    Future,
    /// Status targets a scope that no longer owns the resource.
    Late,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct HostStatusIdentity {
    resource_key: ResourceKey,
    scope: ScopeId,
    command_revision: Revision,
    status_revision: Revision,
}

impl From<&HostStatusEvent> for HostStatusIdentity {
    fn from(status: &HostStatusEvent) -> Self {
        Self {
            resource_key: status.resource_key.clone(),
            scope: status.scope,
            command_revision: status.command_revision,
            status_revision: status.status_revision,
        }
    }
}
