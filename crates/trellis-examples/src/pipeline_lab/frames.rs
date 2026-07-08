use trellis_core::OutputFrameKind;

use super::types::{PipelineFrame, PipelineSnapshot};

pub(super) fn pipeline_frame(kind: &OutputFrameKind) -> PipelineFrame {
    match kind {
        OutputFrameKind::Baseline(value) => {
            PipelineFrame::Baseline(value.get::<PipelineSnapshot>().cloned().unwrap_or_default())
        }
        OutputFrameKind::Delta(value) => {
            PipelineFrame::Delta(value.get::<PipelineSnapshot>().cloned().unwrap_or_default())
        }
        OutputFrameKind::Rebaseline(value, _) => {
            PipelineFrame::Rebaseline(value.get::<PipelineSnapshot>().cloned().unwrap_or_default())
        }
        OutputFrameKind::Clear(_) => PipelineFrame::Cleared,
    }
}
