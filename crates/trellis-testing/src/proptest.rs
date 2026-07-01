//! Optional `proptest` strategy helpers for Trellis model scripts.

use proptest::prelude::{Strategy, any};
use trellis_core::testing::{ModelGenerator, ModelScript};

/// Produces replayable Trellis model scripts from a generated seed.
pub fn model_script_strategy(max_steps: usize) -> impl Strategy<Value = ModelScript> {
    any::<u64>().prop_map(move |seed| ModelGenerator::new(seed).script(max_steps))
}
