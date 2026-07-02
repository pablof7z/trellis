#![cfg(feature = "serde")]

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use trellis_core::{
    DependencyList, Graph, GraphResult, ResourceCommandKind, ResourceCommandTrace, ResourceKey,
    ResourcePlan, ScopeId, Transaction,
};
use trellis_testing::{
    DataTransactionScript, ScenarioError, ScenarioTarget, SerializedScenario, TRACE_FORMAT_VERSION,
    TrellisHarness,
};

const GOLDEN_SCRIPT: &str = r#"{
  "formatVersion": 1,
  "steps": [
    {
      "name": "open",
      "operations": [
        {
          "type": "set",
          "members": [1, 2]
        }
      ]
    },
    {
      "name": "shrink",
      "operations": [
        {
          "type": "set",
          "members": [1]
        }
      ]
    }
  ]
}"#;

const GOLDEN_TRACE: &str = include_str!("fixtures/serialized_trace_v1.json");

#[derive(Clone, Debug, Eq, PartialEq)]
enum Command {
    Open(u8),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum Operation {
    Set { members: Vec<u8> },
}

struct TestGraph {
    graph: Graph<Command>,
    source: trellis_core::InputNode<BTreeSet<u8>>,
    scope: ScopeId,
}

impl ScenarioTarget<Command> for TestGraph {
    fn graph(&self) -> &Graph<Command> {
        &self.graph
    }

    fn graph_mut(&mut self) -> &mut Graph<Command> {
        &mut self.graph
    }
}

#[test]
fn golden_data_script_replays_after_json_round_trip() {
    let script = DataTransactionScript::<Operation>::from_json(GOLDEN_SCRIPT).unwrap();
    assert_eq!(script.format_version(), TRACE_FORMAT_VERSION);

    let encoded = script.to_json().unwrap();
    let decoded = DataTransactionScript::<Operation>::from_json(&encoded).unwrap();
    let first = replay(&decoded).unwrap();
    let second = replay(&decoded).unwrap();

    first.assert_replay_matches(&second).unwrap();
    first
        .scenario()
        .assert_step_resource_commands(
            "shrink",
            &[command_trace(
                2,
                first.target().scope,
                ResourceCommandKind::Close,
            )],
        )
        .unwrap();
}

#[test]
fn versioned_trace_file_round_trips_to_scenario() {
    let script = DataTransactionScript::<Operation>::from_json(GOLDEN_SCRIPT).unwrap();
    let first = replay(&script).unwrap();
    let trace_file = SerializedScenario::from_scenario(first.scenario());
    let json = trace_file.to_json().unwrap();
    assert_eq!(json, GOLDEN_TRACE.trim_end());

    let decoded = SerializedScenario::from_json(GOLDEN_TRACE).unwrap();
    assert_eq!(decoded.format_version(), TRACE_FORMAT_VERSION);
    decoded.assert_matches_scenario(first.scenario()).unwrap();
    let scenario = decoded.into_scenario().unwrap();

    first.scenario().assert_replay_matches(&scenario).unwrap();
    assert!(json.contains("resource:2"));
}

#[test]
fn resource_key_json_accepts_legacy_strings_and_structured_segments() {
    let legacy: ResourceKey = serde_json::from_str(r#""resource:2""#).unwrap();
    assert_eq!(legacy.segments().collect::<Vec<_>>(), vec!["resource:2"]);
    assert_eq!(serde_json::to_string(&legacy).unwrap(), r#""resource:2""#);

    let structured = ResourceKey::from_segments(["article-feed", "acct/a", "home/main"]);
    let json = serde_json::to_string(&structured).unwrap();
    assert_eq!(json, r#"["article-feed","acct/a","home/main"]"#);
    let decoded: ResourceKey = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, structured);
}

#[test]
fn unsupported_script_version_is_a_graceful_replay_error() {
    let json = GOLDEN_SCRIPT.replace("\"formatVersion\": 1", "\"formatVersion\": 99");
    let script = DataTransactionScript::<Operation>::from_json(&json).unwrap();
    let error = match replay(&script) {
        Ok(_) => panic!("unsupported script version replayed"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        ScenarioError::TraceFormatVersionMismatch {
            expected: TRACE_FORMAT_VERSION,
            actual: 99
        }
    ));
}

#[test]
fn unsupported_trace_file_version_is_a_graceful_error() {
    let script = DataTransactionScript::<Operation>::from_json(GOLDEN_SCRIPT).unwrap();
    let first = replay(&script).unwrap();
    let json = SerializedScenario::from_scenario(first.scenario())
        .to_json()
        .unwrap()
        .replace("\"formatVersion\": 1", "\"formatVersion\": 99");
    let trace_file = SerializedScenario::from_json(&json).unwrap();
    let error = match trace_file.into_scenario() {
        Ok(_) => panic!("unsupported trace version loaded"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        ScenarioError::TraceFormatVersionMismatch {
            expected: TRACE_FORMAT_VERSION,
            actual: 99
        }
    ));
}

fn replay(
    script: &DataTransactionScript<Operation>,
) -> Result<TrellisHarness<TestGraph, Command>, ScenarioError> {
    let seed = build_target();
    let source = seed.source;
    drop(seed);
    TrellisHarness::replay_data(build_target, script, move |operation, tx| {
        apply_operation(source, operation, tx)
    })
}

fn apply_operation(
    source: trellis_core::InputNode<BTreeSet<u8>>,
    operation: &Operation,
    tx: &mut Transaction<'_, Command>,
) -> GraphResult<()> {
    match operation {
        Operation::Set { members } => tx.set_input(source, members.iter().copied().collect()),
    }
}

fn build_target() -> TestGraph {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let source = tx.input::<BTreeSet<u8>>("source").unwrap();
    tx.set_input(source, BTreeSet::new()).unwrap();
    let collection = tx
        .set_collection(
            "demand",
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(source)?.clone()),
        )
        .unwrap();
    tx.set_resource_planner(collection, scope, move |ctx| {
        let mut plan = ResourcePlan::new();
        for added in &ctx.diff().added {
            plan.open(key(added.value), ctx.scope(), Command::Open(added.value));
        }
        for removed in &ctx.diff().removed {
            plan.close(key(removed.value), ctx.scope());
        }
        Ok(plan)
    })
    .unwrap();
    tx.commit().unwrap();
    drop(tx);

    TestGraph {
        graph,
        source,
        scope,
    }
}

fn command_trace(value: u8, scope: ScopeId, kind: ResourceCommandKind) -> ResourceCommandTrace {
    ResourceCommandTrace {
        key: key(value),
        scope,
        kind,
    }
}

fn key(value: u8) -> ResourceKey {
    ResourceKey::new(format!("resource:{value}"))
}
