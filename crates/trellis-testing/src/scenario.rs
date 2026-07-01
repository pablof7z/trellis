use trellis_core::{
    OutputFrameTrace, ResourceCommandTrace, ResourceKey, TraceMismatch, TransactionResult,
    TransactionTrace, assert_transaction_traces_match,
};

use crate::{FullRecomputeOracle, OracleCheck, OracleMismatch, assert_incremental_equals_full};

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
    /// A named step was not found.
    MissingStep(String),
    /// A named step had different structural data.
    StepMismatch {
        /// Step whose assertion failed.
        step: String,
        /// Trace field whose value differed.
        field: &'static str,
        /// Expected structural value.
        expected: String,
        /// Actual structural value.
        actual: String,
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
    pub fn record<C, O>(&mut self, name: impl Into<String>, result: &TransactionResult<C, O>) {
        self.steps.push(ScenarioStep {
            name: name.into(),
            trace: result.trace(),
        });
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

    /// Compares two scenario trace sequences structurally.
    pub fn assert_replay_matches(&self, other: &Scenario) -> Result<(), ScenarioError> {
        let expected = self
            .steps
            .iter()
            .map(|step| step.trace.clone())
            .collect::<Vec<_>>();
        let actual = other
            .steps
            .iter()
            .map(|step| step.trace.clone())
            .collect::<Vec<_>>();
        assert_transaction_traces_match(&expected, &actual).map_err(ScenarioError::ReplayMismatch)
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
    trace
}
