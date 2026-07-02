use trellis_core::{
    Graph, NodeId, OutputFrameKind, OutputFrameKindTrace, OutputKey, ResourceCommandKind,
    ResourceKey, Revision, ScopeId, TransactionId, TransactionResult,
};

/// Structural context for an audited resource command assertion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceAuditContext {
    /// Resource key for the audited command.
    pub key: ResourceKey,
    /// Scope associated with the command.
    pub scope: ScopeId,
    /// Transaction that emitted the command.
    pub transaction_id: TransactionId,
    /// Revision that emitted the command.
    pub revision: Revision,
    /// Command operation without application payload.
    pub kind: ResourceCommandKind,
}

/// Structural context for an audited output frame assertion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputAuditContext {
    /// Output key for the audited frame.
    pub key: OutputKey,
    /// Scope associated with the frame.
    pub scope: ScopeId,
    /// Transaction that emitted the frame.
    pub transaction_id: TransactionId,
    /// Revision carried by the frame.
    pub revision: Revision,
    /// Frame kind without materialized payload.
    pub kind: OutputFrameKindTrace,
}

/// Failure from an explainability assertion over transaction audit data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuditAssertionError {
    /// A resource command had no graph-visible explanation.
    MissingResourceCommand {
        /// Command context.
        context: ResourceAuditContext,
    },
    /// A resource command explanation did not match the emitted command.
    ResourceCommandMismatch {
        /// Command context.
        context: ResourceAuditContext,
        /// Missing or mismatched field name.
        field: &'static str,
    },
    /// An output frame had no graph-visible explanation.
    MissingOutputFrame {
        /// Frame context.
        context: OutputAuditContext,
    },
    /// An output frame explanation did not match the emitted frame.
    OutputFrameMismatch {
        /// Frame context.
        context: OutputAuditContext,
        /// Missing or mismatched field name.
        field: &'static str,
    },
    /// A requested dependency path was missing.
    MissingDependencyPath {
        /// Upstream node.
        from: NodeId,
        /// Downstream node.
        to: NodeId,
    },
}

/// Asserts every resource command in a result has matching audit explanation.
pub fn assert_no_unexplained_plan<C, O>(
    graph: &Graph<C, O>,
    result: &TransactionResult<C, O>,
) -> Result<(), AuditAssertionError> {
    for command in result.resource_plan.commands() {
        let context = resource_audit_context(command, result);
        let key = command.key();
        let explanation = graph.why_resource_command(key).ok_or_else(|| {
            AuditAssertionError::MissingResourceCommand {
                context: context.clone(),
            }
        })?;
        if explanation.scope != command.scope() {
            return Err(AuditAssertionError::ResourceCommandMismatch {
                context,
                field: "scope",
            });
        }
        if explanation.transaction_id != result.transaction_id {
            return Err(AuditAssertionError::ResourceCommandMismatch {
                context,
                field: "transaction_id",
            });
        }
        if explanation.revision != result.revision {
            return Err(AuditAssertionError::ResourceCommandMismatch {
                context,
                field: "revision",
            });
        }
        if explanation.kind != resource_command_kind(command) {
            return Err(AuditAssertionError::ResourceCommandMismatch {
                context,
                field: "kind",
            });
        }
        if !resource_cause_is_explainable(explanation, command, result) {
            return Err(AuditAssertionError::ResourceCommandMismatch {
                context,
                field: "cause",
            });
        }
    }
    Ok(())
}

/// Asserts every output frame in a result has matching audit explanation.
pub fn assert_no_unexplained_output_frame<C, O>(
    graph: &Graph<C, O>,
    result: &TransactionResult<C, O>,
) -> Result<(), AuditAssertionError> {
    for frame in &result.output_frames {
        let context = output_audit_context(frame);
        let explanation = graph.why_output_frame(frame.output_key).ok_or(
            AuditAssertionError::MissingOutputFrame {
                context: context.clone(),
            },
        )?;
        if explanation.scope != frame.scope {
            return Err(AuditAssertionError::OutputFrameMismatch {
                context,
                field: "scope",
            });
        }
        if explanation.transaction_id != frame.transaction_id {
            return Err(AuditAssertionError::OutputFrameMismatch {
                context,
                field: "transaction_id",
            });
        }
        if explanation.revision != frame.revision {
            return Err(AuditAssertionError::OutputFrameMismatch {
                context,
                field: "revision",
            });
        }
        if explanation.kind != output_frame_kind(&frame.kind) {
            return Err(AuditAssertionError::OutputFrameMismatch {
                context,
                field: "kind",
            });
        }
        if !output_frame_is_explainable(explanation, result) {
            return Err(AuditAssertionError::OutputFrameMismatch {
                context,
                field: "input_causes",
            });
        }
    }
    Ok(())
}

