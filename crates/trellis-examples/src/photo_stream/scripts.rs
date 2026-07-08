use crate::showcase_trace::{ShowcaseStep, ShowcaseTrace, build_showcase_trace};

use super::PhotoStreamApp;
use super::sample::{opening_album, sample_catalog};
use super::types::{AssetViewport, PhotoStreamEvent, SmartAlbumRule, StoragePolicy};

/// Runs the headless `smart-album-lifecycle` showcase script.
pub fn smart_album_lifecycle_showcase_trace() -> ShowcaseTrace {
    build_showcase_trace(
        "photo-stream",
        "smart-album-lifecycle",
        &[
            "cargo",
            "run",
            "-p",
            "trellis-examples",
            "--example",
            "photo_stream",
            "--",
            "--script",
            "smart-album-lifecycle",
        ],
        || {
            let mut app = PhotoStreamApp::new(sample_catalog());
            let album = app.open_album(opening_album());
            app.drain_effects();
            app.drain_output(album);
            app.drain_diagnostic_traces();

            app.apply_event(
                album,
                PhotoStreamEvent::ReplaceRule(SmartAlbumRule::Person("Ava".to_owned())),
            );
            let rule_change = pop_trace(&mut app, "rule-change");

            app.apply_event(
                album,
                PhotoStreamEvent::ScrollViewport(AssetViewport { start: 3, len: 4 }),
            );
            let scroll_viewport = pop_trace(&mut app, "scroll-viewport");

            app.apply_event(
                album,
                PhotoStreamEvent::SetStoragePolicy(StoragePolicy::Constrained),
            );
            let storage_pressure = pop_trace(&mut app, "storage-pressure");

            app.apply_event(
                album,
                PhotoStreamEvent::ReplaceRule(SmartAlbumRule::AllAssets),
            );
            let large_album_diff = pop_trace(&mut app, "large-album-diff");

            app.close(album);
            let close_album = pop_trace(&mut app, "close-album");

            vec![
                rule_change,
                scroll_viewport,
                storage_pressure,
                large_album_diff,
                close_album,
            ]
        },
    )
}

fn pop_trace(app: &mut PhotoStreamApp, name: &str) -> ShowcaseStep {
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
