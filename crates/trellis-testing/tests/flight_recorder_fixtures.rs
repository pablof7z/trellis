#![cfg(feature = "serde")]

use trellis_testing::{SerializedScenario, TRACE_FORMAT_VERSION};

const TRACES: &[(&str, &str)] = &[
    (
        "normal-session",
        include_str!("../../../demos/flight-recorder/traces/normal-session.json"),
    ),
    (
        "seeded-leak",
        include_str!("../../../demos/flight-recorder/traces/seeded-leak.json"),
    ),
    (
        "teardown-cascade",
        include_str!("../../../demos/flight-recorder/traces/teardown-cascade.json"),
    ),
];

#[test]
fn flight_recorder_demo_traces_use_serialized_scenario_v1() {
    for (name, json) in TRACES {
        let scenario = SerializedScenario::from_json(json)
            .unwrap_or_else(|error| panic!("{name} must deserialize: {error}"));

        assert_eq!(scenario.format_version(), TRACE_FORMAT_VERSION, "{name}");
        assert!(
            !scenario.steps().is_empty(),
            "{name} must include trace steps"
        );
    }
}
