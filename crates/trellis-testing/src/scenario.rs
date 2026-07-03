use trellis_core::{
    GraphError, OutputFrameTrace, ResourceCommandTrace, ResourceKey, Revision, TraceMismatch,
    TransactionId, TransactionResult, TransactionTrace, assert_transaction_traces_match,
};

use crate::{
    FullRecomputeOracle, OracleCheck, OracleMismatch, ResourceLedgerError,
    assert_incremental_equals_full,
};

/// Named transaction trace captured by a scenario test.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScenarioStep {
    /// Human-readable step name.
    pub name: String,
    /// Structural transaction trace for this step.
    pub trace: TransactionTrace,
}

/// Deterministic scenario recorder for transaction scripts.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Scenario {
    steps: Vec<ScenarioStep>,
}

/// Scenario assertion failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScenarioError {
    /// The replay trace sequence differed.
    ReplayMismatch(TraceMismatch),
    /// A serialized script or trace used an unsupported format version.
    TraceFormatVersionMismatch {
        /// Format version supported by this crate.
        expected: u32,
        /// Format version found in the serialized artifact.
        actual: u32,
    },
    /// The final deterministic graph dump differed after replay.
    ReplayFinalStateMismatch {
        /// Expected final graph dump.
        expected: String,
        /// Actual final graph dump.
        actual: String,
    },
    /// The replayed typed ledger state differed.
    ReplayLedgerMismatch {
        /// Ledger field whose value differed.
        field: &'static str,
        /// Expected typed ledger state.
        expected: String,
        /// Actual typed ledger state.
        actual: String,
    },
    /// A named step was not found.
    MissingStep(String),
    /// A step name was recorded more than once.
    DuplicateStep {
        /// Duplicate step name.
        step: String,
    },
    /// A named step had different structural data.
    StepMismatch {
        /// Step whose assertion failed.
        step: String,
        /// Transaction that produced the mismatched structural value.
        transaction_id: TransactionId,
        /// Graph revision at the mismatched step.
        revision: Revision,
        /// Trace field whose value differed.
        field: &'static str,
        /// Expected structural value.
        expected: String,
        /// Actual structural value.
        actual: String,
    },
    /// A scenario step failed while staging or committing a transaction.
    StepCommitFailed {
        /// Step whose transaction failed.
        step: String,
        /// Graph error returned by core.
        error: GraphError,
    },
    /// A resource-ledger invariant failed after a committed step.
    ResourceLedgerInvariantFailed {
        /// Step whose transaction produced the failed invariant.
        step: String,
        /// Ledger error returned by the invariant.
        error: Box<ResourceLedgerError>,
    },
    /// A step-level invariant hook failed.
    InvariantFailed {
        /// Step whose invariant failed.
        step: String,
        /// Stable invariant name.
        invariant: String,
        /// Transaction that produced the failure.
        transaction_id: TransactionId,
        /// Graph revision at the failed invariant.
        revision: Revision,
    },
}

/// Redaction hook for scenario debug and snapshot output.
pub trait TraceRedactor {
    /// Redacts a scenario step name.
    fn step_name(&self, name: &str) -> String {
        name.to_owned()
    }

    /// Redacts a resource key.
    fn resource_key(&self, key: &ResourceKey) -> ResourceKey {
        key.clone()
    }

    /// Redacts an invariant name.
    fn invariant_name(&self, name: &str) -> String {
        name.to_owned()
    }
}

/// Redactor that preserves all trace data.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct NoRedaction;

impl TraceRedactor for NoRedaction {}

impl Scenario {
    /// Creates an empty scenario recorder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a committed transaction result under a stable step name.
    pub fn record<C>(
        &mut self,
        name: impl Into<String>,
        result: &TransactionResult<C>,
    ) -> Result<(), ScenarioError> {
        self.record_trace(name, result.trace())
    }

    /// Records an already-built structural transaction trace under a step name.
    pub fn record_trace(
        &mut self,
        name: impl Into<String>,
        trace: TransactionTrace,
    ) -> Result<(), ScenarioError> {
        let name = name.into();
        self.ensure_step_name_available(&name)?;
        self.steps.push(ScenarioStep { name, trace });
        Ok(())
    }

    /// Returns all recorded steps in commit order.
    pub fn steps(&self) -> &[ScenarioStep] {
        &self.steps
    }

