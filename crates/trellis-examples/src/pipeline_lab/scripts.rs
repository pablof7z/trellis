use crate::showcase_trace::{
    ShowcaseHostStatus, ShowcaseStep, ShowcaseTrace, build_showcase_trace,
};

use super::PipelineLabApp;
use super::sample::{opening_pipeline, sample_credentials, sample_pipeline};
use super::types::{PipelineJobStatus, PipelineLabEvent};

/// Runs the headless `pipeline-lifecycle` showcase script.
pub fn pipeline_lifecycle_showcase_trace() -> ShowcaseTrace {
    build_showcase_trace(
        "pipeline-lab",
        "pipeline-lifecycle",
        &[
            "cargo",
            "run",
            "-p",
            "trellis-examples",
            "--example",
            "pipeline_lab",
            "--",
            "--script",
            "pipeline-lifecycle",
        ],
        || {
            let mut app = PipelineLabApp::new(sample_pipeline(), sample_credentials());
            let pipeline = app.open_pipeline(opening_pipeline());
            app.drain_effects();
            app.drain_output(pipeline);
            app.drain_diagnostic_traces();

            app.apply_event(
                pipeline,
                PipelineLabEvent::EditTransform {
                    node_id: "clean_orders".to_owned(),
                    expression: "filter status in paid,shipped".to_owned(),
                },
            );
            let transform_edit = pop_trace(&mut app, "transform-edit", Vec::new());

            app.apply_event(
                pipeline,
                PipelineLabEvent::ApplyJobStatus {
                    node_id: "daily_revenue".to_owned(),
                    status: PipelineJobStatus::Failed("worker timeout".to_owned()),
                },
            );
            let job_failure = pop_trace(
                &mut app,
                "job-failure",
                vec![ShowcaseHostStatus {
                    target: "daily_revenue".to_owned(),
                    status: "failed: worker timeout".to_owned(),
                    command_revision: None,
                }],
            );

            app.apply_event(
                pipeline,
                PipelineLabEvent::HideNode("daily_revenue".to_owned()),
            );
            let hide_panel = pop_trace(&mut app, "hide-panel", Vec::new());

            app.apply_event(
                pipeline,
                PipelineLabEvent::RevokeSourceCredential {
                    source_id: "warehouse".to_owned(),
                },
            );
            let revoke_credential = pop_trace(&mut app, "revoke-credential", Vec::new());

            app.close(pipeline);
            let close_pipeline = pop_trace(&mut app, "close-pipeline", Vec::new());

            vec![
                transform_edit,
                job_failure,
                hide_panel,
                revoke_credential,
                close_pipeline,
            ]
        },
    )
}

fn pop_trace(
    app: &mut PipelineLabApp,
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
