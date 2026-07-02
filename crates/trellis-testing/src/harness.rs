use std::fmt::Debug;

use trellis_core::{
    AuditExplanationLevel, Graph, GraphResult, InvariantResultTrace, OutputFrameTrace,
    ResourceCommandTrace, Transaction, TransactionOptions,
};

use crate::harness_step::{HarnessStep, NamedInvariantCheck};
use crate::{
    DataTransactionScript, FullRecomputeOracle, OracleCheck, OracleMismatch, OutputLedger,
    ResourceLedger, Scenario, ScenarioError, StageOperation, TransactionScript,
};

/// Application target that exposes the Trellis graph under test.
pub trait ScenarioTarget<C = ()> {
    /// Returns the underlying graph.
    fn graph(&self) -> &Graph<C>;

    /// Returns the underlying graph mutably.
    fn graph_mut(&mut self) -> &mut Graph<C>;
}

impl<C> ScenarioTarget<C> for Graph<C> {
    fn graph(&self) -> &Graph<C> {
        self
    }

    fn graph_mut(&mut self) -> &mut Graph<C> {
        self
    }
}

/// Scenario runner for deterministic transaction scripts.
pub struct TrellisHarness<G, C = ()> {
    target: G,
    scenario: Scenario,
    resource_ledger: ResourceLedger<C>,
    output_ledger: OutputLedger,
}

