use std::fmt::{Debug, Write as _};
use std::ops::RangeInclusive;

use proptest::collection::vec;
use proptest::prelude::{Just, Strategy, prop_oneof};

/// Replayable generated sequence for application-owned model tests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelSequence<Step> {
    steps: Vec<Step>,
}

impl<Step> ModelSequence<Step> {
    /// Creates a generated model sequence from ordered steps.
    pub fn new(steps: Vec<Step>) -> Self {
        Self { steps }
    }

    /// Returns generated steps in replay order.
    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    /// Consumes this sequence into its ordered steps.
    pub fn into_steps(self) -> Vec<Step> {
        self.steps
    }

    /// Returns true when the generated sequence has no steps.
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Returns the generated step count.
    pub fn len(&self) -> usize {
        self.steps.len()
    }
}

impl<Step: Debug> ModelSequence<Step> {
    /// Returns deterministic replay-friendly debug text for failure output.
    pub fn to_replay_debug_string(&self) -> String {
        let mut output = String::new();
        for (index, step) in self.steps.iter().enumerate() {
            let _ = writeln!(output, "{index}: {step:?}");
        }
        output
    }
}

impl<Step> IntoIterator for ModelSequence<Step> {
    type Item = Step;
    type IntoIter = std::vec::IntoIter<Step>;

    fn into_iter(self) -> Self::IntoIter {
        self.steps.into_iter()
    }
}

/// Generated canonical input mutation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InputChange<I> {
    /// Replace the canonical input with an app-owned value.
    Set(I),
}

/// Generated scope lifecycle mutation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScopeChange<S> {
    /// Open an app-owned scope identity.
    Open(S),
    /// Close an app-owned scope identity.
    Close(S),
}

/// Generated collection mutation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CollectionChange<K, V> {
    /// Add a key/value pair.
    Add {
        /// App-owned collection key.
        key: K,
        /// App-owned collection value.
        value: V,
    },
    /// Remove a key.
    Remove {
        /// App-owned collection key.
        key: K,
    },
    /// Update a key/value pair.
    Update {
        /// App-owned collection key.
        key: K,
        /// App-owned collection value.
        value: V,
    },
}

/// Generated host resource status mutation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResourceStatusChange<R, E> {
    /// Resource command completed successfully.
    Succeeded(R),
    /// Resource command failed with an app-owned error value.
    Failed {
        /// App-owned resource identity.
        resource: R,
        /// App-owned failure value.
        error: E,
    },
}

/// Generated transaction expectation for failure/retry models.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransactionChange<E> {
    /// The next transaction is expected to fail with app-owned detail.
    ExpectFailure(E),
    /// Retry the last failed transaction when the application models retries.
    RetryLastFailure,
}

/// Generated materialized output mutation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OutputChange<O> {
    /// Ask the graph to rebaseline an app-owned output identity.
    Rebaseline(O),
}

/// Produces shrinkable generated model sequences from any app-owned step strategy.
pub fn model_sequence_strategy<Step>(
    step: impl Strategy<Value = Step>,
    len: RangeInclusive<usize>,
) -> impl Strategy<Value = ModelSequence<Step>>
where
    Step: Debug,
{
    vec(step, len).prop_map(ModelSequence::new)
}

/// Produces shrinkable canonical input changes from an app-owned value strategy.
pub fn canonical_input_change<I>(
    value: impl Strategy<Value = I>,
) -> impl Strategy<Value = InputChange<I>>
where
    I: Debug,
{
    value.prop_map(InputChange::Set)
}

/// Produces shrinkable canonical input change sequences.
pub fn canonical_input_sequence<I>(
    value: impl Strategy<Value = I>,
    len: RangeInclusive<usize>,
) -> impl Strategy<Value = ModelSequence<InputChange<I>>>
where
    I: Debug,
{
    model_sequence_strategy(canonical_input_change(value), len)
}

/// Produces shrinkable scope open/close changes.
pub fn scope_change<S>(
    open_scope: impl Strategy<Value = S>,
    close_scope: impl Strategy<Value = S>,
) -> impl Strategy<Value = ScopeChange<S>>
where
    S: Debug,
{
    prop_oneof![
        open_scope.prop_map(ScopeChange::Open),
        close_scope.prop_map(ScopeChange::Close),
    ]
}

