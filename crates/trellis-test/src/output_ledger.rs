use std::collections::{BTreeMap, BTreeSet};

use trellis_core::{
    OutputFrame, OutputFrameKind, OutputKey, Revision, ScopeId, TransactionId, TransactionResult,
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
}

/// Output ledger assertion failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OutputLedgerError {
    /// A frame revision moved backward.
    RevisionRegression {
        /// Output key.
        key: OutputKey,
        /// Previous revision.
        previous: Revision,
        /// New revision.
        next: Revision,
    },
    /// Output was not cleared.
    NotCleared(OutputKey),
    /// A closed scope emitted a non-terminal frame.
    FrameAfterClosedScope {
        /// Output key.
        key: OutputKey,
        /// Scope that was already closed.
        scope: ScopeId,
    },
    /// Current state differs from an expected baseline/rebaseline.
    StateMismatch(OutputKey),
}

/// Fake output consumer ledger for materialized output frames.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OutputLedger<O> {
    outputs: BTreeMap<OutputKey, OutputSnapshot<O>>,
    closed_scopes: BTreeSet<ScopeId>,
    errors: Vec<OutputLedgerError>,
}

impl<O: Clone + PartialEq> OutputLedger<O> {
    /// Creates an empty output ledger.
    pub fn new() -> Self {
        Self {
            outputs: BTreeMap::new(),
            closed_scopes: BTreeSet::new(),
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
        if self.closed_scopes.contains(&frame.scope)
            && !matches!(frame.kind, OutputFrameKind::Clear(_))
        {
            self.errors.push(OutputLedgerError::FrameAfterClosedScope {
                key: frame.output_key,
                scope: frame.scope,
            });
        }

        if let Some(previous) = self.outputs.get(&frame.output_key)
            && frame.revision < previous.revision
        {
            self.errors.push(OutputLedgerError::RevisionRegression {
                key: frame.output_key,
                previous: previous.revision,
                next: frame.revision,
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
            },
        );
    }

    /// Returns current output state.
    pub fn snapshot(&self, key: OutputKey) -> Option<&OutputSnapshot<O>> {
        self.outputs.get(&key)
    }

    /// Asserts no revision regressions or closed-scope frame errors occurred.
    pub fn assert_revision_monotonic(&self) -> Result<(), OutputLedgerError> {
        if let Some(error) = self.errors.first() {
            Err(error.clone())
        } else {
            Ok(())
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
            Err(OutputLedgerError::NotCleared(key))
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
            Err(OutputLedgerError::StateMismatch(key))
        }
    }
}
