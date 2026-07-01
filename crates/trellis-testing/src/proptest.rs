//! Optional `proptest` strategy helpers for Trellis model scripts.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use proptest::collection::{btree_set, vec};
use proptest::prelude::{Just, Strategy, prop_oneof};
use trellis_core::testing::{ModelScript, ModelStep, ModelTopology};

mod sequence;

pub use sequence::*;

/// Produces replayable Trellis model scripts with shrinkable model steps.
pub fn model_script_strategy(max_steps: usize) -> impl Strategy<Value = ModelScript> {
    (
        prop_oneof![
            Just(ModelTopology::ScalarChain),
            Just(ModelTopology::SetResourceOutput),
        ],
        vec(model_step_strategy(), 0..=max_steps),
    )
        .prop_map(|(topology, steps)| ModelScript { topology, steps })
}

/// Produces shrinkable Trellis core model steps.
pub fn model_step_strategy() -> impl Strategy<Value = ModelStep> {
    prop_oneof![
        member_set_strategy().prop_map(ModelStep::SetMembers),
        Just(ModelStep::RebaselineOutput),
        Just(ModelStep::ClosePrimaryScope),
    ]
}

/// Returns deterministic replay-friendly debug text for a core model script.
pub fn model_script_replay_debug(script: &ModelScript) -> String {
    let mut output = format!("topology: {:?}\n", script.topology);
    for (index, step) in script.steps.iter().enumerate() {
        let _ = writeln!(output, "{index}: {step:?}");
    }
    output
}

fn member_set_strategy() -> impl Strategy<Value = BTreeSet<u8>> {
    btree_set(0u8..6, 0..=3)
}
