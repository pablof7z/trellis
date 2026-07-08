use std::collections::VecDeque;

use trellis_core::{
    InvariantResultTrace, OutputFrameKind, ResourceCommand, TransactionResult, TransactionTrace,
};

use super::graph::{SearchGraph, build_graph, resource_from_key};
use super::types::{
    SearchCatalog, SearchCommand, SearchEffect, SearchFrame, SearchHandle, SearchOpsEvent,
    SearchOpsUpdate, SearchSession, SearchSnapshot,
};

/// Domain wrapper for the SearchOps live search/index showcase.
pub struct SearchOpsApp {
    graph: SearchGraph,
    next_handle: u64,
    handle: Option<SearchHandle>,
    closed: bool,
    effects: VecDeque<SearchEffect>,
    output_queue: VecDeque<SearchFrame>,
    diagnostic_traces: VecDeque<TransactionTrace>,
}

impl SearchOpsApp {
    /// Creates a SearchOps app around host-owned catalog data.
    pub fn new(catalog: SearchCatalog) -> Self {
        Self {
            graph: build_graph(catalog),
            next_handle: 1,
            handle: None,
            closed: true,
            effects: VecDeque::new(),
            output_queue: VecDeque::new(),
            diagnostic_traces: VecDeque::new(),
        }
    }

    /// Opens one search workspace and returns an opaque handle.
    pub fn open_search(&mut self, session: SearchSession) -> SearchHandle {
        let handle = self.handle.unwrap_or_else(|| {
            let handle = SearchHandle(self.next_handle);
            self.next_handle += 1;
            self.handle = Some(handle);
            handle
        });
        self.closed = false;
        let _ = self.commit_inputs(Some(session), self.current_catalog(), false);
        handle
    }

    /// Applies one domain event to an open search workspace.
    pub fn apply_event(&mut self, handle: SearchHandle, event: SearchOpsEvent) -> SearchOpsUpdate {
        if !self.handle_is_open(handle) {
            return SearchOpsUpdate::default();
        }
        let mut session = self.current_session();
        let mut catalog = self.current_catalog();
        let mut rebaseline = false;
        match event {
            SearchOpsEvent::SelectCorpus(corpus) => {
                if let Some(session) = session.as_mut() {
                    session.corpus = corpus;
                    session.window.start = 0;
                }
            }
            SearchOpsEvent::ChangeQuery(query) => {
                if let Some(session) = session.as_mut() {
                    session.query = query;
                    session.window.start = 0;
                }
            }
            SearchOpsEvent::ReplaceFilter(filter) => {
                if let Some(session) = session.as_mut() {
                    session.filter = filter;
                    session.window.start = 0;
                }
            }
            SearchOpsEvent::SetWindow(window) => {
                if let Some(session) = session.as_mut() {
                    session.window = window;
                    rebaseline = true;
                }
            }
            SearchOpsEvent::RevokeDocumentPermission { doc_id } => {
                if let (Some(session), Some(doc)) =
                    (session.as_ref(), catalog.documents.get_mut(&doc_id))
                {
                    doc.allowed_users.remove(&session.user);
                }
            }
            SearchOpsEvent::ReplaceCatalog(next) => {
                catalog = next;
            }
        }
        self.commit_inputs(session, catalog, rebaseline)
    }

    /// Closes the search workspace and clears resources and output.
    pub fn close(&mut self, handle: SearchHandle) -> SearchOpsUpdate {
        if !self.handle_is_open(handle) {
            return SearchOpsUpdate::default();
        }
        let before_effects = self.effects.len();
        let before_frames = self.output_queue.len();
        let mut tx = self.graph.graph.begin_transaction().unwrap();
        tx.close_scope(self.graph.search_scope).unwrap();
        let result = tx.commit().unwrap();
        drop(tx);
        self.apply_result(result);
        self.closed = true;
        SearchOpsUpdate {
            emitted_effects: self.effects.len() - before_effects,
            emitted_frames: self.output_queue.len() - before_frames,
        }
    }

