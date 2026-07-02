#![cfg(feature = "proptest")]

use proptest::prelude::*;
use trellis_testing::proptest::{
    CollectionChange, InputChange, OutputChange, ResourceStatusChange, ScopeChange,
    TransactionChange, canonical_input_change, collection_change, model_script_replay_debug,
    model_script_strategy, model_sequence_strategy, output_rebaseline, resource_status_change,
    scope_change, transaction_change,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum AppStep {
    Input(InputChange<u8>),
    Scope(ScopeChange<&'static str>),
    Collection(CollectionChange<u8, u8>),
    Resource(ResourceStatusChange<u8, &'static str>),
    Transaction(TransactionChange<&'static str>),
    Output(OutputChange<&'static str>),
}

fn app_step_strategy() -> impl Strategy<Value = AppStep> {
    prop_oneof![
        canonical_input_change(0u8..4).prop_map(AppStep::Input),
        scope_change(Just("screen"), Just("screen")).prop_map(AppStep::Scope),
        collection_change((0u8..4, 0u8..4), 0u8..4, (0u8..4, 0u8..4)).prop_map(AppStep::Collection),
        resource_status_change(0u8..4, (0u8..4, Just("failed"))).prop_map(AppStep::Resource),
        transaction_change(Just("retryable")).prop_map(AppStep::Transaction),
        output_rebaseline(Just("rows")).prop_map(AppStep::Output),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig {
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn generic_sequence_helpers_are_debuggable(seq in model_sequence_strategy(app_step_strategy(), 1..=8)) {
        prop_assert!(!seq.is_empty());
        prop_assert!(seq.len() <= 8);
        prop_assert!(seq.to_replay_debug_string().contains("0:"));
    }

    #[test]
    fn core_model_script_strategy_generates_shrinkable_steps(script in model_script_strategy(6)) {
        prop_assert!(script.steps.len() <= 6);
        prop_assert!(model_script_replay_debug(&script).contains("topology:"));
    }
}
