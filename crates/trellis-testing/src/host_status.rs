use trellis_core::{HostResourceStatus, ResourceKey, Revision, ScopeId, TransactionId};

pub use trellis_core::HostStatusClass;

/// Explicit host status event fed to tests after plan application.
pub type HostStatusEvent = HostResourceStatus;

/// Recorded classification for a host status event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostStatusRecord {
    /// Status supplied by the fake host.
    pub status: HostStatusEvent,
    /// Deterministic classification assigned by the ledger.
    pub class: HostStatusClass,
    /// Last command transaction known for this resource key, if any.
    pub last_transaction_id: Option<TransactionId>,
    /// Last command revision known for this resource key, if any.
    pub last_command_revision: Option<Revision>,
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
