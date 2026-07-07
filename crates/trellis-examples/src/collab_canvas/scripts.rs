use crate::showcase_trace::{ShowcaseTrace, build_showcase_trace};

use super::CollabCanvasApp;
use super::sample::{design_doc_base, design_doc_with_spec, ids, style_doc};
use super::types::CollabDocumentEvent;

/// Runs the headless `document-lifecycle` showcase script.
pub fn document_lifecycle_showcase_trace() -> ShowcaseTrace {
    build_showcase_trace(
        "collab-canvas",
        "document-lifecycle",
        &[
            "cargo",
            "run",
            "-p",
            "trellis-examples",
            "--example",
            "collab_canvas",
            "--",
            "--script",
            "document-lifecycle",
        ],
        || {
            let mut app = CollabCanvasApp::new();
            let design = app.open_document("design", design_doc_base(), ids([]));
            let style = app.open_document("style", style_doc(), ids([]));
            app.drain_effects();
            app.drain_output(design);
            app.drain_output(style);
            app.drain_diagnostic_traces();

            app.apply_document_event(
                design,
                CollabDocumentEvent::SetVisibleAttachments(ids(["hero.png"])),
            );
            let show_attachment = pop_trace(&mut app, "show-attachment");

            app.apply_document_event(
                design,
                CollabDocumentEvent::ReplaceManifest(design_doc_with_spec()),
            );
            let add_embedded_doc = pop_trace(&mut app, "add-embedded-doc");

            app.apply_document_event(design, CollabDocumentEvent::SetVisibleAttachments(ids([])));
            let hide_attachment = pop_trace(&mut app, "hide-attachment");

            app.apply_document_event(
                design,
                CollabDocumentEvent::ReplaceManifest(design_doc_base()),
            );
            let remove_embedded_doc = pop_trace(&mut app, "remove-embedded-doc");

            app.close_document(design);
            let close_design = pop_trace(&mut app, "close-design-document");

            app.close_document(style);
            let close_shared_doc = pop_trace(&mut app, "close-shared-document");

            vec![
                show_attachment,
                add_embedded_doc,
                hide_attachment,
                remove_embedded_doc,
                close_design,
                close_shared_doc,
            ]
        },
    )
}

fn pop_trace(app: &mut CollabCanvasApp, name: &str) -> crate::showcase_trace::ShowcaseStep {
    let trace = app
        .drain_diagnostic_traces()
        .pop()
        .expect("script step emits one trace");
    crate::showcase_trace::ShowcaseStep {
        name: name.to_owned(),
        host_statuses: Vec::new(),
        trace,
    }
}
