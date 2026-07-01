use trellis_core::{
    TraceMismatch, TransactionResult, TransactionTrace, assert_transaction_traces_match,
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
    /// A named step was not found.
    MissingStep(String),
}

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
}
