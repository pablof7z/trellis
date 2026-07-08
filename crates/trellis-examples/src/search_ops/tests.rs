use trellis_core::{OutputFrameKindTrace, ScopeLifecycleKind};

use super::*;

fn open_search() -> (SearchOpsApp, SearchHandle) {
    let mut app = SearchOpsApp::new(sample_catalog());
    let handle = app.open_search(opening_search());
    app.drain_effects();
    app.drain_output(handle);
    app.drain_diagnostic_traces();
    (app, handle)
}

#[test]
fn corpus_change_closes_old_readers_and_opens_new_readers() {
    let (mut app, handle) = open_search();

    app.apply_event(handle, SearchOpsEvent::SelectCorpus("docs".to_owned()));
    let effects = app.drain_effects();
    assert!(
        effects.contains(&SearchEffect::Close(SearchResource::ShardReader {
            corpus: "mail".to_owned(),
            shard: "mail-a".to_owned(),
        }))
    );
    assert!(
        effects.contains(&SearchEffect::Close(SearchResource::ShardReader {
            corpus: "mail".to_owned(),
            shard: "mail-b".to_owned(),
        }))
    );
    assert!(
        effects.contains(&SearchEffect::Open(SearchResource::ShardReader {
            corpus: "docs".to_owned(),
            shard: "docs-a".to_owned(),
        }))
    );
    assert!(
        effects.contains(&SearchEffect::Open(SearchResource::ShardReader {
            corpus: "docs".to_owned(),
            shard: "docs-b".to_owned(),
        }))
    );
    assert!(matches!(
        app.drain_output(handle).first(),
        Some(SearchFrame::Delta(snapshot))
            if snapshot.corpus.as_deref() == Some("docs")
                && snapshot.row_doc_ids().contains("docs-001")
                && snapshot.rows.iter().all(|row| row.corpus == "docs")
    ));
}

#[test]
fn query_change_cancels_ranking_jobs_without_closing_readers() {
    let (mut app, handle) = open_search();

    app.apply_event(handle, SearchOpsEvent::ChangeQuery("budget".to_owned()));
    let effects = app.drain_effects();
    assert!(
        effects.contains(&SearchEffect::Close(SearchResource::RankingJob {
            corpus: "mail".to_owned(),
            shard: "mail-a".to_owned(),
            query_key: "rust:all".to_owned(),
        }))
    );
    assert!(
        effects.contains(&SearchEffect::Open(SearchResource::RankingJob {
            corpus: "mail".to_owned(),
            shard: "mail-a".to_owned(),
            query_key: "budget:all".to_owned(),
        }))
    );
    assert!(!effects.iter().any(|effect| matches!(
        effect,
        SearchEffect::Open(SearchResource::ShardReader { .. })
            | SearchEffect::Close(SearchResource::ShardReader { .. })
    )));
    assert!(matches!(
        app.drain_output(handle).first(),
        Some(SearchFrame::Delta(snapshot))
            if snapshot.query_key.as_deref() == Some("budget:all")
                && snapshot.total_results == 3
    ));
}

#[test]
fn permission_revoke_clears_unauthorized_results() {
    let (mut app, handle) = open_search();
    app.apply_event(handle, SearchOpsEvent::ChangeQuery("budget".to_owned()));
    app.drain_effects();
    app.drain_output(handle);
    app.drain_diagnostic_traces();
    app.apply_event(
        handle,
        SearchOpsEvent::SetWindow(ResultWindow { start: 1, len: 2 }),
    );
    app.drain_effects();
    app.drain_output(handle);
    app.drain_diagnostic_traces();

    app.apply_event(
        handle,
        SearchOpsEvent::RevokeDocumentPermission {
            doc_id: "mail-002".to_owned(),
        },
    );
    let effects = app.drain_effects();
    assert!(
        effects.contains(&SearchEffect::Close(SearchResource::ResultCacheWindow {
            corpus: "mail".to_owned(),
            query_key: "budget:all".to_owned(),
            result_fingerprint: "mail-002|mail-004".to_owned(),
            start: 1,
            len: 2,
        }))
    );
    assert!(matches!(
        app.drain_output(handle).first(),
        Some(SearchFrame::Delta(snapshot))
            if !snapshot.row_doc_ids().contains("mail-002")
                && snapshot.total_results == 2
    ));
}

