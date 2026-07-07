use crate::showcase_trace::{ShowcaseStep, ShowcaseTrace, build_showcase_trace};

use super::engine::default_params;
use super::sample::topic;
use super::status::FleetHostOutcome;
use super::status_runtime::host_status_for;
use super::types::{FleetMetric, FleetPanel, FleetPermissionChange, FleetTarget};
use super::{FleetDataset, FleetPulseApp};

/// Runs the headless `revoke-permission` showcase script.
pub fn revoke_permission_showcase_trace() -> ShowcaseTrace {
    build_showcase_trace(
        "fleetpulse",
        "revoke-permission",
        &[
            "cargo",
            "run",
            "-p",
            "trellis-examples",
            "--example",
            "fleetpulse",
            "--",
            "--script",
            "revoke-permission",
        ],
        || {
            let mut app = FleetPulseApp::new(FleetDataset::sample());
            let handle = app.open_fleet_dashboard(default_params());
            let revoked_topic =
                FleetTarget::Topic(topic("plant-7", "pump-2", FleetMetric::Temperature));
            let open_revision = app
                .command_revision_for(&revoked_topic)
                .expect("revoked topic opens during setup");
            app.drain_effects();
            app.drain_output(handle);
            app.drain_diagnostic_traces();

            app.apply_permission_change(
                handle,
                FleetPermissionChange::RevokeDevice {
                    device_id: "pump-2".to_owned(),
                },
            );
            let revoke_trace = app
                .drain_diagnostic_traces()
                .pop()
                .expect("permission revoke emits one trace");

            app.apply_host_status(
                handle,
                host_status_for(
                    revoked_topic,
                    FleetPanel::Overview,
                    open_revision,
                    100,
                    FleetHostOutcome::Open,
                ),
            );
            let late_statuses = app.drain_showcase_host_statuses();
            let status_trace = app
                .drain_diagnostic_traces()
                .pop()
                .expect("late status emits one trace");

            app.close(handle);
            let close_trace = app
                .drain_diagnostic_traces()
                .pop()
                .expect("close emits one trace");

            vec![
                ShowcaseStep {
                    name: "revoke-permission".to_owned(),
                    host_statuses: Vec::new(),
                    trace: revoke_trace,
                },
                ShowcaseStep {
                    name: "late-closed-topic-status".to_owned(),
                    host_statuses: late_statuses,
                    trace: status_trace,
                },
                ShowcaseStep {
                    name: "close-dashboard".to_owned(),
                    host_statuses: Vec::new(),
                    trace: close_trace,
                },
            ]
        },
    )
}
