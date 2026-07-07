use std::collections::BTreeSet;

use trellis_core::{
    DependencyList, Graph, HostResourceOutcome, InputNode, ResourceCommand, ResourceKey,
    ResourcePlan, Revision, ScopeId, TransactionId, TransactionResult,
};
use trellis_testing::{
    ConformanceCheckResult, ConformanceLevel, HostConformanceError, HostConformanceLedger,
    HostStatusEvent, conformance,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Command {
    Open(u8),
}

struct TestGraph {
    graph: Graph<Command>,
    source: InputNode<BTreeSet<u8>>,
    scope: ScopeId,
}

fn members(values: &[u8]) -> BTreeSet<u8> {
    values.iter().copied().collect()
}

fn key(value: u8) -> ResourceKey {
    ResourceKey::new(format!("host:{value}"))
}

fn build_graph() -> TestGraph {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("host-scope").unwrap();
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

fn preview_source(target: &mut TestGraph, values: &[u8]) -> TransactionResult<Command> {
    let mut tx = target.graph.begin_transaction().unwrap();
    tx.set_input(target.source, members(values)).unwrap();
    tx.preview().unwrap()
}

fn commit_source(target: &mut TestGraph, values: &[u8]) -> TransactionResult<Command> {
    let mut tx = target.graph.begin_transaction().unwrap();
    tx.set_input(target.source, members(values)).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);
    result
}

#[test]
fn host_conformance_records_preview_commit_effect_and_status() {
    let mut target = build_graph();
    let preview = preview_source(&mut target, &[1]);
    let commit = commit_source(&mut target, &[1]);

    let mut ledger = HostConformanceLedger::new();
    ledger.declare_executor("resource-executor");
    ledger.record_effect_site("resource-executor");
    ledger.record_preview("open one", &preview);
    ledger.record_commit("open one", &commit);
    ledger.record_effects_from_commit("open one", "resource-executor", &commit);
    ledger.record_status(HostStatusEvent {
        resource_key: key(1),
        scope: target.scope,
        command_revision: commit.revision,
        status_revision: Revision::new(1),
        status: HostResourceOutcome::Open,
    });

    ledger.assert_commits_match_previews().unwrap();
    ledger.assert_effects_match_commits().unwrap();
    ledger.assert_effects_use_declared_executors().unwrap();
    ledger.assert_statuses_follow_effects().unwrap();
    ledger.assert_host_seam_conforms().unwrap();

    let report = conformance()
        .check(
            ConformanceLevel::HostSeam,
            "host effects are previewed before execution",
            move || match ledger.assert_host_seam_conforms() {
                Ok(()) => ConformanceCheckResult::passed(),
                Err(error) => ConformanceCheckResult::failed(format!("{error:#?}")),
            },
        )
        .run()
        .unwrap();
    assert!(report.supports(ConformanceLevel::HostSeam));
}

#[test]
fn host_conformance_rejects_commit_drift_from_preview() {
    let mut target = build_graph();
    let preview = preview_source(&mut target, &[1]);
    let commit = commit_source(&mut target, &[2]);

    let mut ledger = HostConformanceLedger::new();
    ledger.record_preview("open", &preview);
    ledger.record_commit("open", &commit);

    let error = ledger.assert_commits_match_previews().unwrap_err();
    assert!(matches!(
        error,
        HostConformanceError::CommitDiffersFromPreview { step, .. } if step == "open"
    ));
}

#[test]
fn host_conformance_rejects_effect_without_commit() {
    let target = build_graph();
    let mut ledger = HostConformanceLedger::new();
    ledger.declare_executor("resource-executor");
    ledger.record_effect(
        "bypass",
        "resource-executor",
        TransactionId::new(42),
        Revision::new(7),
        ResourceCommand::Open {
            key: key(9),
            scope: target.scope,
            command: Command::Open(9),
        },
    );

    let error = ledger.assert_effects_match_commits().unwrap_err();
    assert!(matches!(
        error,
        HostConformanceError::EffectWithoutCommit { effect }
            if effect.step == "bypass" && effect.executor == "resource-executor"
    ));
}

#[test]
fn host_conformance_rejects_undeclared_effect_site() {
    let mut ledger = HostConformanceLedger::<Command>::new();
    ledger.declare_executor("resource-executor");
    ledger.record_effect_site("direct-socket-write");

    let error = ledger.assert_effects_use_declared_executors().unwrap_err();
    assert!(matches!(
        error,
        HostConformanceError::UndeclaredEffectSite { site } if site == "direct-socket-write"
    ));
}
