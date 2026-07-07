//! FleetPulse telemetry dashboard flagship showcase.

mod bug_capsule_paths;
mod bug_capsules;
mod engine;
mod graph;
mod result;
mod sample;
mod scripts;
mod selectors;
mod status;
mod status_runtime;
mod types;

#[cfg(test)]
mod tests;

pub use bug_capsules::{available_bug_capsules, run_all_bug_capsules, run_bug_capsule};
pub use engine::FleetPulseApp;
pub use scripts::revoke_permission_showcase_trace;
pub use status::{FleetHostOutcome, FleetHostStatus, FleetStatusClass, FleetStatusFrame};
pub use types::{
    FleetAlert, FleetAlertRule, FleetCard, FleetDashboardHandle, FleetDashboardParams,
    FleetDataset, FleetDevice, FleetEffect, FleetFilterChange, FleetFrame, FleetMetric, FleetPanel,
    FleetPermissionChange, FleetPermissions, FleetSnapshot, FleetTarget, FleetUpdate,
    TelemetryTopic,
};
