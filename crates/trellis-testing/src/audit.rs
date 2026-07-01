use trellis_core::{
    Graph, NodeId, OutputFrameKind, OutputFrameKindTrace, OutputKey, ResourceCommandKind,
    ResourceKey, TransactionResult,
};

/// Failure from an explainability assertion over transaction audit data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuditAssertionError {
    /// A resource command had no graph-visible explanation.
    MissingResourceCommand {
        /// Resource key for the unexplained command.
        key: ResourceKey,
    },
    /// A resource command explanation did not match the emitted command.
    ResourceCommandMismatch {
        /// Resource key for the mismatched command.
        key: ResourceKey,
        /// Missing or mismatched field name.
        field: &'static str,
    },
    /// An output frame had no graph-visible explanation.
    MissingOutputFrame {
        /// Output key for the unexplained frame.
        key: OutputKey,
    },
    /// An output frame explanation did not match the emitted frame.
    OutputFrameMismatch {
        /// Output key for the mismatched frame.
        key: OutputKey,
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
        let key = command.key();
        let explanation = graph
            .why_resource_command(key)
            .ok_or_else(|| AuditAssertionError::MissingResourceCommand { key: key.clone() })?;
        if explanation.scope != command.scope() {
            return Err(AuditAssertionError::ResourceCommandMismatch {
                key: key.clone(),
                field: "scope",
            });
        }
        if explanation.transaction_id != result.transaction_id {
            return Err(AuditAssertionError::ResourceCommandMismatch {
                key: key.clone(),
                field: "transaction_id",
            });
        }
        if explanation.revision != result.revision {
            return Err(AuditAssertionError::ResourceCommandMismatch {
                key: key.clone(),
                field: "revision",
            });
        }
        if explanation.kind != resource_command_kind(command) {
            return Err(AuditAssertionError::ResourceCommandMismatch {
                key: key.clone(),
                field: "kind",
            });
        }
    }
    Ok(())
}

/// Asserts every resource command has a graph-visible cause.
pub fn assert_every_resource_command_has_cause<C, O>(
    graph: &Graph<C, O>,
    result: &TransactionResult<C, O>,
) -> Result<(), AuditAssertionError> {
    assert_no_unexplained_plan(graph, result)
}

/// Asserts every output frame in a result has matching audit explanation.
pub fn assert_no_unexplained_output_frame<C, O>(
    graph: &Graph<C, O>,
    result: &TransactionResult<C, O>,
) -> Result<(), AuditAssertionError> {
    for frame in &result.output_frames {
        let explanation = graph.why_output_frame(frame.output_key).ok_or(
            AuditAssertionError::MissingOutputFrame {
                key: frame.output_key,
            },
        )?;
        if explanation.scope != frame.scope {
            return Err(AuditAssertionError::OutputFrameMismatch {
                key: frame.output_key,
                field: "scope",
            });
        }
        if explanation.transaction_id != frame.transaction_id {
            return Err(AuditAssertionError::OutputFrameMismatch {
                key: frame.output_key,
                field: "transaction_id",
            });
        }
        if explanation.revision != frame.revision {
            return Err(AuditAssertionError::OutputFrameMismatch {
                key: frame.output_key,
                field: "revision",
            });
        }
        if explanation.kind != output_frame_kind(&frame.kind) {
            return Err(AuditAssertionError::OutputFrameMismatch {
                key: frame.output_key,
                field: "kind",
            });
        }
    }
    Ok(())
}

/// Asserts every output frame has a graph-visible revision explanation.
pub fn assert_every_output_frame_has_revision<C, O>(
    graph: &Graph<C, O>,
    result: &TransactionResult<C, O>,
) -> Result<(), AuditAssertionError> {
    assert_no_unexplained_output_frame(graph, result)
}

/// Asserts every output frame has a graph-visible scope explanation.
pub fn assert_every_output_frame_has_scope<C, O>(
    graph: &Graph<C, O>,
    result: &TransactionResult<C, O>,
) -> Result<(), AuditAssertionError> {
    assert_no_unexplained_output_frame(graph, result)
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
