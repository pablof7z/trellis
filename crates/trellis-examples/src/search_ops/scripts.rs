use crate::showcase_trace::{ShowcaseStep, ShowcaseTrace, build_showcase_trace};

use super::SearchOpsApp;
use super::sample::{opening_search, sample_catalog};
use super::types::{ResultWindow, SearchOpsEvent};

/// Runs the headless `search-lifecycle` showcase script.
pub fn search_lifecycle_showcase_trace() -> ShowcaseTrace {
    build_showcase_trace(
        "search-ops",
        "search-lifecycle",
        &[
            "cargo",
            "run",
            "-p",
            "trellis-examples",
            "--example",
            "search_ops",
            "--",
            "--script",
            "search-lifecycle",
        ],
        || {
            let mut app = SearchOpsApp::new(sample_catalog());
            let search = app.open_search(opening_search());
            app.drain_effects();
            app.drain_output(search);
            app.drain_diagnostic_traces();

            app.apply_event(search, SearchOpsEvent::ChangeQuery("budget".to_owned()));
            let query_change = pop_trace(&mut app, "query-change");

            app.apply_event(
                search,
                SearchOpsEvent::SetWindow(ResultWindow { start: 1, len: 2 }),
            );
            let page_window = pop_trace(&mut app, "page-window");

            app.apply_event(
                search,
                SearchOpsEvent::RevokeDocumentPermission {
                    doc_id: "mail-002".to_owned(),
                },
            );
            let revoke_permission = pop_trace(&mut app, "revoke-permission");

            app.apply_event(search, SearchOpsEvent::SelectCorpus("docs".to_owned()));
            let corpus_change = pop_trace(&mut app, "corpus-change");

            app.close(search);
            let close_search = pop_trace(&mut app, "close-search");

            vec![
                query_change,
                page_window,
                revoke_permission,
                corpus_change,
                close_search,
            ]
        },
    )
}

fn pop_trace(app: &mut SearchOpsApp, name: &str) -> ShowcaseStep {
    let trace = app
        .drain_diagnostic_traces()
        .pop()
        .expect("script step emits one trace");
    ShowcaseStep {
        name: name.to_owned(),
        host_statuses: Vec::new(),
        trace,
    }
}
