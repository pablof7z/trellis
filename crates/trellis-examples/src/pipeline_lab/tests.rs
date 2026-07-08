use trellis_core::{OutputFrameKindTrace, ScopeLifecycleKind};

use super::*;

fn open_pipeline() -> (PipelineLabApp, PipelineHandle) {
    let mut app = PipelineLabApp::new(sample_pipeline(), sample_credentials());
    let handle = app.open_pipeline(opening_pipeline());
    app.drain_effects();
    app.drain_output(handle);
    app.drain_diagnostic_traces();
    (app, handle)
}

#[test]
fn transform_edit_restarts_downstream_jobs_and_rebaselines() {
    let (mut app, handle) = open_pipeline();

    app.apply_event(
        handle,
        PipelineLabEvent::EditTransform {
            node_id: "clean_orders".to_owned(),
            expression: "filter status in paid,shipped".to_owned(),
        },
    );
    let effects = app.drain_effects();
    assert!(effects.iter().any(|effect| matches!(
        effect,
        PipelineEffect::Close(PipelineResource::ComputeJob { node_id, .. })
            if node_id == "clean_orders"
    )));
    assert!(effects.iter().any(|effect| matches!(
        effect,
        PipelineEffect::Open(PipelineResource::ComputeJob { node_id, .. })
            if node_id == "daily_revenue"
    )));
    assert!(effects.iter().any(|effect| matches!(
        effect,
        PipelineEffect::Close(PipelineResource::PreviewQuery { node_id, .. })
            if node_id == "daily_revenue"
    )));
    assert!(matches!(
        app.drain_output(handle).first(),
        Some(PipelineFrame::Rebaseline(snapshot))
            if snapshot.panel_node_ids().contains("clean_orders")
                && snapshot.panel_node_ids().contains("daily_revenue")
    ));
}

#[test]
fn credential_revoke_closes_connection_jobs_and_clears_previews() {
    let (mut app, handle) = open_pipeline();

    app.apply_event(
        handle,
        PipelineLabEvent::RevokeSourceCredential {
            source_id: "warehouse".to_owned(),
        },
    );
    let effects = app.drain_effects();
    assert!(
        effects.contains(&PipelineEffect::Close(PipelineResource::SourceConnection {
            source_id: "warehouse".to_owned(),
        }))
    );
    assert!(effects.iter().any(|effect| matches!(
        effect,
        PipelineEffect::Close(PipelineResource::ComputeJob { node_id, .. })
            if node_id == "daily_revenue"
    )));
    assert!(effects.iter().any(|effect| matches!(
        effect,
        PipelineEffect::Close(PipelineResource::PreviewQuery { node_id, .. })
            if node_id == "clean_orders"
    )));
    assert!(matches!(
        app.drain_output(handle).first(),
        Some(PipelineFrame::Delta(snapshot)) if snapshot.panels.is_empty()
    ));
}

#[test]
fn hidden_node_closes_only_panel_preview_resources() {
    let (mut app, handle) = open_pipeline();

    app.apply_event(
        handle,
        PipelineLabEvent::HideNode("daily_revenue".to_owned()),
    );
    let effects = app.drain_effects();
    assert!(effects.iter().any(|effect| matches!(
        effect,
        PipelineEffect::Close(PipelineResource::PreviewQuery { node_id, .. })
            if node_id == "daily_revenue"
    )));
    assert!(!effects.iter().any(|effect| matches!(
        effect,
        PipelineEffect::Close(PipelineResource::ComputeJob { node_id, .. })
            if node_id == "daily_revenue"
    )));
    assert!(!effects.iter().any(|effect| matches!(
        effect,
        PipelineEffect::Close(PipelineResource::SourceConnection { .. })
    )));
    assert!(matches!(
        app.drain_output(handle).first(),
        Some(PipelineFrame::Rebaseline(snapshot))
            if snapshot.panel_node_ids().contains("clean_orders")
                && !snapshot.panel_node_ids().contains("daily_revenue")
    ));
}

#[test]
fn job_failure_status_is_input_without_resource_churn() {
    let (mut app, handle) = open_pipeline();

    app.apply_event(
        handle,
        PipelineLabEvent::ApplyJobStatus {
            node_id: "daily_revenue".to_owned(),
            status: PipelineJobStatus::Failed("worker timeout".to_owned()),
        },
    );
    assert!(app.drain_effects().is_empty());
    assert!(matches!(
        app.drain_output(handle).first(),
        Some(PipelineFrame::Delta(snapshot))
            if snapshot
                .panels
                .iter()
                .any(|panel| panel.node_id == "daily_revenue"
                    && panel.status == PipelineJobStatus::Failed("worker timeout".to_owned())
                    && panel.rows.is_empty())
    ));
}

#[test]
fn close_pipeline_closes_resources_and_clears_output() {
    let (mut app, handle) = open_pipeline();

    app.close(handle);
    let effects = app.drain_effects();
    assert!(
        effects.contains(&PipelineEffect::Close(PipelineResource::SourceConnection {
            source_id: "warehouse".to_owned(),
        }))
    );
    assert!(effects.iter().any(|effect| matches!(
        effect,
        PipelineEffect::Close(PipelineResource::ComputeJob { node_id, .. })
            if node_id == "daily_revenue"
    )));
    assert!(app.drain_output(handle).contains(&PipelineFrame::Cleared));
}

#[test]
fn pipeline_lifecycle_trace_uses_showcase_contract() {
    let trace = pipeline_lifecycle_showcase_trace();

    assert_eq!(trace.showcase, "pipeline-lab");
    assert_eq!(trace.script, "pipeline-lifecycle");
    assert_eq!(trace.replay.status, "passed");
    assert_eq!(
        trace
            .steps
            .iter()
            .map(|step| step.name.as_str())
            .collect::<Vec<_>>(),
        vec![
            "transform-edit",
            "job-failure",
            "hide-panel",
            "revoke-credential",
            "close-pipeline",
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
fn seeded_bug_capsule_detects_stale_revoked_pipeline_work() {
    let report = run_bug_capsule("pipeline-credential-revoke-clears-previews").unwrap();

    assert_eq!(report.status, "pass");
    assert!(report.expected_failures_detected);
    assert_eq!(available_bug_capsules().len(), 1);
}
