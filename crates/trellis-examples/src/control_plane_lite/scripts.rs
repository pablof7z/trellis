use crate::showcase_trace::{
    ShowcaseHostStatus, ShowcaseStep, ShowcaseTrace, build_showcase_trace,
};

use super::ControlPlaneLiteApp;
use super::sample::{initial_config, updated_config, worker_resource};
use super::types::{ControlPlaneEvent, ControlResourceStatus};

/// Runs the headless `control-plane-lifecycle` showcase script.
pub fn control_plane_lifecycle_showcase_trace() -> ShowcaseTrace {
    build_showcase_trace(
        "control-plane-lite",
        "control-plane-lifecycle",
        &[
            "cargo",
            "run",
            "-p",
            "trellis-examples",
            "--example",
            "control_plane_lite",
            "--",
            "--script",
            "control-plane-lifecycle",
        ],
        || {
            let mut app = ControlPlaneLiteApp::new();
            let controller = app.open_controller(initial_config());
            app.drain_effects();
            app.drain_output(controller);
            app.drain_diagnostic_traces();

            app.apply_event(
                controller,
                ControlPlaneEvent::ReplaceConfig(updated_config()),
            );
            let config_change = pop_trace(&mut app, "config-change", Vec::new());

            let failed_worker = worker_resource("v2", 1);
            app.apply_event(
                controller,
                ControlPlaneEvent::ApplyResourceStatus {
                    resource: failed_worker.clone(),
                    status: ControlResourceStatus::Failed("crash loop".to_owned()),
                },
            );
            let resource_failed = pop_trace(
                &mut app,
                "resource-failed",
                vec![ShowcaseHostStatus {
                    target: "checkout/worker/1".to_owned(),
                    status: "failed: crash loop".to_owned(),
                    command_revision: None,
                }],
            );

            app.apply_event(
                controller,
                ControlPlaneEvent::ApplyResourceStatus {
                    resource: failed_worker,
                    status: ControlResourceStatus::Ready,
                },
            );
            let resource_recovered = pop_trace(
                &mut app,
                "resource-recovered",
                vec![ShowcaseHostStatus {
                    target: "checkout/worker/1".to_owned(),
                    status: "ready".to_owned(),
                    command_revision: None,
                }],
            );

            app.close(controller);
            let close_controller = pop_trace(&mut app, "close-controller", Vec::new());

            vec![
                config_change,
                resource_failed,
                resource_recovered,
                close_controller,
            ]
        },
    )
}

fn pop_trace(
    app: &mut ControlPlaneLiteApp,
    name: &str,
    host_statuses: Vec<ShowcaseHostStatus>,
) -> ShowcaseStep {
    let trace = app
        .drain_diagnostic_traces()
        .pop()
        .expect("script step emits one trace");
    ShowcaseStep {
        name: name.to_owned(),
        host_statuses,
        trace,
    }
}
