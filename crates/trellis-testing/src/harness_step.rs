use std::fmt::Debug;

use trellis_core::{
    GraphResult, InputNode, OutputFrameTrace, ResourceCommandTrace, Transaction, TransactionResult,
};

use crate::{ScenarioError, ScenarioTarget, StageOperation, TrellisHarness};

pub(crate) type InvariantCheck<G, C, O> = dyn Fn(&G, &TransactionResult<C, O>) -> bool + 'static;

pub(crate) struct NamedInvariantCheck<G, C, O> {
    pub(crate) name: String,
    pub(crate) check: Box<InvariantCheck<G, C, O>>,
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

impl<'harness, G, C, O> HarnessStep<'harness, G, C, O> {
    pub(crate) fn new(harness: &'harness mut TrellisHarness<G, C, O>, name: String) -> Self {
        Self {
            harness,
            name,
            operations: Vec::new(),
            expected_resource_commands: None,
            expected_output_frames: None,
            invariant_checks: Vec::new(),
        }
    }
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
