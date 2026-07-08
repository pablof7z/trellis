use trellis_core::OutputFrameKind;

use super::types::{ControlFrame, ControlSnapshot};

pub(super) fn control_frame(kind: &OutputFrameKind) -> ControlFrame {
    match kind {
        OutputFrameKind::Baseline(value) => {
            ControlFrame::Baseline(value.get::<ControlSnapshot>().cloned().unwrap_or_default())
        }
        OutputFrameKind::Delta(value) => {
            ControlFrame::Delta(value.get::<ControlSnapshot>().cloned().unwrap_or_default())
        }
        OutputFrameKind::Rebaseline(value, _) => {
            ControlFrame::Rebaseline(value.get::<ControlSnapshot>().cloned().unwrap_or_default())
        }
        OutputFrameKind::Clear(_) => ControlFrame::Cleared,
    }
}
