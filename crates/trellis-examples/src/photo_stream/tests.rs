use trellis_core::ScopeLifecycleKind;

use super::*;

fn open_photos() -> (PhotoStreamApp, PhotoAlbumHandle) {
    let mut app = PhotoStreamApp::new(sample_catalog());
    let handle = app.open_album(opening_album());
    app.drain_effects();
    app.drain_output(handle);
    app.drain_diagnostic_traces();
    (app, handle)
}

#[test]
fn rule_change_cancels_removed_jobs_and_starts_added_jobs() {
    let (mut app, handle) = open_photos();

    app.apply_event(
        handle,
        PhotoStreamEvent::ReplaceRule(SmartAlbumRule::Person("Ava".to_owned())),
    );
    let effects = app.drain_effects();
    assert!(
        effects.contains(&PhotoEffect::Close(PhotoResource::ThumbnailJob(
            "asset-002".to_owned()
        )))
    );
    assert!(
        effects.contains(&PhotoEffect::Open(PhotoResource::ThumbnailJob(
            "asset-003".to_owned()
        )))
    );
    assert!(matches!(
        app.drain_output(handle).first(),
        Some(PhotoFrame::Delta(snapshot))
            if snapshot.tiles.len() <= 4
                && snapshot.tiles.iter().any(|tile| tile.asset_id == "asset-003")
    ));
}

#[test]
fn viewport_scroll_reconciles_offscreen_high_res_jobs() {
    let (mut app, handle) = open_photos();

    app.apply_event(
        handle,
        PhotoStreamEvent::ScrollViewport(AssetViewport { start: 2, len: 3 }),
    );
    let effects = app.drain_effects();
    assert!(
        effects.contains(&PhotoEffect::Close(PhotoResource::HighResPreview(
            "asset-001".to_owned()
        )))
    );
    assert!(
        effects.contains(&PhotoEffect::Open(PhotoResource::HighResPreview(
            "asset-008".to_owned()
        )))
    );
}

#[test]
fn storage_pressure_drops_optional_resources() {
    let (mut app, handle) = open_photos();

    app.apply_event(
        handle,
        PhotoStreamEvent::SetStoragePolicy(StoragePolicy::Constrained),
    );
    let effects = app.drain_effects();
    assert!(
        effects.contains(&PhotoEffect::Close(PhotoResource::HighResPreview(
            "asset-001".to_owned()
        )))
    );
    assert!(
        effects.contains(&PhotoEffect::Close(PhotoResource::CloudDownload(
            "asset-002".to_owned()
        )))
    );
    assert!(
        !effects.contains(&PhotoEffect::Close(PhotoResource::ThumbnailJob(
            "asset-001".to_owned()
        )))
    );
}

#[test]
fn scope_close_cancels_jobs_and_clears_grid() {
    let (mut app, handle) = open_photos();

    app.close(handle);
    let effects = app.drain_effects();
    assert!(
        effects.contains(&PhotoEffect::Close(PhotoResource::ThumbnailJob(
            "asset-001".to_owned()
        )))
    );
    assert!(
        effects.contains(&PhotoEffect::Close(PhotoResource::HighResPreview(
            "asset-001".to_owned()
        )))
    );
    assert!(app.drain_output(handle).contains(&PhotoFrame::Cleared));
}

#[test]
fn large_rule_diff_keeps_output_bounded() {
    let (mut app, handle) = open_photos();

    app.apply_event(
        handle,
        PhotoStreamEvent::ReplaceRule(SmartAlbumRule::AllAssets),
    );
    let trace = app.drain_diagnostic_traces().pop().unwrap();
    assert!(
        trace
            .collection_diffs
            .iter()
            .any(|diff| diff.added >= 100 || diff.unchanged >= 100)
    );
    assert!(matches!(
        app.drain_output(handle).first(),
        Some(PhotoFrame::Delta(snapshot))
            if snapshot.total_matches >= 100 && snapshot.tiles.len() <= 4
    ));
}

#[test]
fn smart_album_lifecycle_trace_uses_showcase_contract() {
    let trace = smart_album_lifecycle_showcase_trace();

    assert_eq!(trace.showcase, "photo-stream");
    assert_eq!(trace.script, "smart-album-lifecycle");
    assert_eq!(trace.replay.status, "passed");
    assert_eq!(
        trace
            .steps
            .iter()
            .map(|step| step.name.as_str())
            .collect::<Vec<_>>(),
        vec![
            "rule-change",
            "scroll-viewport",
            "storage-pressure",
            "large-album-diff",
            "close-album",
        ]
    );
    assert!(trace.steps.iter().all(|step| {
        step.trace
            .invariant_results
            .iter()
            .any(|result| result.name == "incremental_equals_full_recompute" && result.passed)
    }));
    assert!(trace.steps.iter().any(|step| {
        step.trace
            .scope_events
            .iter()
            .any(|event| event.kind == ScopeLifecycleKind::Closed)
    }));
}

#[test]
fn seeded_bug_capsule_detects_optional_work_leak() {
    let report = run_bug_capsule("photo-storage-pressure-drops-optional-work").unwrap();

    assert_eq!(report.status, "pass");
    assert!(report.expected_failures_detected);
    assert_eq!(available_bug_capsules().len(), 1);
}
