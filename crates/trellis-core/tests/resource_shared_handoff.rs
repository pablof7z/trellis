use std::collections::{BTreeMap, BTreeSet};

use trellis_core::{
    CollectionNode, DependencyList, Graph, GraphError, InputNode, ResourceCommand, ResourceKey,
    ResourcePlan, ScopeId,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Command {
    Open(String),
    Replace(String),
    Refresh(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PlannerOrder {
    AThenB,
    BThenA,
}

struct HandoffGraph {
    graph: Graph<Command>,
    a_source: InputNode<BTreeMap<String, String>>,
    b_source: InputNode<BTreeMap<String, String>>,
    a_scope: ScopeId,
    b_scope: ScopeId,
}

fn key(value: &str) -> ResourceKey {
    ResourceKey::new(value.to_owned())
}

fn desired(payload: &str) -> BTreeMap<String, String> {
    BTreeMap::from([("shared".to_owned(), payload.to_owned())])
}

fn empty() -> BTreeMap<String, String> {
    BTreeMap::new()
}

fn register_planner(
    tx: &mut trellis_core::Transaction<'_, Command>,
    collection: CollectionNode<String, String>,
    scope: ScopeId,
) {
    tx.map_resource_planner(collection, scope, move |ctx| {
        let mut plan = ResourcePlan::new();
        for added in &ctx.diff().added {
            let (member_key, payload) = &added.value;
            plan.open(key(member_key), ctx.scope(), Command::Open(payload.clone()));
        }
        for updated in &ctx.diff().updated {
            plan.replace(
                key(&updated.key),
                ctx.scope(),
                Command::Replace(updated.current.clone()),
            );
        }
        for removed in &ctx.diff().removed {
            plan.close(key(&removed.value.0), ctx.scope());
        }
        Ok(plan)
    })
    .unwrap();
}

fn build_handoff_graph(order: PlannerOrder) -> HandoffGraph {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let a_scope = tx.create_scope("a").unwrap();
    let b_scope = tx.create_scope("b").unwrap();
    let a_source = tx.input::<BTreeMap<String, String>>("a-source").unwrap();
    let b_source = tx.input::<BTreeMap<String, String>>("b-source").unwrap();
    tx.set_input(a_source, desired("p1")).unwrap();
    tx.set_input(b_source, empty()).unwrap();
    let a_collection = tx
        .collection(
            "a-resources",
            DependencyList::new([a_source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(a_source)?.clone()),
        )
        .unwrap();
    let b_collection = tx
        .collection(
            "b-resources",
            DependencyList::new([b_source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(b_source)?.clone()),
        )
        .unwrap();

    match order {
        PlannerOrder::AThenB => {
            register_planner(&mut tx, a_collection, a_scope);
            register_planner(&mut tx, b_collection, b_scope);
        }
        PlannerOrder::BThenA => {
            register_planner(&mut tx, b_collection, b_scope);
            register_planner(&mut tx, a_collection, a_scope);
        }
    }

    let initial = tx.commit().unwrap();
    assert_eq!(
        initial.resource_plan.commands(),
        &[ResourceCommand::Open {
            key: key("shared"),
            scope: a_scope,
            command: Command::Open("p1".to_owned()),
        }]
    );
    drop(tx);

    HandoffGraph {
        graph,
        a_source,
        b_source,
        a_scope,
        b_scope,
    }
}

fn commit_handoff(
    case: &mut HandoffGraph,
    joining_payload: &str,
) -> Result<Vec<ResourceCommand<Command>>, GraphError> {
    let mut tx = case.graph.begin_transaction().unwrap();
    tx.set_input(case.a_source, empty()).unwrap();
    tx.set_input(case.b_source, desired(joining_payload))
        .unwrap();
    let result = tx.commit()?;
    Ok(result.resource_plan.into_commands())
}

#[test]
fn same_payload_handoff_does_not_emit_host_churn_in_any_planner_order() {
    for order in [PlannerOrder::AThenB, PlannerOrder::BThenA] {
        let mut case = build_handoff_graph(order);

        let commands = commit_handoff(&mut case, "p1").unwrap();

        assert_eq!(commands, []);
        assert_eq!(
            case.graph.resource_owners(&key("shared")).cloned(),
            Some(BTreeSet::from([case.b_scope]))
        );
    }
}

#[test]
fn changed_payload_handoff_is_canonical_in_any_planner_order() {
    for order in [PlannerOrder::AThenB, PlannerOrder::BThenA] {
        let mut case = build_handoff_graph(order);

        let commands = commit_handoff(&mut case, "p2").unwrap();

        assert_eq!(
            commands,
            [
                ResourceCommand::Close {
                    key: key("shared"),
                    scope: case.a_scope,
                },
                ResourceCommand::Open {
                    key: key("shared"),
                    scope: case.b_scope,
                    command: Command::Open("p2".to_owned()),
                },
            ]
        );
        assert_eq!(
            case.graph.resource_owners(&key("shared")).cloned(),
            Some(BTreeSet::from([case.b_scope]))
        );
    }
}

#[test]
fn shared_owner_replace_requires_payload_agreement() {
    let mut case = build_handoff_graph(PlannerOrder::AThenB);
    let mut tx = case.graph.begin_transaction().unwrap();
    tx.set_input(case.b_source, desired("p1")).unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = case.graph.begin_transaction().unwrap();
    tx.set_input(case.a_source, desired("p2")).unwrap();
    assert!(matches!(
        tx.commit(),
        Err(GraphError::ResourcePayloadConflict(_))
    ));
    drop(tx);

    assert!(case.graph.full_recompute_check().is_ok());
}

#[test]
fn shared_owner_refresh_requires_payload_agreement() {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let a_scope = tx.create_scope("a").unwrap();
    let b_scope = tx.create_scope("b").unwrap();
    let a_source = tx.input::<BTreeMap<String, String>>("a-source").unwrap();
    let b_source = tx.input::<BTreeMap<String, String>>("b-source").unwrap();
    tx.set_input(a_source, desired("p1")).unwrap();
    tx.set_input(b_source, desired("p1")).unwrap();
    let a_collection = tx
        .collection(
            "a-resources",
            DependencyList::new([a_source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(a_source)?.clone()),
        )
        .unwrap();
    let b_collection = tx
        .collection(
            "b-resources",
            DependencyList::new([b_source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(b_source)?.clone()),
        )
        .unwrap();
    tx.map_resource_planner(a_collection, a_scope, move |ctx| {
        let mut plan = ResourcePlan::new();
        for added in &ctx.diff().added {
            plan.open(
                key(&added.value.0),
                ctx.scope(),
                Command::Open(added.value.1.clone()),
            );
        }
        for updated in &ctx.diff().updated {
            plan.refresh(
                key(&updated.key),
                ctx.scope(),
                Command::Refresh(updated.current.clone()),
            );
        }
        Ok(plan)
    })
    .unwrap();
    register_planner(&mut tx, b_collection, b_scope);
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(a_source, desired("p2")).unwrap();
    assert!(matches!(
        tx.commit(),
        Err(GraphError::ResourcePayloadConflict(_))
    ));
    drop(tx);

    assert!(graph.full_recompute_check().is_ok());
}