    /// Returns a named step.
    pub fn step(&self, name: &str) -> Result<&ScenarioStep, ScenarioError> {
        self.steps
            .iter()
            .find(|step| step.name == name)
            .ok_or_else(|| ScenarioError::MissingStep(name.to_owned()))
    }

    pub(crate) fn ensure_step_name_available(&self, name: &str) -> Result<(), ScenarioError> {
        if self.steps.iter().any(|step| step.name == name) {
            Err(ScenarioError::DuplicateStep {
                step: name.to_owned(),
            })
        } else {
            Ok(())
        }
    }

    /// Compares two scenario trace sequences structurally.
    pub fn assert_replay_matches(&self, other: &Scenario) -> Result<(), ScenarioError> {
        assert_transaction_traces_match(&self.traces(), &other.traces())
            .map_err(ScenarioError::ReplayMismatch)
    }

    /// Returns all transaction traces in commit order.
    pub fn traces(&self) -> Vec<TransactionTrace> {
        self.steps
            .iter()
            .map(|step| step.trace.clone())
            .collect::<Vec<_>>()
    }

    /// Returns all resource command traces in commit order.
    pub fn resource_commands(&self) -> Vec<ResourceCommandTrace> {
        self.steps
            .iter()
            .flat_map(|step| step.trace.resource_commands.iter().cloned())
            .collect()
    }

    /// Returns all output frame traces in commit order.
    pub fn output_frames(&self) -> Vec<OutputFrameTrace> {
        self.steps
            .iter()
            .flat_map(|step| step.trace.output_frames.iter().cloned())
            .collect()
    }

    /// Asserts a named step emitted the expected resource command trace.
    pub fn assert_step_resource_commands(
        &self,
        name: &str,
        expected: &[ResourceCommandTrace],
    ) -> Result<(), ScenarioError> {
        let step = self.step(name)?;
        if step.trace.resource_commands == expected {
            Ok(())
        } else {
            Err(ScenarioError::StepMismatch {
                step: name.to_owned(),
                transaction_id: step.trace.transaction_id,
                revision: step.trace.revision,
                field: "resource_commands",
                expected: format!("{expected:#?}"),
                actual: format!("{:#?}", step.trace.resource_commands),
            })
        }
    }

    /// Asserts a named step emitted the expected output frame trace.
    pub fn assert_step_output_frames(
        &self,
        name: &str,
        expected: &[OutputFrameTrace],
    ) -> Result<(), ScenarioError> {
        let step = self.step(name)?;
        if step.trace.output_frames == expected {
            Ok(())
        } else {
            Err(ScenarioError::StepMismatch {
                step: name.to_owned(),
                transaction_id: step.trace.transaction_id,
                revision: step.trace.revision,
                field: "output_frames",
                expected: format!("{expected:#?}"),
                actual: format!("{:#?}", step.trace.output_frames),
            })
        }
    }

    /// Runs an app-owned oracle check through the scenario harness.
    pub fn assert_oracle<G, O>(
        &self,
        graph: &G,
        inputs: &O::CanonicalInputs,
    ) -> Result<OracleCheck<O::ExpectedState>, OracleMismatch<O::ExpectedState>>
    where
        O: FullRecomputeOracle<G>,
    {
        assert_incremental_equals_full::<G, O>(graph, inputs)
    }

    /// Returns a redacted copy of the scenario for snapshot/debug output.
    pub fn redacted(&self, redactor: &impl TraceRedactor) -> Self {
        let steps = self
            .steps
            .iter()
            .map(|step| ScenarioStep {
                name: redactor.step_name(&step.name),
                trace: redact_trace(&step.trace, redactor),
            })
            .collect();
        Self { steps }
    }

    /// Returns deterministic redacted debug output for snapshots.
    pub fn to_redacted_debug_string(&self, redactor: &impl TraceRedactor) -> String {
        format!("{:#?}", self.redacted(redactor))
    }
}

fn redact_trace(trace: &TransactionTrace, redactor: &impl TraceRedactor) -> TransactionTrace {
    let mut trace = trace.clone();
    for command in &mut trace.resource_commands {
        command.key = redactor.resource_key(&command.key);
    }
    for coalesced in &mut trace.resource_coalescences {
        coalesced.key = redactor.resource_key(&coalesced.key);
    }
    for result in &mut trace.invariant_results {
        result.name = redactor.invariant_name(&result.name);
    }
    trace
}