impl<G, C> TrellisHarness<G, C>
where
    G: ScenarioTarget<C>,
    C: Clone + Debug + PartialEq,
{
    /// Builds a harness from an application-supplied constructor.
    pub fn new(build: impl FnOnce() -> G) -> Self {
        Self::from_target(build())
    }

    /// Builds a harness around an already-constructed target.
    pub fn from_target(target: G) -> Self {
        Self {
            target,
            scenario: Scenario::new(),
            resource_ledger: ResourceLedger::new(),
            output_ledger: OutputLedger::new(),
        }
    }

    /// Returns the wrapped application target.
    pub fn target(&self) -> &G {
        &self.target
    }

    /// Returns the recorded scenario.
    pub fn scenario(&self) -> &Scenario {
        &self.scenario
    }

    /// Returns the resource ledger updated after each committed step.
    pub fn resource_ledger(&self) -> &ResourceLedger<C> {
        &self.resource_ledger
    }

    /// Returns the output ledger updated after each committed step.
    pub fn output_ledger(&self) -> &OutputLedger {
        &self.output_ledger
    }

    /// Starts a named single-transaction step.
    pub fn step(&mut self, name: impl Into<String>) -> HarnessStep<'_, G, C> {
        HarnessStep::new(self, name.into())
    }

    /// Runs every step in a replayable transaction script.
    pub fn run_script(&mut self, script: &TransactionScript<C>) -> Result<(), ScenarioError> {
        for step in script.steps() {
            self.commit_operations(step.name(), &step.operations, &[], None, None)?;
        }
        Ok(())
    }

    /// Runs every step in a serializable data transaction script.
    pub fn run_data_script<Operation>(
        &mut self,
        script: &DataTransactionScript<Operation>,
        mut apply: impl for<'tx> FnMut(&Operation, &mut Transaction<'tx, C>) -> GraphResult<()>,
    ) -> Result<(), ScenarioError> {
        script.validate_format_version()?;
        for step in script.steps() {
            self.commit_data_operations(step.name(), step.operations(), &mut apply)?;
        }
        Ok(())
    }

    /// Replays a transaction script against a fresh application graph.
    pub fn replay(
        build: impl FnOnce() -> G,
        script: &TransactionScript<C>,
    ) -> Result<Self, ScenarioError> {
        let mut harness = Self::new(build);
        harness.run_script(script)?;
        Ok(harness)
    }

    /// Replays a serializable data transaction script against a fresh graph.
    pub fn replay_data<Operation>(
        build: impl FnOnce() -> G,
        script: &DataTransactionScript<Operation>,
        apply: impl for<'tx> FnMut(&Operation, &mut Transaction<'tx, C>) -> GraphResult<()>,
    ) -> Result<Self, ScenarioError> {
        let mut harness = Self::new(build);
        harness.run_data_script(script, apply)?;
        Ok(harness)
    }

    /// Compares replay traces and final graph state.
    pub fn assert_replay_matches(&self, other: &Self) -> Result<(), ScenarioError> {
        self.scenario.assert_replay_matches(&other.scenario)?;
        let expected = self.final_state_debug_dump();
        let actual = other.final_state_debug_dump();
        if expected != actual {
            return Err(ScenarioError::ReplayFinalStateMismatch { expected, actual });
        }
        assert_equal_debug(
            "resource_command_records",
            self.resource_ledger.command_records(),
            other.resource_ledger.command_records(),
        )?;
        assert_equal_debug(
            "output_frame_records",
            self.output_ledger.frame_records(),
            other.output_ledger.frame_records(),
        )?;
        assert_equal_debug(
            "resource_ledger_snapshots",
            &self.resource_ledger,
            &other.resource_ledger,
        )?;
        assert_equal_debug(
            "output_ledger_snapshots",
            &self.output_ledger,
            &other.output_ledger,
        )?;
        Ok(())
    }

    /// Returns a deterministic graph metadata dump for final-state comparison.
    pub fn final_state_debug_dump(&self) -> String {
        self.target.graph().debug_dump()
    }

    /// Runs an app-owned full-recompute oracle against the wrapped target.
    pub fn assert_oracle<Oracle>(
        &self,
        inputs: &Oracle::CanonicalInputs,
    ) -> Result<OracleCheck<Oracle::ExpectedState>, OracleMismatch<Oracle::ExpectedState>>
    where
        Oracle: FullRecomputeOracle<G>,
    {
        crate::assert_incremental_equals_full::<G, Oracle>(&self.target, inputs)
    }

    pub(crate) fn commit_operations(
        &mut self,
        name: &str,
        operations: &[Box<StageOperation<C>>],
        invariant_checks: &[NamedInvariantCheck<G, C>],
        expected_resource_commands: Option<&[ResourceCommandTrace]>,
        expected_output_frames: Option<&[OutputFrameTrace]>,
    ) -> Result<(), ScenarioError> {
        self.scenario.ensure_step_name_available(name)?;
        let result = {
            let graph = self.target.graph_mut();
            let mut tx = graph
                .begin_transaction_with_options(harness_transaction_options())
                .map_err(|error| step_commit_failed(name, error))?;
            for operation in operations {
                operation(&mut tx).map_err(|error| step_commit_failed(name, error))?;
            }
            tx.commit()
                .map_err(|error| step_commit_failed(name, error))?
        };

        let mut trace = result.trace();
        for check in invariant_checks {
            let passed = (check.check)(&self.target, &result);
            trace.invariant_results.push(InvariantResultTrace {
                name: check.name.clone(),
                passed,
            });
            if !passed {
                return Err(ScenarioError::InvariantFailed {
                    step: name.to_owned(),
                    invariant: check.name.clone(),
                    transaction_id: result.transaction_id,
                    revision: result.revision,
                });
            }
        }

        self.resource_ledger.apply_result(&result);
        self.output_ledger.apply_result(&result);
        self.resource_ledger
            .assert_graph_has_no_orphan_resources(self.target.graph())
            .map_err(|error| ScenarioError::ResourceLedgerInvariantFailed {
                step: name.to_owned(),
                error: Box::new(error),
            })?;
        self.scenario.record_trace(name, trace)?;

        if let Some(expected) = expected_resource_commands {
            self.scenario
                .assert_step_resource_commands(name, expected)?;
        }
        if let Some(expected) = expected_output_frames {
            self.scenario.assert_step_output_frames(name, expected)?;
        }
        Ok(())
    }

    fn commit_data_operations<Operation>(
        &mut self,
        name: &str,
        operations: &[Operation],
        apply: &mut impl for<'tx> FnMut(&Operation, &mut Transaction<'tx, C>) -> GraphResult<()>,
    ) -> Result<(), ScenarioError> {
        self.scenario.ensure_step_name_available(name)?;
        let result = {
            let graph = self.target.graph_mut();
            let mut tx = graph
                .begin_transaction_with_options(harness_transaction_options())
                .map_err(|error| step_commit_failed(name, error))?;
            for operation in operations {
                apply(operation, &mut tx).map_err(|error| step_commit_failed(name, error))?;
            }
            tx.commit()
                .map_err(|error| step_commit_failed(name, error))?
        };

        self.resource_ledger.apply_result(&result);
        self.output_ledger.apply_result(&result);
        self.resource_ledger
            .assert_graph_has_no_orphan_resources(self.target.graph())
            .map_err(|error| ScenarioError::ResourceLedgerInvariantFailed {
                step: name.to_owned(),
                error: Box::new(error),
            })?;
        self.scenario.record(name, &result)
    }
}

fn harness_transaction_options() -> TransactionOptions {
    TransactionOptions::default().with_audit_explanations(AuditExplanationLevel::DependencyPaths)
}

fn step_commit_failed(step: &str, error: trellis_core::GraphError) -> ScenarioError {
    ScenarioError::StepCommitFailed {
        step: step.to_owned(),
        error,
    }
}

fn assert_equal_debug<T>(field: &'static str, expected: &T, actual: &T) -> Result<(), ScenarioError>
where
    T: Debug + PartialEq + ?Sized,
{
    if expected == actual {
        Ok(())
    } else {
        Err(ScenarioError::ReplayLedgerMismatch {
            field,
            expected: format!("{expected:#?}"),
            actual: format!("{actual:#?}"),
        })
    }
}
