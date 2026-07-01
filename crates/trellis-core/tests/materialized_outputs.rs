use trellis_core::{
    ClearReason, DependencyList, Graph, OutputFrameKind, OutputOptions, RebaselineReason,
    TransactionOptions,
};

fn apply_frame(state: &mut Option<String>, kind: &OutputFrameKind<String>) {
    match kind {
        OutputFrameKind::Baseline(value)
        | OutputFrameKind::Delta(value)
        | OutputFrameKind::Rebaseline(value, _) => {
            *state = Some(value.clone());
        }
        OutputFrameKind::Clear(_) => {
            *state = None;
        }
    }
}

#[test]
fn input_change_emits_baseline_and_delta_with_revisions() {
    let mut graph = Graph::<(), String>::new_with_output_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let source = tx.input::<String>("source").unwrap();
    tx.set_input(source, "one".to_owned()).unwrap();
    let output = tx
        .materialized_output(
            "output",
            scope,
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(source)?.clone()),
        )
        .unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert_eq!(result.output_frames.len(), 1);
    assert_eq!(result.output_frames[0].output_key, output.key());
    assert_eq!(result.output_frames[0].scope, scope);
    assert_eq!(
        result.output_frames[0].transaction_id,
        result.transaction_id
    );
    assert_eq!(result.output_frames[0].revision, result.revision);
    assert_eq!(
        result.output_frames[0].kind,
        OutputFrameKind::Baseline("one".to_owned())
    );

    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(source, "two".to_owned()).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert_eq!(result.revision.get(), 2);
    assert_eq!(
        result.output_frames[0].kind,
        OutputFrameKind::Delta("two".to_owned())
    );
}

#[test]
fn equal_output_emits_no_delta_unless_configured() {
    let mut graph = Graph::<(), String>::new_with_output_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let source = tx.input::<String>("source").unwrap();
    tx.set_input(source, "same".to_owned()).unwrap();
    tx.materialized_output(
        "default",
        scope,
        DependencyList::new([source.id()]).unwrap(),
        move |ctx| Ok(ctx.input(source)?.clone()),
    )
    .unwrap();
    tx.materialized_output_with_options(
        "emit-equal",
        scope,
        DependencyList::new([source.id()]).unwrap(),
        OutputOptions { emit_equal: true },
        move |ctx| Ok(ctx.input(source)?.clone()),
    )
    .unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph
        .begin_transaction_with_options(TransactionOptions {
            skip_equal_inputs: false,
        })
        .unwrap();
    tx.set_input(source, "same".to_owned()).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert_eq!(result.output_frames.len(), 1);
    assert_eq!(
        result.output_frames[0].kind,
        OutputFrameKind::Delta("same".to_owned())
    );
}

#[test]
fn scope_close_emits_clear_frame() {
    let mut graph = Graph::<(), String>::new_with_output_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let source = tx.input::<String>("source").unwrap();
    tx.set_input(source, "visible".to_owned()).unwrap();
    let output = tx
        .materialized_output(
            "output",
            scope,
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(source)?.clone()),
        )
        .unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.close_scope(scope).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert_eq!(result.output_frames.len(), 1);
    assert_eq!(result.output_frames[0].output_key, output.key());
    assert_eq!(
        result.output_frames[0].kind,
        OutputFrameKind::Clear(ClearReason::ScopeClosed)
    );
    assert!(graph.output_meta(output.key()).is_none());
}

#[test]
fn rebaseline_emits_coherent_current_state() {
    let mut graph = Graph::<(), String>::new_with_output_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let source = tx.input::<String>("source").unwrap();
    tx.set_input(source, "current".to_owned()).unwrap();
    let output = tx
        .materialized_output(
            "output",
            scope,
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(source)?.clone()),
        )
        .unwrap();
    tx.commit().unwrap();
    drop(tx);

    let mut tx = graph.begin_transaction().unwrap();
    tx.rebaseline_output(output).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    assert_eq!(
        result.output_frames[0].kind,
        OutputFrameKind::Rebaseline("current".to_owned(), RebaselineReason::Requested)
    );
}

#[test]
fn deltas_reconstruct_final_baseline_state() {
    let mut graph = Graph::<(), String>::new_with_output_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let source = tx.input::<String>("source").unwrap();
    tx.set_input(source, "one".to_owned()).unwrap();
    let output = tx
        .materialized_output(
            "output",
            scope,
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(ctx.input(source)?.clone()),
        )
        .unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    let mut consumer = None;
    for frame in &result.output_frames {
        apply_frame(&mut consumer, &frame.kind);
    }

    for value in ["two", "three"] {
        let mut tx = graph.begin_transaction().unwrap();
        tx.set_input(source, value.to_owned()).unwrap();
        let result = tx.commit().unwrap();
        drop(tx);
        for frame in &result.output_frames {
            apply_frame(&mut consumer, &frame.kind);
        }
    }

    let mut tx = graph.begin_transaction().unwrap();
    tx.rebaseline_output(output).unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    let OutputFrameKind::Rebaseline(final_state, _) = &result.output_frames[0].kind else {
        panic!("expected rebaseline");
    };
    assert_eq!(consumer.as_ref(), Some(final_state));
}

#[test]
fn output_frame_ordering_is_deterministic_by_key() {
    let mut graph = Graph::<(), String>::new_with_output_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("scope").unwrap();
    let source = tx.input::<String>("source").unwrap();
    tx.set_input(source, "value".to_owned()).unwrap();
    let first = tx
        .materialized_output(
            "first",
            scope,
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(format!("first:{}", ctx.input(source)?)),
        )
        .unwrap();
    let second = tx
        .materialized_output(
            "second",
            scope,
            DependencyList::new([source.id()]).unwrap(),
            move |ctx| Ok(format!("second:{}", ctx.input(source)?)),
        )
        .unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    let keys: Vec<_> = result
        .output_frames
        .iter()
        .map(|frame| frame.output_key)
        .collect();
    assert_eq!(keys, vec![first.key(), second.key()]);
}
