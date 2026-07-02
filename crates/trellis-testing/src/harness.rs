use std::fmt::Debug;
use std::marker::PhantomData;

use trellis_core::{
    Graph, GraphResult, InputNode, InvariantResultTrace, OutputFrameTrace, ResourceCommandTrace,
    Transaction, TransactionResult,
};

use crate::{
    FullRecomputeOracle, OracleCheck, OracleMismatch, OutputLedger, ResourceLedger, Scenario,
    ScenarioError, StageOperation, TransactionScript,
};

type InvariantCheck<G, C, O> = dyn Fn(&G, &TransactionResult<C, O>) -> bool + 'static;

/// Application target that exposes the Trellis graph under test.
pub trait ScenarioTarget<C = (), O = ()> {
    /// Returns the underlying graph.
    fn graph(&self) -> &Graph<C, O>;

    /// Returns the underlying graph mutably.
    fn graph_mut(&mut self) -> &mut Graph<C, O>;
}

impl<C, O> ScenarioTarget<C, O> for Graph<C, O> {
    fn graph(&self) -> &Graph<C, O> {
        self
    }

    fn graph_mut(&mut self) -> &mut Graph<C, O> {
        self
    }
}

/// Scenario runner for deterministic transaction scripts.
pub struct TrellisHarness<G, C = (), O = ()> {
    target: G,
    scenario: Scenario,
    resource_ledger: ResourceLedger<C>,
    output_ledger: OutputLedger<O>,
    _marker: PhantomData<fn() -> C>,
}

impl<G, C, O> TrellisHarness<G, C, O>
where
    G: ScenarioTarget<C, O>,
    C: Clone + Debug + PartialEq,
    O: Clone + Debug + PartialEq,
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
            _marker: PhantomData,
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
    pub fn output_ledger(&self) -> &OutputLedger<O> {
        &self.output_ledger
    }

    /// Starts a named single-transaction step.
    pub fn step(&mut self, name: impl Into<String>) -> HarnessStep<'_, G, C, O> {
        HarnessStep {
            harness: self,
            name: name.into(),
            operations: Vec::new(),
            expected_resource_commands: None,
            expected_output_frames: None,
            invariant_checks: Vec::new(),
        }
    }

    /// Runs every step in a replayable transaction script.
    pub fn run_script(&mut self, script: &TransactionScript<C, O>) -> Result<(), ScenarioError> {
        for step in script.steps() {
            self.commit_operations(step.name(), &step.operations, &[], None, None)?;
        }
        Ok(())
    }

    /// Replays a transaction script against a fresh application graph.
    pub fn replay(
        build: impl FnOnce() -> G,
        script: &TransactionScript<C, O>,
    ) -> Result<Self, ScenarioError> {
        let mut harness = Self::new(build);
        harness.run_script(script)?;
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

    fn commit_operations(
        &mut self,
        name: &str,
        operations: &[Box<StageOperation<C, O>>],
        invariant_checks: &[NamedInvariantCheck<G, C, O>],
        expected_resource_commands: Option<&[ResourceCommandTrace]>,
        expected_output_frames: Option<&[OutputFrameTrace]>,
    ) -> Result<(), ScenarioError> {
        let result = {
            let graph = self.target.graph_mut();
            let mut tx = graph
                .begin_transaction()
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
        self.scenario.record_trace(name, trace);

        if let Some(expected) = expected_resource_commands {
            self.scenario
                .assert_step_resource_commands(name, expected)?;
        }
        if let Some(expected) = expected_output_frames {
            self.scenario.assert_step_output_frames(name, expected)?;
        }
        Ok(())
    }
}

/// Builder for one harness transaction step.
pub struct HarnessStep<'harness, G, C, O> {
    harness: &'harness mut TrellisHarness<G, C, O>,
    name: String,
    operations: Vec<Box<StageOperation<C, O>>>,
    expected_resource_commands: Option<Vec<ResourceCommandTrace>>,
    expected_output_frames: Option<Vec<OutputFrameTrace>>,
    invariant_checks: Vec<NamedInvariantCheck<G, C, O>>,
}

impl<'harness, G, C, O> HarnessStep<'harness, G, C, O>
where
    G: ScenarioTarget<C, O>,
    C: Clone + Debug + PartialEq,
    O: Clone + Debug + PartialEq,
{
    /// Stages a typed canonical input write for this step.
    pub fn input<T>(mut self, input: InputNode<T>, value: T) -> Self
    where
        T: Clone + PartialEq + Send + Sync + 'static,
    {
        self.operations
            .push(Box::new(move |tx| tx.set_input(input, value.clone())));
        self
    }

    /// Stages a custom operation against the transaction.
    pub fn operation(
        mut self,
        operation: impl for<'tx> Fn(&mut Transaction<'tx, C, O>) -> GraphResult<()> + 'static,
    ) -> Self {
        self.operations.push(Box::new(operation));
        self
    }

    /// Expects one resource command trace in this step.
    pub fn expect_plan(mut self, command: ResourceCommandTrace) -> Self {
        self.expected_resource_commands
            .get_or_insert_with(Vec::new)
            .push(command);
        self
    }

    /// Expects the complete resource command trace for this step.
    pub fn expect_plans(
        mut self,
        commands: impl IntoIterator<Item = ResourceCommandTrace>,
    ) -> Self {
        self.expected_resource_commands = Some(commands.into_iter().collect());
        self
    }

    /// Expects one output frame trace in this step.
    pub fn expect_output(mut self, frame: OutputFrameTrace) -> Self {
        self.expected_output_frames
            .get_or_insert_with(Vec::new)
            .push(frame);
        self
    }

    /// Expects the complete output frame trace for this step.
    pub fn expect_outputs(mut self, frames: impl IntoIterator<Item = OutputFrameTrace>) -> Self {
        self.expected_output_frames = Some(frames.into_iter().collect());
        self
    }

    /// Adds a structural invariant check to record in the transaction trace.
    pub fn check(
        mut self,
        name: impl Into<String>,
        check: impl Fn(&G, &TransactionResult<C, O>) -> bool + 'static,
    ) -> Self {
        self.invariant_checks.push(NamedInvariantCheck {
            name: name.into(),
            check: Box::new(check),
        });
        self
    }

    /// Commits exactly one transaction for this step.
    pub fn commit(self) -> Result<&'harness mut TrellisHarness<G, C, O>, ScenarioError> {
        self.harness.commit_operations(
            &self.name,
            &self.operations,
            &self.invariant_checks,
            self.expected_resource_commands.as_deref(),
            self.expected_output_frames.as_deref(),
        )?;
        Ok(self.harness)
    }
}

struct NamedInvariantCheck<G, C, O> {
    name: String,
    check: Box<InvariantCheck<G, C, O>>,
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
