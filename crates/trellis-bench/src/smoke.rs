use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use crate::smoke_collections;
use trellis_core::{DependencyList, Graph};

pub struct Bench {
    pub name: &'static str,
    pub iterations: usize,
    pub run: fn() -> usize,
}

pub const BENCHES: &[Bench] = &[
    Bench {
        name: "no_op_transaction",
        iterations: 100,
        run: no_op_transaction,
    },
    Bench {
        name: "deep_graph_propagation",
        iterations: 20,
        run: deep_graph_propagation,
    },
    Bench {
        name: "wide_graph_propagation",
        iterations: 20,
        run: wide_graph_propagation,
    },
    Bench {
        name: "input_change_no_downstream_change",
        iterations: 100,
        run: no_downstream_change,
    },
    Bench {
        name: "input_change_with_recompute",
        iterations: 100,
        run: downstream_recompute,
    },
    Bench {
        name: "large_set_growth",
        iterations: 5,
        run: smoke_collections::large_set_growth,
    },
    Bench {
        name: "large_set_shrink",
        iterations: 5,
        run: smoke_collections::large_set_shrink,
    },
    Bench {
        name: "large_map_update",
        iterations: 5,
        run: smoke_collections::large_map_update,
    },
    Bench {
        name: "scope_close_many_resources",
        iterations: 5,
        run: smoke_collections::scope_close_many_resources,
    },
    Bench {
        name: "shared_resource_many_owners",
        iterations: 3,
        run: smoke_collections::shared_resource_many_owners,
    },
    Bench {
        name: "output_baseline_then_delta",
        iterations: 10,
        run: smoke_collections::output_baseline_then_delta,
    },
    Bench {
        name: "full_recompute_oracle",
        iterations: 5,
        run: smoke_collections::full_recompute_oracle,
    },
    Bench {
        name: "trace_replay_compare",
        iterations: 10,
        run: smoke_collections::trace_replay_compare,
    },
];

fn no_op_transaction() -> usize {
    let mut graph = Graph::<(), ()>::new();
    let mut tx = graph.begin_transaction().unwrap();
    tx.commit().unwrap().phase_trace.len()
}

fn deep_graph_propagation() -> usize {
    let mut graph = Graph::<(), ()>::new();
    let mut tx = graph.begin_transaction().unwrap();
    let input = tx.input::<u32>("input").unwrap();
    tx.set_input(input, 1).unwrap();
    let mut previous = tx
        .derived(
            "derived-0",
            DependencyList::new([input.id()]).unwrap(),
            move |ctx| Ok(*ctx.input(input)? + 1),
        )
        .unwrap();
    for index in 1..64 {
        let dependency = previous;
        previous = tx
            .derived(
                format!("derived-{index}"),
                DependencyList::new([dependency.id()]).unwrap(),
                move |ctx| Ok(*ctx.derived(dependency)? + 1),
            )
            .unwrap();
    }
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(input, 2).unwrap();
    tx.commit().unwrap().changed_derived_nodes.len()
}

fn wide_graph_propagation() -> usize {
    let mut graph = Graph::<(), ()>::new();
    let mut tx = graph.begin_transaction().unwrap();
    let input = tx.input::<u32>("input").unwrap();
    tx.set_input(input, 1).unwrap();
    for index in 0..64 {
        tx.derived(
            format!("derived-{index}"),
            DependencyList::new([input.id()]).unwrap(),
            move |ctx| Ok(*ctx.input(input)? + index),
        )
        .unwrap();
    }
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(input, 2).unwrap();
    tx.commit().unwrap().changed_derived_nodes.len()
}

fn no_downstream_change() -> usize {
    let mut graph = Graph::<(), ()>::new();
    let mut tx = graph.begin_transaction().unwrap();
    let input = tx.input::<u32>("input").unwrap();
    tx.set_input(input, 1).unwrap();
    let parity = tx
        .derived(
            "parity",
            DependencyList::new([input.id()]).unwrap(),
            move |ctx| Ok(ctx.input(input)? % 2),
        )
        .unwrap();
    let runs = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&runs);
    tx.derived(
        "downstream",
        DependencyList::new([parity.id()]).unwrap(),
        move |ctx| {
            counter.fetch_add(1, Ordering::Relaxed);
            Ok(*ctx.derived(parity)?)
        },
    )
    .unwrap();
    tx.commit().unwrap();
    drop(tx);

    let previous_runs = runs.load(Ordering::Relaxed);
    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(input, 3).unwrap();
    let result = tx.commit().unwrap();
    assert_eq!(runs.load(Ordering::Relaxed), previous_runs);
    result.changed_derived_nodes.len()
}

fn downstream_recompute() -> usize {
    let mut graph = Graph::<(), ()>::new();
    let mut tx = graph.begin_transaction().unwrap();
    let input = tx.input::<u32>("input").unwrap();
    tx.set_input(input, 1).unwrap();
    tx.derived(
        "double",
        DependencyList::new([input.id()]).unwrap(),
        move |ctx| Ok(ctx.input(input)? * 2),
    )
    .unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(input, 2).unwrap();
    tx.commit().unwrap().changed_derived_nodes.len()
}
