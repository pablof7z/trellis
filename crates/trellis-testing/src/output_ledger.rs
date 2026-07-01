use std::collections::{BTreeMap, BTreeSet};

use trellis_core::{
    OutputFrame, OutputFrameKind, OutputFrameKindTrace, OutputFrameTrace, OutputKey, Revision,
    ScopeId, TransactionId, TransactionResult,
};

/// Current ledger view for one materialized output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputSnapshot<O> {
    /// Scope that owns the output.
    pub scope: ScopeId,
    /// Last transaction that emitted a frame.
    pub transaction_id: TransactionId,
    /// Last revision observed for this output.
    pub revision: Revision,
    /// Current consumer state after applying frames.
    pub state: Option<O>,
    /// Whether a clear frame has been observed.
    pub cleared: bool,
    /// Last frame trace observed for this output.
    pub frame: OutputFrameTrace,
}

/// Output ledger assertion failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OutputLedgerError {
    /// A frame revision moved backward.
    RevisionRegression {
        /// Frame that regressed.
        context: OutputFrameTrace,
        /// Previous revision.
        previous: Revision,
    },
    /// Output was not cleared.
    NotCleared {
        /// Output key.
        key: OutputKey,
        /// Last frame context for the output, if any.
        context: Option<OutputFrameTrace>,
    },
    /// A closed scope emitted a non-terminal frame.
    FrameAfterClosedScope {
        /// Frame that targeted the closed scope.
        context: OutputFrameTrace,
    },
    /// Outputs owned by a closed scope were not cleared.
    ClosedScopeNotCleared {
        /// Closed scope.
        scope: ScopeId,
        /// Output keys that remain uncleared.
        outputs: Vec<OutputKey>,
        /// Last frame contexts for uncleared outputs.
        contexts: Vec<OutputFrameTrace>,
    },
    /// Current state differs from an expected baseline/rebaseline.
    StateMismatch {
        /// Output key.
        key: OutputKey,
        /// Last frame context for the output, if any.
        context: Option<OutputFrameTrace>,
    },
}

/// Fake output consumer ledger for materialized output frames.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OutputLedger<O> {
    pub(crate) outputs: BTreeMap<OutputKey, OutputSnapshot<O>>,
    pub(crate) closed_scopes: BTreeSet<ScopeId>,
    pub(crate) frames: Vec<OutputFrameTrace>,
    pub(crate) frame_records: Vec<OutputFrame<O>>,
    pub(crate) errors: Vec<OutputLedgerError>,
}

