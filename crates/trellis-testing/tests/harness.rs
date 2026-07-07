use std::collections::BTreeSet;

use trellis_core::{
    DependencyList, Graph, OutputFrameKindTrace, OutputFrameTrace, ResourceCommandKind,
    ResourceCommandTrace, ResourceKey, ResourcePlan, ResourceTransitionPolicy, Revision, ScopeId,
    TransactionId,
};
use trellis_testing::{ScenarioTarget, TraceRedactor, TransactionScript, TrellisHarness};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Command {
    Open(u8),
}

struct TestGraph {
    graph: Graph<Command>,
    source: trellis_core::InputNode<BTreeSet<u8>>,
    output: trellis_core::MaterializedOutput<BTreeSet<u8>>,
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

fn members(values: &[u8]) -> BTreeSet<u8> {
    values.iter().copied().collect()
}

fn key(value: u8) -> ResourceKey {
    ResourceKey::new(format!("resource:{value}"))
}

fn command_trace(value: u8, scope: ScopeId, kind: ResourceCommandKind) -> ResourceCommandTrace {
    ResourceCommandTrace {
        key: key(value),
        scope,
        kind,
        transition_policy: ResourceTransitionPolicy::from_kind(kind),
    }
}

fn build_target() -> TestGraph {
    build_target_with_payload_adjustments(0, 0)
}

fn build_target_with_payload_adjustments(command_offset: u8, output_offset: u8) -> TestGraph {
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
            plan.open(
                key(added.value),
                ctx.scope(),
                Command::Open(added.value + command_offset),
            );
        }
        for removed in &ctx.diff().removed {
            plan.close(key(removed.value), ctx.scope());
        }
        Ok(plan)
    })
    .unwrap();
    let output = tx
        .materialized_output(
            "output",
            scope,
            DependencyList::new([collection.id()]).unwrap(),
            move |ctx| {
                Ok(ctx
                    .set_collection(collection)?
                    .iter()
                    .map(|value| value + output_offset)
                    .collect())
            },
        )
        .unwrap();
    tx.commit().unwrap();
    drop(tx);

    TestGraph {
        graph,
        source,
        output,
        scope,
    }
}

#[test]
fn harness_step_commits_one_transaction_and_checks_structural_expectations() {
    let target = build_target();
    let source = target.source;
    let output_key = target.output.key();
    let scope = target.scope;
    let mut harness = TrellisHarness::from_target(target);

    harness
        .step("open")
        .input(source, members(&[2, 1]))
        .expect_plans([
            command_trace(1, scope, ResourceCommandKind::Open),
            command_trace(2, scope, ResourceCommandKind::Open),
        ])
        .expect_output(OutputFrameTrace {
            output_key,
            scope,
            transaction_id: TransactionId::new(2),
            revision: Revision::new(2),
            kind: OutputFrameKindTrace::Delta,
        })
        .check("full recompute", |target, _| {
            target.graph.assert_incremental_equals_full().is_ok()
        })
        .commit()
        .unwrap();

    assert!(
        harness
            .scenario()
            .step("open")
            .unwrap()
            .trace
            .invariant_results[0]
            .passed
    );
    harness
        .resource_ledger()
        .assert_command_order(&[
            command_trace(1, scope, ResourceCommandKind::Open),
            command_trace(2, scope, ResourceCommandKind::Open),
        ])
        .unwrap();
    harness
        .output_ledger()
        .assert_current_equals(output_key, &members(&[1, 2]))
        .unwrap();
}

struct MaskKeys;

impl TraceRedactor for MaskKeys {
    fn resource_key(&self, _key: &ResourceKey) -> ResourceKey {
        ResourceKey::new("redacted")
    }
}

#[test]
fn transaction_script_replays_and_redacts_snapshot_dumps() {
    let seed = build_target();
    let source = seed.source;
    drop(seed);

    let mut script = TransactionScript::new();
    script
        .step("open secret resources")
        .input(source, members(&[1, 2]))
        .commit();
    script.step("shrink").input(source, members(&[1])).commit();

    let first = TrellisHarness::replay(build_target, &script).unwrap();
    let second = TrellisHarness::replay(build_target, &script).unwrap();

    first.assert_replay_matches(&second).unwrap();
    assert_eq!(
        first.scenario().resource_commands(),
        second.scenario().resource_commands()
    );
    assert_eq!(
        first.scenario().output_frames(),
        second.scenario().output_frames()
    );

    let trace_dump = first.scenario().to_redacted_debug_string(&MaskKeys);
    let resource_dump = first.resource_ledger().to_redacted_debug_string(&MaskKeys);
    let output_dump = first
        .output_ledger()
        .to_redacted_debug_string(|_| "<state>".to_owned());

    assert!(trace_dump.contains("redacted"));
    assert!(!trace_dump.contains("resource:1"));
    assert!(resource_dump.contains("redacted"));
    assert!(!resource_dump.contains("resource:1"));
    assert!(output_dump.contains("<state>"));
    assert!(!output_dump.contains("{1"));
}

#[test]
fn replay_detects_resource_command_payload_drift() {
    let seed = build_target();
    let source = seed.source;
    drop(seed);

    let mut script = TransactionScript::new();
    script.step("open").input(source, members(&[1])).commit();

    let expected = TrellisHarness::replay(build_target, &script).unwrap();
    let actual =
        TrellisHarness::replay(|| build_target_with_payload_adjustments(10, 0), &script).unwrap();

    let error = expected.assert_replay_matches(&actual).unwrap_err();
    assert!(matches!(
        error,
        trellis_testing::ScenarioError::ReplayLedgerMismatch {
            field: "resource_command_records",
            ..
        }
    ));
}

#[test]
fn replay_detects_output_payload_drift() {
    let seed = build_target();
    let source = seed.source;
    drop(seed);

    let mut script = TransactionScript::new();
    script.step("open").input(source, members(&[1])).commit();

    let expected = TrellisHarness::replay(build_target, &script).unwrap();
    let actual =
        TrellisHarness::replay(|| build_target_with_payload_adjustments(0, 10), &script).unwrap();

    let error = expected.assert_replay_matches(&actual).unwrap_err();
    assert!(matches!(
        error,
        trellis_testing::ScenarioError::ReplayLedgerMismatch {
            field: "output_frame_records",
            ..
        }
    ));
}

#[test]
fn harness_rejects_duplicate_step_names_before_commit() {
    let target = build_target();
    let source = target.source;
    let mut harness = TrellisHarness::from_target(target);

    harness
        .step("open")
        .input(source, members(&[1]))
        .commit()
        .unwrap();
    let error = match harness.step("open").input(source, members(&[2])).commit() {
        Ok(_) => panic!("duplicate step unexpectedly committed"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        trellis_testing::ScenarioError::DuplicateStep { step } if step == "open"
    ));
    assert_eq!(harness.scenario().steps().len(), 1);
}
