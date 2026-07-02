use crate::output_payload::{OutputPayload, StoredOutput};
use crate::{MaterializedOutput, OutputKey, Revision, ScopeId, TransactionId};

/// Reason a materialized output was cleared.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ClearReason {
    /// The owning scope was closed.
    ScopeClosed,
}

/// Reason a materialized output was rebaselined.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RebaselineReason {
    /// The host explicitly requested a rebaseline.
    Requested,
}

/// Data-only output frame kind.
#[derive(Clone, Debug, PartialEq)]
pub enum OutputFrameKind {
    /// Complete current state for a newly attached output.
    Baseline(OutputPayload),
    /// State-replacement delta for an existing output.
    Delta(OutputPayload),
    /// Clear the consumer state for this output.
    Clear(ClearReason),
    /// Complete current state after an explicit discontinuity.
    Rebaseline(OutputPayload, RebaselineReason),
}

impl OutputFrameKind {
    /// Builds a baseline frame kind with a typed payload.
    pub fn baseline<T>(value: T) -> Self
    where
        T: Clone + PartialEq + Send + Sync + 'static,
    {
        Self::Baseline(OutputPayload::new(value))
    }

    /// Builds a delta frame kind with a typed payload.
    pub fn delta<T>(value: T) -> Self
    where
        T: Clone + PartialEq + Send + Sync + 'static,
    {
        Self::Delta(OutputPayload::new(value))
    }

    /// Builds a rebaseline frame kind with a typed payload.
    pub fn rebaseline<T>(value: T, reason: RebaselineReason) -> Self
    where
        T: Clone + PartialEq + Send + Sync + 'static,
    {
        Self::Rebaseline(OutputPayload::new(value), reason)
    }

    pub(crate) fn baseline_from_stored(value: Box<dyn StoredOutput>) -> Self {
        Self::Baseline(OutputPayload::from_stored(value))
    }

    pub(crate) fn delta_from_stored(value: Box<dyn StoredOutput>) -> Self {
        Self::Delta(OutputPayload::from_stored(value))
    }

    pub(crate) fn rebaseline_from_stored(
        value: Box<dyn StoredOutput>,
        reason: RebaselineReason,
    ) -> Self {
        Self::Rebaseline(OutputPayload::from_stored(value), reason)
    }

    /// Returns this frame payload as the requested type, if this is a payload frame.
    pub fn payload<T>(&self) -> Option<&T>
    where
        T: Clone + PartialEq + Send + Sync + 'static,
    {
        match self {
            Self::Baseline(payload) | Self::Delta(payload) | Self::Rebaseline(payload, _) => {
                payload.get::<T>()
            }
            Self::Clear(_) => None,
        }
    }

    /// Returns the clear reason, if this is a clear frame.
    pub fn clear_reason(&self) -> Option<ClearReason> {
        match self {
            Self::Clear(reason) => Some(*reason),
            _ => None,
        }
    }

    /// Returns the rebaseline reason, if this is a rebaseline frame.
    pub fn rebaseline_reason(&self) -> Option<RebaselineReason> {
        match self {
            Self::Rebaseline(_, reason) => Some(*reason),
            _ => None,
        }
    }
}

/// Data-only materialized output frame returned from a transaction.
#[derive(Clone, Debug, PartialEq)]
pub struct OutputFrame {
    /// Output key this frame targets.
    pub output_key: OutputKey,
    /// Scope that owns this output.
    pub scope: ScopeId,
    /// Transaction that emitted this frame.
    pub transaction_id: TransactionId,
    /// Graph revision this frame belongs to.
    pub revision: Revision,
    /// Frame payload.
    pub kind: OutputFrameKind,
}

impl OutputFrame {
    /// Returns this frame payload as the requested type when both key and type match.
    pub fn payload_for<T>(&self, output: &MaterializedOutput<T>) -> Option<&T>
    where
        T: Clone + PartialEq + Send + Sync + 'static,
    {
        (self.output_key == output.key())
            .then(|| self.kind.payload::<T>())
            .flatten()
    }
}
