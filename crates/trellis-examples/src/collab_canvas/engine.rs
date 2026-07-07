use std::collections::{BTreeMap, BTreeSet, VecDeque};

use trellis_core::{
    InvariantResultTrace, OutputFrameKind, OutputKey, ResourceCommand, TransactionResult,
    TransactionTrace,
};

use super::graph::{CollabCanvasGraph, DocumentSlot, build_graph, resource_from_key};
use super::types::{
    CanvasCommand, CanvasEffect, CanvasFrame, CollabDocumentEvent, CollabDocumentHandle,
    CollabUpdate, DocumentManifest, DocumentSession, EditorSnapshot,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct ActiveDocument {
    slot: DocumentSlot,
    open: bool,
}

/// Domain wrapper for the CollabCanvas document lifecycle showcase.
pub struct CollabCanvasApp {
    graph: CollabCanvasGraph,
    next_handle: u64,
    handles: BTreeMap<CollabDocumentHandle, ActiveDocument>,
    output_queue: BTreeMap<CollabDocumentHandle, VecDeque<CanvasFrame>>,
    effects: VecDeque<CanvasEffect>,
    diagnostic_traces: VecDeque<TransactionTrace>,
}

impl CollabCanvasApp {
    /// Creates an empty CollabCanvas app.
    pub fn new() -> Self {
        Self {
            graph: build_graph(),
            next_handle: 1,
            handles: BTreeMap::new(),
            output_queue: BTreeMap::new(),
            effects: VecDeque::new(),
            diagnostic_traces: VecDeque::new(),
        }
    }

    /// Opens one document and returns an opaque handle.
    pub fn open_document(
        &mut self,
        document_id: impl Into<String>,
        manifest: DocumentManifest,
        visible_attachments: BTreeSet<String>,
    ) -> CollabDocumentHandle {
        let slot = self
            .next_slot()
            .expect("CollabCanvas supports two documents");
        let handle = CollabDocumentHandle(self.next_handle);
        self.next_handle += 1;
        self.handles
            .insert(handle, ActiveDocument { slot, open: true });
        self.output_queue.entry(handle).or_default();

        let session = DocumentSession {
            document_id: document_id.into(),
            manifest,
            visible_attachments,
        };
        self.commit_session(slot, Some(session));
        handle
    }

    /// Applies one domain event to an open document.
    pub fn apply_document_event(
        &mut self,
        handle: CollabDocumentHandle,
        event: CollabDocumentEvent,
    ) -> CollabUpdate {
        let Some(active) = self
            .handles
            .get(&handle)
            .copied()
            .filter(|active| active.open)
        else {
            return CollabUpdate::default();
        };
        let Some(mut session) = self.current_session(active.slot) else {
            return CollabUpdate::default();
        };
        match event {
            CollabDocumentEvent::ReplaceManifest(manifest) => {
                session.manifest = manifest;
            }
            CollabDocumentEvent::SetVisibleAttachments(visible) => {
                session.visible_attachments = visible;
            }
        }
        self.commit_session(active.slot, Some(session))
    }

    /// Closes a document scope and clears its editor output.
    pub fn close_document(&mut self, handle: CollabDocumentHandle) -> CollabUpdate {
        let Some(active) = self
            .handles
            .get(&handle)
            .copied()
            .filter(|active| active.open)
        else {
            return CollabUpdate::default();
        };
        let before_effects = self.effects.len();
        let before_frames = self.output_frame_count();
        let scope = self.graph.document(active.slot).scope;

        let mut tx = self.graph.graph.begin_transaction().unwrap();
        tx.close_scope(scope).unwrap();
        let result = tx.commit().unwrap();
        drop(tx);
        self.apply_result(result);

        if let Some(active) = self.handles.get_mut(&handle) {
            active.open = false;
        }
        CollabUpdate {
            emitted_effects: self.effects.len() - before_effects,
            emitted_frames: self.output_frame_count() - before_frames,
        }
    }

    /// Drains queued host lifecycle effects.
    pub fn drain_effects(&mut self) -> Vec<CanvasEffect> {
        self.effects.drain(..).collect()
    }

    /// Drains queued editor frames for a document handle.
    pub fn drain_output(&mut self, handle: CollabDocumentHandle) -> Vec<CanvasFrame> {
        self.output_queue
            .entry(handle)
            .or_default()
            .drain(..)
            .collect()
    }

    /// Drains diagnostic transaction traces.
    pub fn drain_diagnostic_traces(&mut self) -> Vec<TransactionTrace> {
        self.diagnostic_traces.drain(..).collect()
    }

    fn next_slot(&self) -> Option<DocumentSlot> {
        [DocumentSlot::Primary, DocumentSlot::Secondary]
            .into_iter()
            .find(|slot| !self.handles.values().any(|active| active.slot == *slot))
    }

    fn current_session(&self, slot: DocumentSlot) -> Option<DocumentSession> {
        self.graph
            .graph
            .input_value(self.graph.document(slot).session)
            .unwrap()
            .cloned()
            .unwrap_or_default()
    }

    fn commit_session(
        &mut self,
        slot: DocumentSlot,
        session: Option<DocumentSession>,
    ) -> CollabUpdate {
        let before_effects = self.effects.len();
        let before_frames = self.output_frame_count();
        let session_input = self.graph.document(slot).session;
        let mut tx = self.graph.graph.begin_transaction().unwrap();
        tx.set_input(session_input, session).unwrap();
        let result = tx.commit().unwrap();
        drop(tx);
        self.apply_result(result);
        CollabUpdate {
            emitted_effects: self.effects.len() - before_effects,
            emitted_frames: self.output_frame_count() - before_frames,
        }
    }

    fn apply_result(&mut self, result: TransactionResult<CanvasCommand>) {
        for command in result.resource_plan.commands() {
            match command {
                ResourceCommand::Open {
                    command: CanvasCommand::Open(resource),
                    ..
                } => self.effects.push_back(CanvasEffect::Open(resource.clone())),
                ResourceCommand::Replace {
                    command: CanvasCommand::Open(resource),
                    ..
                }
                | ResourceCommand::Refresh {
                    command: CanvasCommand::Open(resource),
                    ..
                } => self.effects.push_back(CanvasEffect::Open(resource.clone())),
                ResourceCommand::Close { key, .. } => {
                    if let Some(resource) = resource_from_key(key) {
                        self.effects.push_back(CanvasEffect::Close(resource));
                    }
                }
            }
        }

        for frame in &result.output_frames {
            let Some(handle) = self.handle_for_output(frame.output_key) else {
                continue;
            };
            let frame = canvas_frame(&frame.kind);
            self.output_queue
                .entry(handle)
                .or_default()
                .push_back(frame);
        }

        let mut trace = result.trace();
        trace.invariant_results.push(InvariantResultTrace {
            name: "incremental_equals_full_recompute".to_owned(),
            passed: self.graph.graph.full_recompute_check().is_ok(),
        });
        self.diagnostic_traces.push_back(trace);
    }

    fn handle_for_output(&self, key: OutputKey) -> Option<CollabDocumentHandle> {
        self.handles.iter().find_map(|(handle, active)| {
            (self.graph.document(active.slot).output.key() == key).then_some(*handle)
        })
    }

    fn output_frame_count(&self) -> usize {
        self.output_queue.values().map(VecDeque::len).sum()
    }
}

impl Default for CollabCanvasApp {
    fn default() -> Self {
        Self::new()
    }
}

fn canvas_frame(kind: &OutputFrameKind) -> CanvasFrame {
    match kind {
        OutputFrameKind::Baseline(value) => {
            CanvasFrame::Baseline(value.get::<EditorSnapshot>().cloned().unwrap_or_default())
        }
        OutputFrameKind::Delta(value) => {
            CanvasFrame::Delta(value.get::<EditorSnapshot>().cloned().unwrap_or_default())
        }
        OutputFrameKind::Rebaseline(value, _) => {
            CanvasFrame::Rebaseline(value.get::<EditorSnapshot>().cloned().unwrap_or_default())
        }
        OutputFrameKind::Clear(_) => CanvasFrame::Cleared,
    }
}
