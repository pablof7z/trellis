use crate::{
    AuditEntry, ClearReason, OutputFrameKind, OutputKey, RebaselineReason, ResourceCommand,
    ResourceKey, ScopeId, TransactionId, TransactionPhase, TransactionResult,
};
use crate::{NodeId, Revision};

/// Deterministic payload-free projection of a committed transaction result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionTrace {
    /// Committed transaction id.
    pub transaction_id: TransactionId,
    /// Graph revision after commit.
    pub revision: Revision,
    /// Input nodes changed by this transaction.
    pub changed_inputs: Vec<NodeId>,
    /// Derived nodes changed by this transaction.
    pub changed_derived_nodes: Vec<NodeId>,
    /// Collection nodes changed by this transaction.
    pub changed_collection_nodes: Vec<NodeId>,
    /// Resource command identity and operation trace.
    pub resource_commands: Vec<ResourceCommandTrace>,
    /// Output frame identity and kind trace.
    pub output_frames: Vec<OutputFrameTrace>,
    /// Audit log emitted by the transaction.
    pub audit_log: Vec<AuditEntry>,
    /// Phase trace emitted by the transaction.
    pub phase_trace: Vec<TransactionPhase>,
}

impl TransactionTrace {
    /// Builds a deterministic trace from a transaction result.
    pub fn from_result<C, O>(result: &TransactionResult<C, O>) -> Self {
        Self {
            transaction_id: result.transaction_id,
            revision: result.revision,
            changed_inputs: result.changed_inputs.clone(),
            changed_derived_nodes: result.changed_derived_nodes.clone(),
            changed_collection_nodes: result.changed_collection_nodes.clone(),
            resource_commands: result
                .resource_plan
                .commands()
                .iter()
                .map(ResourceCommandTrace::from_command)
                .collect(),
            output_frames: result
                .output_frames
                .iter()
                .map(|frame| OutputFrameTrace {
                    output_key: frame.output_key,
                    scope: frame.scope,
                    transaction_id: frame.transaction_id,
                    revision: frame.revision,
                    kind: OutputFrameKindTrace::from_kind(&frame.kind),
                })
                .collect(),
            audit_log: result.audit_log.clone(),
            phase_trace: result.phase_trace.clone(),
        }
    }
}

/// Payload-free resource command trace.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceCommandTrace {
    /// Resource identity.
    pub key: ResourceKey,
    /// Scope associated with the command.
    pub scope: ScopeId,
    /// Command operation.
    pub kind: ResourceCommandKind,
}

impl ResourceCommandTrace {
    fn from_command<C>(command: &ResourceCommand<C>) -> Self {
        Self {
            key: command.key().clone(),
            scope: command.scope(),
            kind: ResourceCommandKind::from_command(command),
        }
    }
}

/// Resource command operation without application payload.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ResourceCommandKind {
    /// Open a resource.
    Open,
    /// Close a resource.
    Close,
    /// Replace a resource.
    Replace,
    /// Refresh a resource.
    Refresh,
}

impl ResourceCommandKind {
    pub(crate) fn from_command<C>(command: &ResourceCommand<C>) -> Self {
        match command {
            ResourceCommand::Open { .. } => Self::Open,
            ResourceCommand::Close { .. } => Self::Close,
            ResourceCommand::Replace { .. } => Self::Replace,
            ResourceCommand::Refresh { .. } => Self::Refresh,
        }
    }
}

/// Payload-free output frame trace.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputFrameTrace {
    /// Output identity.
    pub output_key: OutputKey,
    /// Scope that owns the output.
    pub scope: ScopeId,
    /// Transaction that emitted the frame.
    pub transaction_id: TransactionId,
    /// Revision carried by the frame.
    pub revision: Revision,
    /// Frame kind without materialized payload.
    pub kind: OutputFrameKindTrace,
}

/// Output frame kind without materialized payload.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum OutputFrameKindTrace {
    /// Baseline frame.
    Baseline,
    /// Delta frame.
    Delta,
    /// Clear frame with reason.
    Clear(ClearReason),
    /// Rebaseline frame with reason.
    Rebaseline(RebaselineReason),
}

impl OutputFrameKindTrace {
    pub(crate) fn from_kind<O>(kind: &OutputFrameKind<O>) -> Self {
        match kind {
            OutputFrameKind::Baseline(_) => Self::Baseline,
            OutputFrameKind::Delta(_) => Self::Delta,
            OutputFrameKind::Clear(reason) => Self::Clear(*reason),
            OutputFrameKind::Rebaseline(_, reason) => Self::Rebaseline(*reason),
        }
    }
}

impl<C, O> TransactionResult<C, O> {
    /// Returns a deterministic payload-free projection of this result.
    pub fn trace(&self) -> TransactionTrace {
        TransactionTrace::from_result(self)
    }
}

/// Difference between two replay trace sequences.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceMismatch {
    /// Expected transaction traces.
    pub expected: Vec<TransactionTrace>,
    /// Actual transaction traces.
    pub actual: Vec<TransactionTrace>,
}

/// Compares two deterministic transaction trace sequences.
pub fn assert_transaction_traces_match(
    expected: &[TransactionTrace],
    actual: &[TransactionTrace],
) -> Result<(), TraceMismatch> {
    if expected == actual {
        Ok(())
    } else {
        Err(TraceMismatch {
            expected: expected.to_vec(),
            actual: actual.to_vec(),
        })
    }
}