/// Asserts that a deterministic dependency path exists in the graph.
pub fn assert_dependency_path_exists<C, O>(
    graph: &Graph<C, O>,
    from: NodeId,
    to: NodeId,
) -> Result<(), AuditAssertionError> {
    graph
        .dependency_path(from, to)
        .map(|_| ())
        .ok_or(AuditAssertionError::MissingDependencyPath { from, to })
}

fn resource_command_kind<C>(command: &trellis_core::ResourceCommand<C>) -> ResourceCommandKind {
    match command {
        trellis_core::ResourceCommand::Open { .. } => ResourceCommandKind::Open,
        trellis_core::ResourceCommand::Close { .. } => ResourceCommandKind::Close,
        trellis_core::ResourceCommand::Replace { .. } => ResourceCommandKind::Replace,
        trellis_core::ResourceCommand::Refresh { .. } => ResourceCommandKind::Refresh,
    }
}

fn output_frame_kind<O>(kind: &OutputFrameKind<O>) -> OutputFrameKindTrace {
    match kind {
        OutputFrameKind::Baseline(_) => OutputFrameKindTrace::Baseline,
        OutputFrameKind::Delta(_) => OutputFrameKindTrace::Delta,
        OutputFrameKind::Clear(reason) => OutputFrameKindTrace::Clear(*reason),
        OutputFrameKind::Rebaseline(_, reason) => OutputFrameKindTrace::Rebaseline(*reason),
    }
}

fn resource_audit_context<C, O>(
    command: &trellis_core::ResourceCommand<C>,
    result: &TransactionResult<C, O>,
) -> ResourceAuditContext {
    ResourceAuditContext {
        key: command.key().clone(),
        scope: command.scope(),
        transaction_id: result.transaction_id,
        revision: result.revision,
        kind: resource_command_kind(command),
    }
}

fn output_audit_context<O>(frame: &trellis_core::OutputFrame<O>) -> OutputAuditContext {
    OutputAuditContext {
        key: frame.output_key,
        scope: frame.scope,
        transaction_id: frame.transaction_id,
        revision: frame.revision,
        kind: output_frame_kind(&frame.kind),
    }
}

fn resource_cause_is_explainable<C, O>(
    explanation: &trellis_core::ResourceCommandExplanation,
    command: &trellis_core::ResourceCommand<C>,
    result: &TransactionResult<C, O>,
) -> bool {
    match explanation.cause {
        trellis_core::ResourceCommandCause::Planner { collection } => {
            explanation.collection_diffs.contains(&collection)
                && (result.changed_inputs.is_empty()
                    || (!explanation.input_causes.is_empty()
                        && !explanation.dependency_paths.is_empty()))
        }
        trellis_core::ResourceCommandCause::ScopeClosed { scope } => scope == command.scope(),
    }
}

fn output_frame_is_explainable<C, O>(
    explanation: &trellis_core::OutputFrameExplanation,
    result: &TransactionResult<C, O>,
) -> bool {
    if result.changed_inputs.is_empty() {
        return true;
    }
    if matches!(
        explanation.kind,
        OutputFrameKindTrace::Baseline
            | OutputFrameKindTrace::Clear(_)
            | OutputFrameKindTrace::Rebaseline(_)
    ) && explanation.changed_dependencies.is_empty()
    {
        return true;
    }
    !explanation.changed_dependencies.is_empty()
        && !explanation.input_causes.is_empty()
        && !explanation.dependency_paths.is_empty()
}
