use std::collections::BTreeSet;

use trellis_core::{
    AuditEvent, DependencyList, Graph, OutputFrameKindTrace, ResourceCommandCause,
    ResourceCommandKind, ResourceKey, ResourcePlan, SetDiff,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Command {
    Open(String),
}

fn key(value: &str) -> ResourceKey {
    ResourceKey::new(value.to_owned())
}

fn set(entries: &[&str]) -> BTreeSet<String> {
    entries.iter().map(|value| (*value).to_owned()).collect()
}

fn plan_added(
    ctx: &trellis_core::PlanContext<'_, SetDiff<String>>,
) -> Result<ResourcePlan<Command>, trellis_core::PlanError> {
    let mut plan = ResourcePlan::new();
    for added in &ctx.diff().added {
        plan.open(
            key(&added.value),
            ctx.scope(),
            Command::Open(added.value.clone()),
        );
    }
    Ok(plan)
}

fn plan_added_removed(
    ctx: &trellis_core::PlanContext<'_, SetDiff<String>>,
) -> Result<ResourcePlan<Command>, trellis_core::PlanError> {
    let mut plan = plan_added(ctx)?;
    for removed in &ctx.diff().removed {
        plan.close(key(&removed.value), ctx.scope());
    }
    Ok(plan)
}

#[test]
fn audit_explains_node_resource_and_output_changes() {
    let mut graph = Graph::<Command, BTreeSet<String>>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let source = tx.input::<BTreeSet<String>>("source").unwrap();
    tx.set_input(source, set(&["a", "b"])).unwrap();
    let collection = tx
        .set_collection(
            "resources",
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(source)?.clone()),
        )
        .unwrap();
    tx.set_resource_planner(collection, scope, plan_added_removed)
        .unwrap();
    let output = tx
        .materialized_output(
            "output",
            scope,
            DependencyList::new([collection.id()]).unwrap(),
            move |ctx| Ok(ctx.set_collection(collection)?.clone()),
        )
        .unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert_eq!(result.trace().resource_commands.len(), 2);
    assert_eq!(
        graph.dependency_path(source.id(), collection.id()),
        Some(vec![source.id(), collection.id()])
    );

    let changed = graph.why_changed(collection).unwrap();
    assert_eq!(changed.node, collection.id());
    assert_eq!(changed.input_causes, vec![source.id()]);
    assert_eq!(
        changed.dependency_paths,
        vec![vec![source.id(), collection.id()]]
    );

    let resource = graph.why_resource_command(&key("a")).unwrap();
    assert_eq!(resource.scope, scope);
    assert_eq!(resource.kind, ResourceCommandKind::Open);
    assert_eq!(
        resource.cause,
        ResourceCommandCause::Planner {
            collection: collection.id()
        }
    );
    assert_eq!(resource.collection_diffs, vec![collection.id()]);
    assert_eq!(resource.input_causes, vec![source.id()]);

    let frame = graph.why_output_frame(output.key()).unwrap();
    assert_eq!(frame.scope, scope);
    assert_eq!(frame.kind, OutputFrameKindTrace::Baseline);
    assert_eq!(frame.dependencies, vec![collection.id()]);
    assert_eq!(frame.changed_dependencies, vec![collection.id()]);

    assert!(
        graph
            .audit_log()
            .iter()
            .any(|entry| { entry.event == AuditEvent::CollectionChanged(collection.id()) })
    );

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(source, set(&["a"])).unwrap();
    tx.commit().unwrap();
    drop(tx);

    let close = graph.why_resource_command(&key("b")).unwrap();
    assert_eq!(close.kind, ResourceCommandKind::Close);
    assert_eq!(
        close.cause,
        ResourceCommandCause::Planner {
            collection: collection.id()
        }
    );
    assert_eq!(close.collection_diffs, vec![collection.id()]);
    assert_eq!(
        close.dependency_paths,
        vec![vec![source.id(), collection.id()]]
    );
}

