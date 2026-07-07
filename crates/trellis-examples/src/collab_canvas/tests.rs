use super::*;

fn open_pair() -> (CollabCanvasApp, CollabDocumentHandle, CollabDocumentHandle) {
    let mut app = CollabCanvasApp::new();
    let design = app.open_document("design", design_doc_base(), ids([]));
    let style = app.open_document("style", style_doc(), ids([]));
    app.drain_effects();
    app.drain_output(design);
    app.drain_output(style);
    app.drain_diagnostic_traces();
    (app, design, style)
}

#[test]
fn visible_attachment_starts_and_hidden_cancels_hydration() {
    let (mut app, design, _) = open_pair();

    let update = app.apply_document_event(
        design,
        CollabDocumentEvent::SetVisibleAttachments(ids(["hero.png"])),
    );
    assert_eq!(update.emitted_frames, 1);
    assert!(app.drain_effects().contains(&CanvasEffect::Open(
        CanvasResource::AttachmentHydration("hero.png".to_owned())
    )));
    assert!(matches!(
        app.drain_output(design).first(),
        Some(CanvasFrame::Delta(snapshot))
            if snapshot.hydrated_attachments == ids(["hero.png"])
    ));

    app.apply_document_event(design, CollabDocumentEvent::SetVisibleAttachments(ids([])));
    assert!(app.drain_effects().contains(&CanvasEffect::Close(
        CanvasResource::AttachmentHydration("hero.png".to_owned())
    )));
    assert!(matches!(
        app.drain_output(design).first(),
        Some(CanvasFrame::Delta(snapshot)) if snapshot.hydrated_attachments.is_empty()
    ));
}

#[test]
fn embedded_doc_add_and_remove_reconciles_subdoc_room() {
    let (mut app, design, _) = open_pair();

    app.apply_document_event(
        design,
        CollabDocumentEvent::ReplaceManifest(design_doc_with_spec()),
    );
    assert!(
        app.drain_effects()
            .contains(&CanvasEffect::Open(CanvasResource::SubdocumentRoom(
                "spec".to_owned()
            )))
    );
    assert!(matches!(
        app.drain_output(design).first(),
        Some(CanvasFrame::Delta(snapshot)) if snapshot.subdocuments.contains("spec")
    ));

    app.apply_document_event(
        design,
        CollabDocumentEvent::ReplaceManifest(design_doc_base()),
    );
    assert!(
        app.drain_effects()
            .contains(&CanvasEffect::Close(CanvasResource::SubdocumentRoom(
                "spec".to_owned()
            )))
    );
    assert!(matches!(
        app.drain_output(design).first(),
        Some(CanvasFrame::Delta(snapshot)) if !snapshot.subdocuments.contains("spec")
    ));
}

#[test]
fn shared_subdocument_closes_after_last_owner() {
    let (mut app, design, style) = open_pair();

    app.close_document(design);
    let effects = app.drain_effects();
    assert!(
        effects.contains(&CanvasEffect::Close(CanvasResource::CommentThread(
            "intro-thread".to_owned()
        )))
    );
    assert!(
        effects.contains(&CanvasEffect::Close(CanvasResource::PresenceRoom(
            "design-room".to_owned()
        )))
    );
    assert!(
        !effects.contains(&CanvasEffect::Close(CanvasResource::SubdocumentRoom(
            "theme".to_owned()
        )))
    );
    assert!(app.drain_output(design).contains(&CanvasFrame::Cleared));

    app.close_document(style);
    assert!(
        app.drain_effects()
            .contains(&CanvasEffect::Close(CanvasResource::SubdocumentRoom(
                "theme".to_owned()
            )))
    );
}

#[test]
fn document_lifecycle_trace_uses_showcase_contract() {
    let trace = document_lifecycle_showcase_trace();

    assert_eq!(trace.showcase, "collab-canvas");
    assert_eq!(trace.script, "document-lifecycle");
    assert_eq!(trace.replay.status, "passed");
    assert_eq!(
        trace
            .steps
            .iter()
            .map(|step| step.name.as_str())
            .collect::<Vec<_>>(),
        vec![
            "show-attachment",
            "add-embedded-doc",
            "hide-attachment",
            "remove-embedded-doc",
            "close-design-document",
            "close-shared-document",
        ]
    );
    assert!(trace.steps.iter().all(|step| {
        step.trace
            .invariant_results
            .iter()
            .any(|result| result.name == "incremental_equals_full_recompute" && result.passed)
    }));
}

#[test]
fn seeded_bug_capsule_detects_stale_attachment_output() {
    let report = run_bug_capsule("collab-hidden-attachment-cancels-hydration").unwrap();

    assert_eq!(report.status, "pass");
    assert!(report.expected_failures_detected);
    assert_eq!(available_bug_capsules().len(), 1);
}
