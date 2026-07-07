use std::collections::BTreeSet;

/// Opaque document handle returned by the CollabCanvas wrapper.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct CollabDocumentHandle(pub u64);

/// Document content discovered by the editor parser.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DocumentManifest {
    /// Embedded subdocument ids referenced by the document body.
    pub subdocuments: BTreeSet<String>,
    /// Comment thread ids visible for this document.
    pub comment_threads: BTreeSet<String>,
    /// Presence room ids joined while the document is open.
    pub presence_rooms: BTreeSet<String>,
    /// Attachment ids referenced by the document body.
    pub attachments: BTreeSet<String>,
}

/// One open editor session input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentSession {
    /// Stable document id.
    pub document_id: String,
    /// Parsed document manifest.
    pub manifest: DocumentManifest,
    /// Attachments currently visible in the viewport.
    pub visible_attachments: BTreeSet<String>,
}

/// User event applied to one open document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CollabDocumentEvent {
    /// Replace the parsed document manifest.
    ReplaceManifest(DocumentManifest),
    /// Replace the set of attachments currently visible in the viewport.
    SetVisibleAttachments(BTreeSet<String>),
}

/// External room or job managed by the host.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum CanvasResource {
    /// Collaborative room for an embedded subdocument.
    SubdocumentRoom(String),
    /// Comment thread room.
    CommentThread(String),
    /// Live presence room.
    PresenceRoom(String),
    /// Attachment hydration or thumbnail job.
    AttachmentHydration(String),
}

/// Host command payload for opening a CollabCanvas resource.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CanvasCommand {
    /// Open the given resource.
    Open(CanvasResource),
}

/// Host-side lifecycle effect emitted by the wrapper.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CanvasEffect {
    /// Open the given resource.
    Open(CanvasResource),
    /// Close the given resource.
    Close(CanvasResource),
}

/// Materialized editor output snapshot.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EditorSnapshot {
    /// Open document id, if any.
    pub document_id: Option<String>,
    /// Embedded subdocument rooms reflected in editor chrome.
    pub subdocuments: BTreeSet<String>,
    /// Comment rooms reflected in editor chrome.
    pub comment_threads: BTreeSet<String>,
    /// Presence rooms reflected in editor chrome.
    pub presence_rooms: BTreeSet<String>,
    /// Hydrated attachments visible in the editor.
    pub hydrated_attachments: BTreeSet<String>,
}

/// Materialized editor output frame.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CanvasFrame {
    /// Initial baseline frame.
    Baseline(EditorSnapshot),
    /// Incremental delta frame.
    Delta(EditorSnapshot),
    /// Explicit rebaseline frame.
    Rebaseline(EditorSnapshot),
    /// Clear frame emitted when the document scope closes.
    Cleared,
}

/// Count of wrapper effects and output frames emitted by an action.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CollabUpdate {
    /// Number of host lifecycle effects queued.
    pub emitted_effects: usize,
    /// Number of editor frames queued.
    pub emitted_frames: usize,
}
