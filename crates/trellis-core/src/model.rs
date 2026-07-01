use std::collections::BTreeSet;

/// Supported model-test graph shape generated for oracle checks.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ModelTopology {
    /// One canonical scalar input feeding a pure derived chain.
    ScalarChain,
    /// A set input feeding a collection, resource planner, and output.
    SetResourceOutput,
}

/// Deterministic mutation generated for model tests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelStep {
    /// Replace the model input set with these members.
    SetMembers(BTreeSet<u8>),
    /// Ask the graph to rebaseline its materialized output.
    RebaselineOutput,
    /// Close the primary scope.
    ClosePrimaryScope,
}

/// Generated deterministic graph shape and input sequence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelScript {
    /// Graph shape selected by the generator.
    pub topology: ModelTopology,
    /// Ordered model mutations.
    pub steps: Vec<ModelStep>,
}

/// Small deterministic generator for repeatable model tests.
#[derive(Clone, Debug)]
pub struct ModelGenerator {
    state: u64,
}

impl ModelGenerator {
    /// Creates a model generator from a stable seed.
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Generates a deterministic supported model script.
    pub fn script(&mut self, step_count: usize) -> ModelScript {
        let topology = if self.next_u8().is_multiple_of(2) {
            ModelTopology::ScalarChain
        } else {
            ModelTopology::SetResourceOutput
        };
        let mut steps = Vec::with_capacity(step_count);
        let close_at = step_count.saturating_sub(1);
        for index in 0..step_count {
            let roll = self.next_u8();
            let step = if index == close_at && step_count > 2 && roll.is_multiple_of(5) {
                ModelStep::ClosePrimaryScope
            } else if roll.is_multiple_of(7) {
                ModelStep::RebaselineOutput
            } else {
                ModelStep::SetMembers(self.members())
            };
            steps.push(step);
        }
        ModelScript { topology, steps }
    }

    fn members(&mut self) -> BTreeSet<u8> {
        let width = usize::from(self.next_u8() % 4);
        let mut members = BTreeSet::new();
        for _ in 0..width {
            members.insert(self.next_u8() % 6);
        }
        members
    }

    fn next_u8(&mut self) -> u8 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        (self.state >> 32) as u8
    }
}
