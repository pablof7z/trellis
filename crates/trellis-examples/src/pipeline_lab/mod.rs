//! PipelineLab visual data-pipeline previewer secondary showcase.

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
pub use engine::PipelineLabApp;
pub use sample::{ids, opening_pipeline, sample_credentials, sample_pipeline};
pub use scripts::pipeline_lifecycle_showcase_trace;
pub use types::{
    CredentialStore, PipelineEffect, PipelineFrame, PipelineGraphNodeView, PipelineGraphSpec,
    PipelineHandle, PipelineJobStatus, PipelineLabEvent, PipelineLabUpdate, PipelineNode,
    PipelineNodeKind, PipelineResource, PipelineSession, PipelineSnapshot, PreviewPanel,
    PreviewRow,
};
