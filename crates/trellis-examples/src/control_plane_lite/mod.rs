//! ControlPlane Lite desired-state reconciler secondary showcase.

mod bug_capsules;
mod engine;
mod frames;
mod graph;
mod sample;
mod scripts;
mod selectors;
mod types;

#[cfg(test)]
mod tests;

pub use bug_capsules::{available_bug_capsules, run_all_bug_capsules, run_bug_capsule};
pub use engine::ControlPlaneLiteApp;
pub use sample::{
    ids, initial_config, initial_port_resource, updated_config, updated_port_resource,
    worker_resource,
};
pub use scripts::control_plane_lifecycle_showcase_trace;
pub use types::{
    ControlCondition, ControlEffect, ControlFrame, ControlPlaneEvent, ControlPlaneHandle,
    ControlPlaneUpdate, ControlResource, ControlResourceStatus, ControlResourceView,
    ControlSnapshot, DesiredAppConfig,
};