#[test]
fn page_window_rebaselines_results_without_waking_readers() {
    let (mut app, handle) = open_search();

    app.apply_event(
        handle,
        SearchOpsEvent::SetWindow(ResultWindow { start: 1, len: 2 }),
    );
    let effects = app.drain_effects();
    assert!(
        effects.contains(&SearchEffect::Close(SearchResource::ResultCacheWindow {
            corpus: "mail".to_owned(),
            query_key: "rust:all".to_owned(),
            result_fingerprint: "mail-001|mail-002".to_owned(),
            start: 0,
            len: 2,
        }))
    );
    assert!(
        effects.contains(&SearchEffect::Open(SearchResource::ResultCacheWindow {
            corpus: "mail".to_owned(),
            query_key: "rust:all".to_owned(),
            result_fingerprint: "mail-002|mail-003".to_owned(),
            start: 1,
            len: 2,
        }))
    );
    assert!(!effects.iter().any(|effect| matches!(
        effect,
        SearchEffect::Open(SearchResource::ShardReader { .. })
            | SearchEffect::Close(SearchResource::ShardReader { .. })
    )));
    assert!(matches!(
        app.drain_output(handle).first(),
        Some(SearchFrame::Rebaseline(snapshot))
            if snapshot.row_doc_ids().contains("mail-002")
                && snapshot.row_doc_ids().contains("mail-003")
    ));
}

#[test]
fn close_search_closes_resources_and_clears_results() {
    let (mut app, handle) = open_search();

    app.close(handle);
    let effects = app.drain_effects();
    assert!(
        effects.contains(&SearchEffect::Close(SearchResource::ShardReader {
            corpus: "mail".to_owned(),
            shard: "mail-a".to_owned(),
        }))
    );
    assert!(
        effects.contains(&SearchEffect::Close(SearchResource::RankingJob {
            corpus: "mail".to_owned(),
            shard: "mail-b".to_owned(),
            query_key: "rust:all".to_owned(),
        }))
    );
    assert!(app.drain_output(handle).contains(&SearchFrame::Cleared));
}

#[test]
fn search_lifecycle_trace_uses_showcase_contract() {
    let trace = search_lifecycle_showcase_trace();

    assert_eq!(trace.showcase, "search-ops");
    assert_eq!(trace.script, "search-lifecycle");
    assert_eq!(trace.replay.status, "passed");
    assert_eq!(
        trace
            .steps
            .iter()
            .map(|step| step.name.as_str())
            .collect::<Vec<_>>(),
        vec![
            "query-change",
            "page-window",
            "revoke-permission",
            "corpus-change",
            "close-search",
        ]
    );
    assert!(trace.steps.iter().all(|step| {
        step.trace
            .invariant_results
            .iter()
            .any(|result| result.name == "incremental_equals_full_recompute" && result.passed)
    }));
    assert!(trace.steps.iter().any(|step| {
        step.trace.output_frames.iter().any(|frame| {
            matches!(
                frame.kind,
                OutputFrameKindTrace::Delta | OutputFrameKindTrace::Rebaseline(_)
            )
        })
    }));
    assert!(trace.steps.iter().any(|step| {
        step.trace
            .scope_events
            .iter()
            .any(|event| event.kind == ScopeLifecycleKind::Closed)
    }));
}

#[test]
fn seeded_bug_capsule_detects_stale_revoked_result() {
    let report = run_bug_capsule("search-permission-revoke-clears-results").unwrap();

    assert_eq!(report.status, "pass");
    assert!(report.expected_failures_detected);
    assert_eq!(available_bug_capsules().len(), 1);
}