impl<O: Clone + PartialEq> OutputLedger<O> {
    /// Creates an empty output ledger.
    pub fn new() -> Self {
        Self {
            outputs: BTreeMap::new(),
            closed_scopes: BTreeSet::new(),
            frames: Vec::new(),
            frame_records: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Marks a scope closed for later frame validation.
    pub fn close_scope(&mut self, scope: ScopeId) {
        self.closed_scopes.insert(scope);
    }

    /// Applies all output frames from a transaction result.
    pub fn apply_result<C>(&mut self, result: &TransactionResult<C, O>) {
        for frame in &result.output_frames {
            self.apply_frame(frame);
        }
    }

    /// Applies a single output frame.
    pub fn apply_frame(&mut self, frame: &OutputFrame<O>) {
        let trace = output_frame_trace(frame);
        self.frames.push(trace.clone());
        self.frame_records.push(frame.clone());
        if self.closed_scopes.contains(&frame.scope)
            && !matches!(frame.kind, OutputFrameKind::Clear(_))
        {
            self.errors
                .push(OutputLedgerError::FrameAfterClosedScope { context: trace });
            return;
        }

        if let Some(previous) = self.outputs.get(&frame.output_key)
            && frame.revision < previous.revision
        {
            self.errors.push(OutputLedgerError::RevisionRegression {
                context: trace.clone(),
                previous: previous.revision,
            });
        }

        let state = match &frame.kind {
            OutputFrameKind::Baseline(value)
            | OutputFrameKind::Delta(value)
            | OutputFrameKind::Rebaseline(value, _) => Some(value.clone()),
            OutputFrameKind::Clear(_) => None,
        };
        self.outputs.insert(
            frame.output_key,
            OutputSnapshot {
                scope: frame.scope,
                transaction_id: frame.transaction_id,
                revision: frame.revision,
                state,
                cleared: matches!(frame.kind, OutputFrameKind::Clear(_)),
                frame: trace,
            },
        );
    }

    /// Returns current output state.
    pub fn snapshot(&self, key: OutputKey) -> Option<&OutputSnapshot<O>> {
        self.outputs.get(&key)
    }

    /// Returns structural ledger errors observed while applying frames.
    pub fn errors(&self) -> &[OutputLedgerError] {
        &self.errors
    }

    /// Returns frame traces in applied delivery order.
    pub fn frame_trace(&self) -> &[OutputFrameTrace] {
        &self.frames
    }

    /// Returns applied output frames including typed payloads in delivery order.
    pub fn frame_records(&self) -> &[OutputFrame<O>] {
        &self.frame_records
    }

    /// Asserts no revision regressions or closed-scope frame errors occurred.
    pub fn assert_revision_monotonic(&self) -> Result<(), OutputLedgerError> {
        self.errors
            .iter()
            .find(|error| matches!(error, OutputLedgerError::RevisionRegression { .. }))
            .cloned()
            .map_or(Ok(()), Err)
    }

    /// Asserts closed scopes emitted no non-terminal output frames.
    pub fn assert_no_frame_for_closed_scope_except_terminal(
        &self,
    ) -> Result<(), OutputLedgerError> {
        self.errors
            .iter()
            .find(|error| matches!(error, OutputLedgerError::FrameAfterClosedScope { .. }))
            .cloned()
            .map_or(Ok(()), Err)
    }

    /// Asserts every output owned by a closed scope has been cleared.
    pub fn assert_closed_scope_cleared(&self, scope: ScopeId) -> Result<(), OutputLedgerError> {
        let uncleared = self
            .outputs
            .iter()
            .filter(|(_, snapshot)| snapshot.scope == scope && !snapshot.cleared)
            .map(|(key, _)| *key)
            .collect::<Vec<_>>();
        if uncleared.is_empty() {
            Ok(())
        } else {
            let contexts = uncleared
                .iter()
                .filter_map(|key| self.outputs.get(key).map(|snapshot| snapshot.frame.clone()))
                .collect();
            Err(OutputLedgerError::ClosedScopeNotCleared {
                scope,
                outputs: uncleared,
                contexts,
            })
        }
    }

    /// Asserts an output key is currently cleared.
    pub fn assert_cleared(&self, key: OutputKey) -> Result<(), OutputLedgerError> {
        if self
            .outputs
            .get(&key)
            .is_some_and(|snapshot| snapshot.cleared)
        {
            Ok(())
        } else {
            Err(OutputLedgerError::NotCleared {
                key,
                context: self
                    .outputs
                    .get(&key)
                    .map(|snapshot| snapshot.frame.clone()),
            })
        }
    }

    /// Asserts the current consumer state equals an expected baseline.
    pub fn assert_current_equals(
        &self,
        key: OutputKey,
        expected: &O,
    ) -> Result<(), OutputLedgerError> {
        if self
            .outputs
            .get(&key)
            .and_then(|snapshot| snapshot.state.as_ref())
            == Some(expected)
        {
            Ok(())
        } else {
            Err(OutputLedgerError::StateMismatch {
                key,
                context: self
                    .outputs
                    .get(&key)
                    .map(|snapshot| snapshot.frame.clone()),
            })
        }
    }

    /// Asserts the current delta-applied state matches a rebaseline value.
    pub fn assert_delta_sequence_matches_rebaseline(
        &self,
        key: OutputKey,
        rebaseline: &O,
    ) -> Result<(), OutputLedgerError> {
        self.assert_current_equals(key, rebaseline)
    }

    /// Asserts the ledger observed no structural frame errors.
    pub fn assert_consumer_needs_no_hidden_graph_state(&self) -> Result<(), OutputLedgerError> {
        self.errors.first().cloned().map_or(Ok(()), Err)
    }
}

fn output_frame_trace<O>(frame: &OutputFrame<O>) -> OutputFrameTrace {
    OutputFrameTrace {
        output_key: frame.output_key,
        scope: frame.scope,
        transaction_id: frame.transaction_id,
        revision: frame.revision,
        kind: output_frame_kind(&frame.kind),
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
