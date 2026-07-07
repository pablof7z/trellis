//! FleetPulse telemetry dashboard flagship showcase.

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

pub use engine::FleetPulseApp;
pub use scripts::revoke_permission_showcase_trace;
pub use status::{FleetHostOutcome, FleetHostStatus, FleetStatusClass, FleetStatusFrame};
pub use types::{
    FleetAlert, FleetAlertRule, FleetCard, FleetDashboardHandle, FleetDashboardParams,
    FleetDataset, FleetDevice, FleetEffect, FleetFilterChange, FleetFrame, FleetMetric, FleetPanel,
    FleetPermissionChange, FleetPermissions, FleetSnapshot, FleetTarget, FleetUpdate,
    TelemetryTopic,
};
