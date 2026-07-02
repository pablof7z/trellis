use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use trellis_core::{DependencyList, Graph, ResourceKey, ResourcePlan};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Command {
    Open(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Output {
    member_count: usize,
}

struct HostComponent {
    graph: Mutex<Graph<Command>>,
}

fn assert_send_sync<T: Send + Sync>() {}

fn set(entries: &[&str]) -> BTreeSet<String> {
    entries.iter().map(|entry| (*entry).to_owned()).collect()
}

#[test]
fn graph_can_live_inside_send_sync_host_component() {
    assert_send_sync::<Graph<Command>>();
    assert_send_sync::<HostComponent>();

    let output_runs = Arc::new(Mutex::new(0_usize));
    let mut graph = Graph::<Command>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("host-owned-session").unwrap();
    let source = tx.input::<BTreeSet<String>>("source-members").unwrap();
    tx.set_input(source, set(&["alice", "bob"])).unwrap();

    let member_count = tx
        .derived::<usize>(
            "member-count",
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(source)?.len()),
        )
        .unwrap();
    let members = tx
        .set_collection(
            "member-resources",
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(source)?.clone()),
        )
        .unwrap();
    tx.set_resource_planner(members, scope, move |ctx| {
        let mut plan = ResourcePlan::new();
        for added in &ctx.diff().added {
            plan.open(
                ResourceKey::new(format!("member:{}", added.value)),
                ctx.scope(),
                Command::Open(added.value.clone()),
            );
        }
        Ok(plan)
    })
    .unwrap();

    let output_runs_for_materialize = Arc::clone(&output_runs);
    tx.materialized_output(
        "member-output",
        scope,
        DependencyList::new([member_count.id()]).unwrap(),
        move |ctx| {
            *output_runs_for_materialize.lock().unwrap() += 1;
            Ok(Output {
                member_count: *ctx.derived(member_count)?,
            })
        },
    )
    .unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert_eq!(result.resource_plan.commands().len(), 2);
    assert_eq!(result.output_frames.len(), 1);
    assert_eq!(*output_runs.lock().unwrap(), 1);

    let component = HostComponent {
        graph: Mutex::new(graph),
    };
    assert_eq!(component.graph.lock().unwrap().revision(), result.revision);
}
