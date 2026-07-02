use std::collections::BTreeSet;

use trellis_core::{
    DependencyList, Graph, InputNode, MaterializedOutput, ScopeId, TransactionTrace,
    testing::{ModelScript, ModelStep},
};

struct ScalarTarget {
    graph: Graph<()>,
    source: InputNode<BTreeSet<u8>>,
    output: MaterializedOutput<usize>,
    scope: ScopeId,
}

pub(crate) fn run_scalar_chain_script(script: &ModelScript) -> Vec<TransactionTrace> {
    let (mut target, initial) = build_scalar_graph();
    let mut traces = vec![TransactionTrace::from_result(&initial)];
    let mut scope_live = true;
    let mut output_live = true;
    target.graph.assert_incremental_equals_full().unwrap();

    for step in &script.steps {
        let result = match step {
            ModelStep::SetMembers(next) => set_scalar_source(&mut target, next.clone()),
            ModelStep::RebaselineOutput if output_live => rebaseline_scalar_output(&mut target),
            ModelStep::ClosePrimaryScope if scope_live => {
                scope_live = false;
                output_live = false;
                close_scalar_scope(&mut target)
            }
            ModelStep::RebaselineOutput | ModelStep::ClosePrimaryScope => {
                commit_scalar_noop(&mut target)
            }
        };
        target.graph.assert_incremental_equals_full().unwrap();
        traces.push(TransactionTrace::from_result(&result));
    }
    traces
}

fn build_scalar_graph() -> (ScalarTarget, trellis_core::TransactionResult<()>) {
    let mut graph = Graph::<()>::new();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let source = tx.input::<BTreeSet<u8>>("source").unwrap();
    tx.set_input(source, BTreeSet::new()).unwrap();
    let count = tx
        .derived(
            "count",
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(source)?.len()),
        )
        .unwrap();
    let output = tx
        .materialized_output(
            "count-output",
            scope,
            DependencyList::new([count.id()]).unwrap(),
            move |ctx| Ok(*ctx.derived(count)?),
        )
        .unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    (
        ScalarTarget {
            graph,
            source,
            output,
            scope,
        },
        result,
    )
}

fn set_scalar_source(
    target: &mut ScalarTarget,
    values: BTreeSet<u8>,
) -> trellis_core::TransactionResult<()> {
    let mut tx = target.graph.begin_transaction().unwrap();
    tx.set_input(target.source, values).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);
    target.graph.assert_incremental_equals_full().unwrap();
    result
}

fn rebaseline_scalar_output(
    target: &mut ScalarTarget,
) -> trellis_core::TransactionResult<()> {
    let mut tx = target.graph.begin_transaction().unwrap();
    tx.rebaseline_output(target.output.clone()).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);
    target.graph.assert_incremental_equals_full().unwrap();
    result
}

fn close_scalar_scope(target: &mut ScalarTarget) -> trellis_core::TransactionResult<()> {
    let mut tx = target.graph.begin_transaction().unwrap();
    tx.close_scope(target.scope).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);
    target.graph.assert_incremental_equals_full().unwrap();
    result
}

fn commit_scalar_noop(target: &mut ScalarTarget) -> trellis_core::TransactionResult<()> {
    let mut tx = target.graph.begin_transaction().unwrap();
    let result = tx.commit().unwrap();
    drop(tx);
    target.graph.assert_incremental_equals_full().unwrap();
    result
}
