use trellis_core::{OutputFrameKindTrace, ScopeLifecycleKind};

use super::*;

fn open_control() -> (ControlPlaneLiteApp, ControlPlaneHandle) {
    let mut app = ControlPlaneLiteApp::new();
    let handle = app.open_controller(initial_config());
    app.drain_effects();
    app.drain_output(handle);
    app.drain_diagnostic_traces();
    (app, handle)
}

#[test]
fn config_change_replaces_workers_and_port() {
    let (mut app, handle) = open_control();

    app.apply_event(handle, ControlPlaneEvent::ReplaceConfig(updated_config()));
    let effects = app.drain_effects();
    assert!(effects.contains(&ControlEffect::Close(worker_resource("v1", 0))));
    assert!(effects.contains(&ControlEffect::Close(worker_resource("v1", 1))));
    assert!(effects.contains(&ControlEffect::Close(initial_port_resource())));
    assert!(effects.contains(&ControlEffect::Open(worker_resource("v2", 2))));
    assert!(effects.contains(&ControlEffect::Open(updated_port_resource())));
    assert!(matches!(
        app.drain_output(handle).first(),
        Some(ControlFrame::Rebaseline(snapshot))
            if snapshot.app_id.as_deref() == Some("checkout")
                && snapshot.desired_resources == 6
    ));
}

#[test]
fn failed_resource_opens_retry_and_degraded_status() {
    let (mut app, handle) = open_control();

    app.apply_event(
        handle,
        ControlPlaneEvent::ApplyResourceStatus {
            resource: worker_resource("v1", 1),
            status: ControlResourceStatus::Failed("crash loop".to_owned()),
        },
    );
    let effects = app.drain_effects();
    assert!(effects.iter().any(|effect| matches!(
        effect,
        ControlEffect::Open(ControlResource::RetryJob { target, .. })
            if target.contains("worker/1")
    )));
    assert!(matches!(
        app.drain_output(handle).first(),
        Some(ControlFrame::Delta(snapshot))
            if snapshot.retry_jobs == 1
                && snapshot.condition_kinds().contains("Degraded")
    ));
}

#[test]
fn recovered_resource_closes_retry_and_restores_available() {
    let (mut app, handle) = open_control();
    app.apply_event(
        handle,
        ControlPlaneEvent::ApplyResourceStatus {
            resource: worker_resource("v1", 1),
            status: ControlResourceStatus::Failed("crash loop".to_owned()),
        },
    );
    app.drain_effects();
    app.drain_output(handle);
    app.drain_diagnostic_traces();

    app.apply_event(
        handle,
        ControlPlaneEvent::ApplyResourceStatus {
            resource: worker_resource("v1", 1),
            status: ControlResourceStatus::Ready,
        },
    );
    let effects = app.drain_effects();
    assert!(effects.iter().any(|effect| matches!(
        effect,
        ControlEffect::Close(ControlResource::RetryJob { target, .. })
            if target.contains("worker/1")
    )));
    assert!(matches!(
        app.drain_output(handle).first(),
        Some(ControlFrame::Delta(snapshot))
            if snapshot.retry_jobs == 0
                && !snapshot.condition_kinds().contains("Degraded")
    ));
}

#[test]
fn scope_close_deletes_owned_resources_and_clears_output() {
    let (mut app, handle) = open_control();

    app.close(handle);
    let effects = app.drain_effects();
    assert!(effects.contains(&ControlEffect::Close(worker_resource("v1", 0))));
    assert!(effects.contains(&ControlEffect::Close(worker_resource("v1", 1))));
    assert!(effects.contains(&ControlEffect::Close(initial_port_resource())));
    assert!(app.drain_output(handle).contains(&ControlFrame::Cleared));
}

#[test]
fn control_plane_lifecycle_trace_uses_showcase_contract() {
    let trace = control_plane_lifecycle_showcase_trace();

    assert_eq!(trace.showcase, "control-plane-lite");
    assert_eq!(trace.script, "control-plane-lifecycle");
    assert_eq!(trace.replay.status, "passed");
    assert_eq!(
        trace
            .steps
            .iter()
            .map(|step| step.name.as_str())
            .collect::<Vec<_>>(),
        vec![
            "config-change",
            "resource-failed",
            "resource-recovered",
            "close-controller",
        ]
    );
    assert!(trace.steps.iter().all(|step| {
        step.trace
            .invariant_results
            .iter()
            .any(|result| result.name == "incremental_equals_full_recompute" && result.passed)
    }));
    assert!(trace.steps.iter().any(|step| {
        step.trace.output_frames.iter().any(|frame| {
            matches!(
                frame.kind,
                OutputFrameKindTrace::Delta | OutputFrameKindTrace::Rebaseline(_)
            )
        })
    }));
    assert!(
        trace
            .steps
            .iter()
            .any(|step| !step.host_statuses.is_empty())
    );
    assert!(trace.steps.iter().any(|step| {
        step.trace
            .scope_events
            .iter()
            .any(|event| event.kind == ScopeLifecycleKind::Closed)
    }));
}

#[test]
fn seeded_bug_capsule_detects_missing_retry() {
    let report = run_bug_capsule("control-resource-failure-opens-retry").unwrap();

    assert_eq!(report.status, "pass");
    assert!(report.expected_failures_detected);
    assert_eq!(available_bug_capsules().len(), 1);
}
