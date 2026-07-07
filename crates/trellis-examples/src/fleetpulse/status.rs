use super::types::{FleetPanel, FleetTarget};

/// Host-reported resource outcome in domain vocabulary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FleetHostOutcome {
    /// The host reports the resource as open.
    Open,
    /// The host reports the resource as closed.
    Closed,
    /// The host failed the resource.
    Failed(String),
    /// The host cannot apply the transition.
    Unsupported(String),
}

/// Domain host status passed back into the dashboard wrapper.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetHostStatus {
    /// Target being reported.
    pub target: FleetTarget,
    /// Panel scope associated with the reported command.
    pub panel: FleetPanel,
    /// Graph revision of the resource command being reported.
    pub command_revision: u64,
    /// Monotonic host observation revision.
    pub status_revision: u64,
    /// Host outcome.
    pub outcome: FleetHostOutcome,
}

/// Classification assigned to a host status.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FleetStatusClass {
    /// Status matches the current resource command.
    Current,
    /// Status duplicates an accepted status.
    Duplicate,
    /// Status targets an older command revision.
    Stale,
    /// Status targets a future command revision.
    Future,
    /// Status targets a closed or non-owned resource.
    Late,
}

impl From<trellis_core::HostStatusClass> for FleetStatusClass {
    fn from(value: trellis_core::HostStatusClass) -> Self {
        match value {
            trellis_core::HostStatusClass::Current => Self::Current,
            trellis_core::HostStatusClass::Duplicate => Self::Duplicate,
            trellis_core::HostStatusClass::Stale => Self::Stale,
            trellis_core::HostStatusClass::Future => Self::Future,
            trellis_core::HostStatusClass::Late => Self::Late,
        }
    }
}

/// Status frame embedded in dashboard output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetStatusFrame {
    /// Target being reported.
    pub target: FleetTarget,
    /// Panel that reported the status.
    pub panel: FleetPanel,
    /// Status classification.
    pub class: FleetStatusClass,
    /// Host outcome.
    pub outcome: FleetHostOutcome,
    /// Command revision the host reported.
    pub command_revision: u64,
    /// Host status revision.
    pub status_revision: u64,
}
