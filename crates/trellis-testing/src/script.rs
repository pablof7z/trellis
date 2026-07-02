use trellis_core::{GraphResult, InputNode, Transaction};

pub(crate) type StageOperation<C> =
    dyn for<'tx> Fn(&mut Transaction<'tx, C>) -> GraphResult<()> + 'static;

/// Deterministic transaction script that can be replayed against a fresh graph.
pub struct TransactionScript<C = ()> {
    pub(crate) steps: Vec<TransactionScriptStep<C>>,
}

impl<C> TransactionScript<C> {
    /// Creates an empty transaction script.
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    /// Starts a named script step.
    pub fn step(&mut self, name: impl Into<String>) -> TransactionScriptStepBuilder<'_, C> {
        TransactionScriptStepBuilder {
            script: self,
            name: name.into(),
            operations: Vec::new(),
        }
    }

    /// Returns script steps in replay order.
    pub fn steps(&self) -> &[TransactionScriptStep<C>] {
        &self.steps
    }
}

impl<C> Default for TransactionScript<C> {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for one transaction script step.
pub struct TransactionScriptStepBuilder<'script, C> {
    script: &'script mut TransactionScript<C>,
    name: String,
    operations: Vec<Box<StageOperation<C>>>,
}

impl<C> TransactionScriptStepBuilder<'_, C> {
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
        operation: impl for<'tx> Fn(&mut Transaction<'tx, C>) -> GraphResult<()> + 'static,
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
pub struct TransactionScriptStep<C = ()> {
    pub(crate) name: String,
    pub(crate) operations: Vec<Box<StageOperation<C>>>,
}

impl<C> TransactionScriptStep<C> {
    /// Returns the step name.
    pub fn name(&self) -> &str {
        &self.name
    }
}
