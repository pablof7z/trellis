use std::collections::{BTreeMap, BTreeSet};

/// Opaque handle for an open ControlPlane Lite controller.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ControlPlaneHandle(pub u64);

/// Desired application configuration owned by the app.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesiredAppConfig {
    /// Stable application id.
    pub app_id: String,
    /// Container image name.
    pub image: String,
    /// Desired rollout version.
    pub version: String,
    /// Desired worker replica count.
    pub replicas: u32,
    /// Desired public port.
    pub port: u16,
    /// Desired volume names.
    pub volumes: BTreeSet<String>,
    /// Desired secret names.
    pub secrets: BTreeSet<String>,
}

/// Host resource controlled by ControlPlane Lite.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ControlResource {
    /// Worker process.
    Worker {
        /// Stable application id.
        app_id: String,
        /// Worker ordinal.
        ordinal: u32,
        /// Container image name.
        image: String,
        /// Rollout version.
        version: String,
    },
    /// Public port binding.
    Port {
        /// Stable application id.
        app_id: String,
        /// Bound port.
        port: u16,
    },
    /// App volume.
    Volume {
        /// Stable application id.
        app_id: String,
        /// Volume name.
        name: String,
    },
    /// App secret.
    Secret {
        /// Stable application id.
        app_id: String,
        /// Secret name.
        name: String,
    },
    /// Retry job for a failed desired resource.
    RetryJob {
        /// Stable application id.
        app_id: String,
        /// Stable failed-resource id.
        target: String,
    },
}

/// Host status for an actual resource.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ControlResourceStatus {
    /// Resource is being created or updated.
    Creating,
    /// Resource is ready.
    Ready,
    /// Resource failed with a host-provided reason.
    Failed(String),
}

/// Domain event applied to an open controller.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ControlPlaneEvent {
    /// Replace the desired app config.
    ReplaceConfig(DesiredAppConfig),
    /// Apply host resource status as canonical input.
    ApplyResourceStatus {
        /// Status target resource.
        resource: ControlResource,
        /// Host status.
        status: ControlResourceStatus,
    },
    /// Clear one host resource status.
    ClearResourceStatus {
        /// Status target resource.
        resource: ControlResource,
    },
    /// Replace all host statuses.
    ReplaceStatuses(BTreeMap<ControlResource, ControlResourceStatus>),
}

/// Host command payload used by Trellis resource planning.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum ControlCommand {
    /// Open the given control-plane resource.
    Open(ControlResource),
}

/// Typed effect emitted to the host executor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ControlEffect {
    /// Create or update the given resource.
    Open(ControlResource),
    /// Delete the given resource.
    Close(ControlResource),
}

/// One materialized resource row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControlResourceView {
    /// Stable resource id.
    pub resource_id: String,
    /// Resource kind.
    pub kind: String,
    /// Whether this is part of desired app config.
    pub desired: bool,
    /// Latest host status, if any.
    pub status: Option<ControlResourceStatus>,
}

/// One materialized status condition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControlCondition {
    /// Condition kind.
    pub kind: String,
    /// Condition status.
    pub status: String,
    /// Human-readable message.
    pub message: String,
}

/// Materialized ControlPlane Lite output.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ControlSnapshot {
    /// Open app id, if any.
    pub app_id: Option<String>,
    /// Desired resource count before retry jobs.
    pub desired_resources: usize,
    /// Retry job count.
    pub retry_jobs: usize,
    /// Resource rows.
    pub resources: Vec<ControlResourceView>,
    /// Status conditions.
    pub conditions: Vec<ControlCondition>,
}

impl ControlSnapshot {
    /// Returns condition kinds in deterministic order.
    pub fn condition_kinds(&self) -> BTreeSet<String> {
        self.conditions
            .iter()
            .map(|condition| condition.kind.clone())
            .collect()
    }
}

/// Public output frame emitted by the ControlPlane Lite wrapper.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ControlFrame {
    /// Initial baseline frame.
    Baseline(ControlSnapshot),
    /// Incremental delta frame.
    Delta(ControlSnapshot),
    /// Explicit rebaseline frame.
    Rebaseline(ControlSnapshot),
    /// Clear frame emitted when the controller closes.
    Cleared,
}

/// Count of wrapper effects and output frames emitted by an action.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ControlPlaneUpdate {
    /// Number of control-plane lifecycle effects queued.
    pub emitted_effects: usize,
    /// Number of status frames queued.
    pub emitted_frames: usize,
}
