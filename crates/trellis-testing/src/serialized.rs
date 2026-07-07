use trellis_core::TransactionTrace;

use crate::{Scenario, ScenarioError};

/// Version for serialized Trellis script and trace artifacts.
pub const TRACE_FORMAT_VERSION: u32 = 2;

/// A versioned, data-only transaction script.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct DataTransactionScript<Operation> {
    format_version: u32,
    steps: Vec<DataScriptStep<Operation>>,
}

/// One named transaction step in a data-only transaction script.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct DataScriptStep<Operation> {
    name: String,
    operations: Vec<Operation>,
}

/// Builder for one data-only transaction script step.
pub struct DataScriptStepBuilder<'script, Operation> {
    script: &'script mut DataTransactionScript<Operation>,
    name: String,
    operations: Vec<Operation>,
}

/// A versioned structural trace file for a recorded scenario.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SerializedScenario {
    format_version: u32,
    steps: Vec<SerializedScenarioStep>,
}

/// One named transaction trace in a versioned scenario trace file.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SerializedScenarioStep {
    /// Human-readable step name.
    pub name: String,
    /// Structural transaction trace for this step.
    pub trace: TransactionTrace,
}

impl<Operation> DataTransactionScript<Operation> {
    /// Creates an empty versioned data script.
    pub fn new() -> Self {
        Self {
            format_version: TRACE_FORMAT_VERSION,
            steps: Vec::new(),
        }
    }

    /// Starts a named data script step.
    pub fn step(&mut self, name: impl Into<String>) -> DataScriptStepBuilder<'_, Operation> {
        DataScriptStepBuilder {
            script: self,
            name: name.into(),
            operations: Vec::new(),
        }
    }

    /// Returns the serialized format version.
    pub fn format_version(&self) -> u32 {
        self.format_version
    }

    /// Returns script steps in replay order.
    pub fn steps(&self) -> &[DataScriptStep<Operation>] {
        &self.steps
    }

    /// Fails when the script uses an unsupported format version.
    pub fn validate_format_version(&self) -> Result<(), ScenarioError> {
        validate_format_version(self.format_version)
    }

    /// Serializes this data script to pretty JSON.
    #[cfg(feature = "serde")]
    pub fn to_json(&self) -> Result<String, serde_json::Error>
    where
        Operation: serde::Serialize,
    {
        serde_json::to_string_pretty(self)
    }

    /// Deserializes a data script from JSON.
    #[cfg(feature = "serde")]
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error>
    where
        Operation: serde::de::DeserializeOwned,
    {
        serde_json::from_str(json)
    }
}

impl<Operation> Default for DataTransactionScript<Operation> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Operation> DataScriptStep<Operation> {
    /// Returns the step name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns data operations in staging order.
    pub fn operations(&self) -> &[Operation] {
        &self.operations
    }
}

impl<Operation> DataScriptStepBuilder<'_, Operation> {
    /// Adds one app-defined data operation to this step.
    pub fn operation(mut self, operation: Operation) -> Self {
        self.operations.push(operation);
        self
    }

    /// Adds this step to the script.
    pub fn commit(self) {
        self.script.steps.push(DataScriptStep {
            name: self.name,
            operations: self.operations,
        });
    }
}

impl SerializedScenario {
    /// Captures a scenario as a versioned structural trace file.
    pub fn from_scenario(scenario: &Scenario) -> Self {
        Self {
            format_version: TRACE_FORMAT_VERSION,
            steps: scenario
                .steps()
                .iter()
                .map(|step| SerializedScenarioStep {
                    name: step.name.clone(),
                    trace: step.trace.clone(),
                })
                .collect(),
        }
    }

    /// Returns the serialized format version.
    pub fn format_version(&self) -> u32 {
        self.format_version
    }

    /// Returns trace steps in commit order.
    pub fn steps(&self) -> &[SerializedScenarioStep] {
        &self.steps
    }

    /// Rebuilds a scenario recorder from this versioned trace file.
    pub fn into_scenario(self) -> Result<Scenario, ScenarioError> {
        validate_format_version(self.format_version)?;
        let mut scenario = Scenario::new();
        for step in self.steps {
            scenario.record_trace(step.name, step.trace)?;
        }
        Ok(scenario)
    }

    /// Checks this trace file against a freshly replayed scenario.
    pub fn assert_matches_scenario(&self, actual: &Scenario) -> Result<(), ScenarioError> {
        self.clone().into_scenario()?.assert_replay_matches(actual)
    }

    /// Serializes this trace file to pretty JSON.
    #[cfg(feature = "serde")]
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserializes a versioned trace file from JSON.
    #[cfg(feature = "serde")]
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

fn validate_format_version(actual: u32) -> Result<(), ScenarioError> {
    if actual == TRACE_FORMAT_VERSION {
        Ok(())
    } else {
        Err(ScenarioError::TraceFormatVersionMismatch {
            expected: TRACE_FORMAT_VERSION,
            actual,
        })
    }
}
