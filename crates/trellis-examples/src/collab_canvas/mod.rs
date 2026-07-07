//! CollabCanvas document/resource lifecycle secondary showcase.

mod bug_capsules;
mod engine;
mod graph;
mod sample;
mod scripts;
mod types;

#[cfg(test)]
mod tests;

pub use bug_capsules::{available_bug_capsules, run_all_bug_capsules, run_bug_capsule};
pub use engine::CollabCanvasApp;
pub use sample::{design_doc_base, design_doc_with_spec, ids, style_doc};
pub use scripts::document_lifecycle_showcase_trace;
pub use types::{
    CanvasCommand, CanvasEffect, CanvasFrame, CanvasResource, CollabDocumentEvent,
    CollabDocumentHandle, CollabUpdate, DocumentManifest, DocumentSession, EditorSnapshot,
};
