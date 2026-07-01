use std::fmt::Debug;

/// Application-owned full-recompute oracle for a Trellis graph wrapper.
pub trait FullRecomputeOracle<G> {
    /// Canonical application inputs used by full recompute.
    type CanonicalInputs;
    /// Comparable state observed from full recompute and incremental graph state.
    type ExpectedState: Clone + Debug + PartialEq;

    /// Computes expected state from canonical application truth.
    fn recompute(inputs: &Self::CanonicalInputs) -> Self::ExpectedState;

    /// Observes the equivalent state from the incremental graph or wrapper.
    fn observe_incremental(graph: &G, inputs: &Self::CanonicalInputs) -> Self::ExpectedState;
}

/// Successful oracle comparison.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleCheck<S> {
    /// State produced by full recompute.
    pub expected: S,
    /// State observed from the incremental graph.
    pub actual: S,
}

/// Full-recompute oracle mismatch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleMismatch<S> {
    /// State produced by full recompute.
    pub expected: S,
    /// State observed from the incremental graph.
    pub actual: S,
}

/// Asserts that incremental observation equals application full recompute.
pub fn assert_incremental_equals_full<G, O>(
    graph: &G,
    inputs: &O::CanonicalInputs,
) -> Result<OracleCheck<O::ExpectedState>, OracleMismatch<O::ExpectedState>>
where
    O: FullRecomputeOracle<G>,
{
    let expected = O::recompute(inputs);
    let actual = O::observe_incremental(graph, inputs);
    if expected == actual {
        Ok(OracleCheck { expected, actual })
    } else {
        Err(OracleMismatch { expected, actual })
    }
}