    /// Drains queued host lifecycle effects.
    pub fn drain_effects(&mut self) -> Vec<SearchEffect> {
        self.effects.drain(..).collect()
    }

    /// Drains queued search result frames for a handle.
    pub fn drain_output(&mut self, handle: SearchHandle) -> Vec<SearchFrame> {
        if Some(handle) != self.handle {
            return Vec::new();
        }
        self.output_queue.drain(..).collect()
    }

    /// Drains diagnostic transaction traces.
    pub fn drain_diagnostic_traces(&mut self) -> Vec<TransactionTrace> {
        self.diagnostic_traces.drain(..).collect()
    }

    fn commit_inputs(
        &mut self,
        session: Option<SearchSession>,
        catalog: SearchCatalog,
        rebaseline: bool,
    ) -> SearchOpsUpdate {
        let before_effects = self.effects.len();
        let before_frames = self.output_queue.len();
        let mut tx = self.graph.graph.begin_transaction().unwrap();
        tx.set_input(self.graph.session, session).unwrap();
        tx.set_input(self.graph.catalog, catalog).unwrap();
        if rebaseline {
            tx.rebaseline_output(self.graph.output.clone()).unwrap();
        }
        let result = tx.commit().unwrap();
        drop(tx);
        self.apply_result(result);
        SearchOpsUpdate {
            emitted_effects: self.effects.len() - before_effects,
            emitted_frames: self.output_queue.len() - before_frames,
        }
    }

    fn handle_is_open(&self, handle: SearchHandle) -> bool {
        Some(handle) == self.handle && !self.closed
    }

    fn current_session(&self) -> Option<SearchSession> {
        self.graph
            .graph
            .input_value(self.graph.session)
            .unwrap()
            .cloned()
            .unwrap_or_default()
    }

    fn current_catalog(&self) -> SearchCatalog {
        self.graph
            .graph
            .input_value(self.graph.catalog)
            .unwrap()
            .cloned()
            .unwrap_or_default()
    }

    fn apply_result(&mut self, result: TransactionResult<SearchCommand>) {
        for command in result.resource_plan.commands() {
            match command {
                ResourceCommand::Open {
                    command: SearchCommand::Open(resource),
                    ..
                } => self.effects.push_back(SearchEffect::Open(resource.clone())),
                ResourceCommand::Replace {
                    command: SearchCommand::Open(resource),
                    ..
                }
                | ResourceCommand::Refresh {
                    command: SearchCommand::Open(resource),
                    ..
                } => self.effects.push_back(SearchEffect::Open(resource.clone())),
                ResourceCommand::Close { key, .. } => {
                    if let Some(resource) = resource_from_key(key) {
                        self.effects.push_back(SearchEffect::Close(resource));
                    }
                }
            }
        }

        for frame in &result.output_frames {
            self.output_queue.push_back(search_frame(&frame.kind));
        }

        let mut trace = result.trace();
        trace.invariant_results.push(InvariantResultTrace {
            name: "incremental_equals_full_recompute".to_owned(),
            passed: self.graph.graph.full_recompute_check().is_ok(),
        });
        self.diagnostic_traces.push_back(trace);
    }
}

impl Default for SearchOpsApp {
    fn default() -> Self {
        Self::new(SearchCatalog::default())
    }
}

fn search_frame(kind: &OutputFrameKind) -> SearchFrame {
    match kind {
        OutputFrameKind::Baseline(value) => {
            SearchFrame::Baseline(value.get::<SearchSnapshot>().cloned().unwrap_or_default())
        }
        OutputFrameKind::Delta(value) => {
            SearchFrame::Delta(value.get::<SearchSnapshot>().cloned().unwrap_or_default())
        }
        OutputFrameKind::Rebaseline(value, _) => {
            SearchFrame::Rebaseline(value.get::<SearchSnapshot>().cloned().unwrap_or_default())
        }
        OutputFrameKind::Clear(_) => SearchFrame::Cleared,
    }
}
