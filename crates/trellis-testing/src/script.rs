use trellis_core::{GraphResult, InputNode, Transaction};

pub(crate) type StageOperation<C, O> =
    dyn for<'tx> Fn(&mut Transaction<'tx, C, O>) -> GraphResult<()> + 'static;

/// Deterministic transaction script that can be replayed against a fresh graph.
pub struct TransactionScript<C = (), O = ()> {
    pub(crate) steps: Vec<TransactionScriptStep<C, O>>,
}

impl<C, O> TransactionScript<C, O> {
    /// Creates an empty transaction script.
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    /// Starts a named script step.
    pub fn step(&mut self, name: impl Into<String>) -> TransactionScriptStepBuilder<'_, C, O> {
        TransactionScriptStepBuilder {
            script: self,
            name: name.into(),
            operations: Vec::new(),
        }
    }

    /// Returns script steps in replay order.
    pub fn steps(&self) -> &[TransactionScriptStep<C, O>] {
        &self.steps
    }
}

impl<C, O> Default for TransactionScript<C, O> {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for one transaction script step.
pub struct TransactionScriptStepBuilder<'script, C, O> {
    script: &'script mut TransactionScript<C, O>,
    name: String,
    operations: Vec<Box<StageOperation<C, O>>>,
}

impl<C, O> TransactionScriptStepBuilder<'_, C, O>
where
    O: Clone + PartialEq,
{
    /// Stages a typed canonical input write for this step.
    pub fn input<T>(mut self, input: InputNode<T>, value: T) -> Self
    where
        T: Clone + PartialEq + 'static,
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

    /// Adds this step to the script.
    pub fn commit(self) {
        self.script.steps.push(TransactionScriptStep {
            name: self.name,
            operations: self.operations,
        });
    }
}

/// One named transaction in a replayable script.
pub struct TransactionScriptStep<C = (), O = ()> {
    pub(crate) name: String,
    pub(crate) operations: Vec<Box<StageOperation<C, O>>>,
}

impl<C, O> TransactionScriptStep<C, O> {
    /// Returns the step name.
    pub fn name(&self) -> &str {
        &self.name
    }
}
