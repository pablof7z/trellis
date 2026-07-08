use std::collections::VecDeque;

use trellis_core::{
    InvariantResultTrace, OutputFrameKind, ResourceCommand, TransactionResult, TransactionTrace,
};

use super::graph::{PhotoStreamGraph, build_graph, resource_from_key};
use super::types::{
    PhotoAlbumHandle, PhotoCatalog, PhotoCommand, PhotoEffect, PhotoFrame, PhotoGridSnapshot,
    PhotoStreamEvent, PhotoStreamUpdate, SmartAlbumSession,
};

/// Domain wrapper for the PhotoStream smart-album showcase.
pub struct PhotoStreamApp {
    graph: PhotoStreamGraph,
    next_handle: u64,
    handle: Option<PhotoAlbumHandle>,
    closed: bool,
    effects: VecDeque<PhotoEffect>,
    output_queue: VecDeque<PhotoFrame>,
    diagnostic_traces: VecDeque<TransactionTrace>,
}

impl PhotoStreamApp {
    /// Creates a PhotoStream app around host-owned catalog data.
    pub fn new(catalog: PhotoCatalog) -> Self {
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

    /// Opens one smart album and returns an opaque handle.
    pub fn open_album(&mut self, session: SmartAlbumSession) -> PhotoAlbumHandle {
        let handle = self.handle.unwrap_or_else(|| {
            let handle = PhotoAlbumHandle(self.next_handle);
            self.next_handle += 1;
            self.handle = Some(handle);
            handle
        });
        self.closed = false;
        let _ = self.commit_inputs(Some(session), self.current_catalog());
        handle
    }

    /// Applies one domain event to an open smart album.
    pub fn apply_event(
        &mut self,
        handle: PhotoAlbumHandle,
        event: PhotoStreamEvent,
    ) -> PhotoStreamUpdate {
        if !self.handle_is_open(handle) {
            return PhotoStreamUpdate::default();
        }
        let mut session = self.current_session();
        let mut catalog = self.current_catalog();
        match event {
            PhotoStreamEvent::ReplaceRule(rule) => {
                if let Some(session) = session.as_mut() {
                    session.rule = rule;
                    session.viewport.start = 0;
                }
            }
            PhotoStreamEvent::ScrollViewport(viewport) => {
                if let Some(session) = session.as_mut() {
                    session.viewport = viewport;
                }
            }
            PhotoStreamEvent::SetStoragePolicy(policy) => {
                if let Some(session) = session.as_mut() {
                    session.storage_policy = policy;
                }
            }
            PhotoStreamEvent::ReplaceCatalog(next) => {
                catalog = next;
            }
        }
        self.commit_inputs(session, catalog)
    }

    /// Closes the smart album scope and clears jobs and output.
    pub fn close(&mut self, handle: PhotoAlbumHandle) -> PhotoStreamUpdate {
        if !self.handle_is_open(handle) {
            return PhotoStreamUpdate::default();
        }
        let before_effects = self.effects.len();
        let before_frames = self.output_queue.len();
        let mut tx = self.graph.graph.begin_transaction().unwrap();
        tx.close_scope(self.graph.album_scope).unwrap();
        let result = tx.commit().unwrap();
        drop(tx);
        self.apply_result(result);
        self.closed = true;
        PhotoStreamUpdate {
            emitted_effects: self.effects.len() - before_effects,
            emitted_frames: self.output_queue.len() - before_frames,
        }
    }

    /// Drains queued host lifecycle effects.
    pub fn drain_effects(&mut self) -> Vec<PhotoEffect> {
        self.effects.drain(..).collect()
    }

    /// Drains queued grid frames for a handle.
    pub fn drain_output(&mut self, handle: PhotoAlbumHandle) -> Vec<PhotoFrame> {
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
        session: Option<SmartAlbumSession>,
        catalog: PhotoCatalog,
    ) -> PhotoStreamUpdate {
        let before_effects = self.effects.len();
        let before_frames = self.output_queue.len();
        let mut tx = self.graph.graph.begin_transaction().unwrap();
        tx.set_input(self.graph.session, session).unwrap();
        tx.set_input(self.graph.catalog, catalog).unwrap();
        let result = tx.commit().unwrap();
        drop(tx);
        self.apply_result(result);
        PhotoStreamUpdate {
            emitted_effects: self.effects.len() - before_effects,
            emitted_frames: self.output_queue.len() - before_frames,
        }
    }

    fn handle_is_open(&self, handle: PhotoAlbumHandle) -> bool {
        Some(handle) == self.handle && !self.closed
    }

    fn current_session(&self) -> Option<SmartAlbumSession> {
        self.graph
            .graph
            .input_value(self.graph.session)
            .unwrap()
            .cloned()
            .unwrap_or_default()
    }

    fn current_catalog(&self) -> PhotoCatalog {
        self.graph
            .graph
            .input_value(self.graph.catalog)
            .unwrap()
            .cloned()
            .unwrap_or_default()
    }

    fn apply_result(&mut self, result: TransactionResult<PhotoCommand>) {
        for command in result.resource_plan.commands() {
            match command {
                ResourceCommand::Open {
                    command: PhotoCommand::Open(resource),
                    ..
                } => self.effects.push_back(PhotoEffect::Open(resource.clone())),
                ResourceCommand::Replace {
                    command: PhotoCommand::Open(resource),
                    ..
                }
                | ResourceCommand::Refresh {
                    command: PhotoCommand::Open(resource),
                    ..
                } => self.effects.push_back(PhotoEffect::Open(resource.clone())),
                ResourceCommand::Close { key, .. } => {
                    if let Some(resource) = resource_from_key(key) {
                        self.effects.push_back(PhotoEffect::Close(resource));
                    }
                }
            }
        }

        for frame in &result.output_frames {
            self.output_queue.push_back(photo_frame(&frame.kind));
        }

        let mut trace = result.trace();
        trace.invariant_results.push(InvariantResultTrace {
            name: "incremental_equals_full_recompute".to_owned(),
            passed: self.graph.graph.full_recompute_check().is_ok(),
        });
        self.diagnostic_traces.push_back(trace);
    }
}

impl Default for PhotoStreamApp {
    fn default() -> Self {
        Self::new(PhotoCatalog::default())
    }
}

fn photo_frame(kind: &OutputFrameKind) -> PhotoFrame {
    match kind {
        OutputFrameKind::Baseline(value) => PhotoFrame::Baseline(
            value
                .get::<PhotoGridSnapshot>()
                .cloned()
                .unwrap_or_default(),
        ),
        OutputFrameKind::Delta(value) => PhotoFrame::Delta(
            value
                .get::<PhotoGridSnapshot>()
                .cloned()
                .unwrap_or_default(),
        ),
        OutputFrameKind::Rebaseline(value, _) => PhotoFrame::Rebaseline(
            value
                .get::<PhotoGridSnapshot>()
                .cloned()
                .unwrap_or_default(),
        ),
        OutputFrameKind::Clear(_) => PhotoFrame::Cleared,
    }
}