#[test]
fn scope_resource_inventory_is_deterministic_and_empty_after_close() {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let source = tx.input::<BTreeSet<String>>("source").unwrap();
    tx.set_input(source, set(&["b", "a"])).unwrap();
    let collection = tx
        .set_collection(
            "resources",
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(source)?.clone()),
        )
        .unwrap();
    tx.set_resource_planner(collection, scope, plan_added_removed)
        .unwrap();
    tx.commit().unwrap();
    drop(tx);

    let inventory = graph.scope_resource_inventory(scope).unwrap();
    assert_eq!(inventory.resources, vec![key("a"), key("b")]);

    let mut tx = graph.begin_transaction().unwrap();
    tx.close_scope(scope).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert_eq!(result.resource_plan.commands().len(), 2);
    assert_eq!(
        graph.why_resource_command(&key("a")).unwrap().cause,
        ResourceCommandCause::ScopeClosed { scope }
    );
    assert!(
        graph
            .scope_resource_inventory(scope)
            .unwrap()
            .resources
            .is_empty()
    );
    assert!(graph.orphan_resources().is_empty());
}

#[test]
fn audit_uses_exact_planner_collection_for_resource_commands() {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let first_input = tx.input::<BTreeSet<String>>("first").unwrap();
    let second_input = tx.input::<BTreeSet<String>>("second").unwrap();
    tx.set_input(first_input, set(&["a"])).unwrap();
    tx.set_input(second_input, set(&["b"])).unwrap();
    let first = tx
        .set_collection(
            "first-set",
            DependencyList::new([first_input.id()]).unwrap(),
            move |ctx| Ok(ctx.input(first_input)?.clone()),
        )
        .unwrap();
    let second = tx
        .set_collection(
            "second-set",
            DependencyList::new([second_input.id()]).unwrap(),
            move |ctx| Ok(ctx.input(second_input)?.clone()),
        )
        .unwrap();
    for collection in [first, second] {
        tx.set_resource_planner(collection, scope, plan_added)
            .unwrap();
    }
    tx.commit().unwrap();
    drop(tx);

    assert_eq!(
        graph.why_resource_command(&key("a")).unwrap().cause,
        ResourceCommandCause::Planner {
            collection: first.id()
        }
    );
    assert_eq!(
        graph.why_resource_command(&key("b")).unwrap().input_causes,
        vec![second_input.id()]
    );
}

#[test]
fn late_planner_registration_explains_existing_collection_members() {
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let source = tx.input::<BTreeSet<String>>("source").unwrap();
    tx.set_input(source, set(&["late"])).unwrap();
    let collection = tx
        .set_collection(
            "resources",
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(source)?.clone()),
        )
        .unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_resource_planner(collection, scope, plan_added)
        .unwrap();
    tx.commit().unwrap();
    drop(tx);

    let explanation = graph.why_resource_command(&key("late")).unwrap();
    assert_eq!(
        explanation.cause,
        ResourceCommandCause::Planner {
            collection: collection.id()
        }
    );
    assert_eq!(explanation.collection_diffs, vec![collection.id()]);
}

#[test]
fn audit_debug_dump_is_deterministic() {
    fn build_dump() -> String {
        let mut graph = Graph::<Command>::new_with_command_type();
        let mut tx = graph.begin_transaction().unwrap();
        let scope = tx.create_scope("scope").unwrap();
        let source = tx.input::<BTreeSet<String>>("source").unwrap();
        tx.set_input(source, set(&["x"])).unwrap();
        let collection = tx
            .set_collection(
                "resources",
                DependencyList::new([source.id()]).unwrap(),
                move |ctx| Ok(ctx.input(source)?.clone()),
            )
            .unwrap();
        tx.set_resource_planner(collection, scope, plan_added)
            .unwrap();
        tx.commit().unwrap();
        drop(tx);
        graph.debug_dump()
    }

    let first = build_dump();
    let second = build_dump();

    assert_eq!(first, second);
    assert!(first.contains("Resources:"));
    assert!(first.contains("Audit:"));
}