/// Produces shrinkable scope open/close sequences.
pub fn scope_sequence<S>(
    open_scope: impl Strategy<Value = S>,
    close_scope: impl Strategy<Value = S>,
    len: RangeInclusive<usize>,
) -> impl Strategy<Value = ModelSequence<ScopeChange<S>>>
where
    S: Debug,
{
    model_sequence_strategy(scope_change(open_scope, close_scope), len)
}

/// Produces shrinkable collection add/remove/update changes.
pub fn collection_change<K, V>(
    add: impl Strategy<Value = (K, V)>,
    remove: impl Strategy<Value = K>,
    update: impl Strategy<Value = (K, V)>,
) -> impl Strategy<Value = CollectionChange<K, V>>
where
    K: Debug,
    V: Debug,
{
    prop_oneof![
        add.prop_map(|(key, value)| CollectionChange::Add { key, value }),
        remove.prop_map(|key| CollectionChange::Remove { key }),
        update.prop_map(|(key, value)| CollectionChange::Update { key, value }),
    ]
}

/// Produces shrinkable collection mutation sequences.
pub fn collection_sequence<K, V>(
    add: impl Strategy<Value = (K, V)>,
    remove: impl Strategy<Value = K>,
    update: impl Strategy<Value = (K, V)>,
    len: RangeInclusive<usize>,
) -> impl Strategy<Value = ModelSequence<CollectionChange<K, V>>>
where
    K: Debug,
    V: Debug,
{
    model_sequence_strategy(collection_change(add, remove, update), len)
}

/// Produces shrinkable host resource status success/failure changes.
pub fn resource_status_change<R, E>(
    success: impl Strategy<Value = R>,
    failure: impl Strategy<Value = (R, E)>,
) -> impl Strategy<Value = ResourceStatusChange<R, E>>
where
    R: Debug,
    E: Debug,
{
    prop_oneof![
        success.prop_map(ResourceStatusChange::Succeeded),
        failure.prop_map(|(resource, error)| ResourceStatusChange::Failed { resource, error }),
    ]
}

/// Produces shrinkable host resource status success/failure sequences.
pub fn resource_status_sequence<R, E>(
    success: impl Strategy<Value = R>,
    failure: impl Strategy<Value = (R, E)>,
    len: RangeInclusive<usize>,
) -> impl Strategy<Value = ModelSequence<ResourceStatusChange<R, E>>>
where
    R: Debug,
    E: Debug,
{
    model_sequence_strategy(resource_status_change(success, failure), len)
}

/// Produces shrinkable transaction failure/retry changes.
pub fn transaction_change<E>(
    failure: impl Strategy<Value = E>,
) -> impl Strategy<Value = TransactionChange<E>>
where
    E: Clone + Debug,
{
    prop_oneof![
        failure.prop_map(TransactionChange::ExpectFailure),
        Just(TransactionChange::RetryLastFailure),
    ]
}

/// Produces shrinkable transaction failure/retry sequences.
pub fn transaction_sequence<E>(
    failure: impl Strategy<Value = E>,
    len: RangeInclusive<usize>,
) -> impl Strategy<Value = ModelSequence<TransactionChange<E>>>
where
    E: Clone + Debug,
{
    model_sequence_strategy(transaction_change(failure), len)
}

/// Produces shrinkable output rebaseline changes.
pub fn output_rebaseline<O>(
    output: impl Strategy<Value = O>,
) -> impl Strategy<Value = OutputChange<O>>
where
    O: Debug,
{
    output.prop_map(OutputChange::Rebaseline)
}

/// Produces shrinkable output rebaseline sequences.
pub fn output_rebaseline_sequence<O>(
    output: impl Strategy<Value = O>,
    len: RangeInclusive<usize>,
) -> impl Strategy<Value = ModelSequence<OutputChange<O>>>
where
    O: Debug,
{
    model_sequence_strategy(output_rebaseline(output), len)
}
