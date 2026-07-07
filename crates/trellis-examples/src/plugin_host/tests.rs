use super::*;

fn open_formatter() -> (PluginHostApp, PluginHandle) {
    let mut app = PluginHostApp::new(WorkspaceKind::Rust, all_permissions());
    let handle = app.enable_plugin(rust_formatter_manifest());
    app.drain_effects();
    app.drain_output(handle);
    app.drain_diagnostic_traces();
    (app, handle)
}

#[test]
fn manifest_change_diffs_plugin_contributions() {
    let (mut app, handle) = open_formatter();

    let update = app.apply_plugin_event(
        handle,
        PluginHostEvent::ReplaceManifest(rust_formatter_manifest_v2()),
    );
    assert!(update.emitted_effects >= 5);
    let effects = app.drain_effects();
    assert!(
        effects.contains(&PluginEffect::Open(PluginResource::Command {
            plugin_id: "fmt".to_owned(),
            name: "Format selection".to_owned(),
        }))
    );
    assert!(
        effects.contains(&PluginEffect::Open(PluginResource::FileWatcher {
            plugin_id: "fmt".to_owned(),
            glob: "Cargo.toml".to_owned(),
        }))
    );

    assert!(matches!(
        app.drain_output(handle).first(),
        Some(PluginFrame::Delta(snapshot))
            if snapshot.commands.contains("Format selection")
                && snapshot.file_watchers.contains("Cargo.toml")
    ));
}

#[test]
fn permission_revoke_closes_hidden_capabilities() {
    let (mut app, handle) = open_formatter();

    app.apply_plugin_event(
        handle,
        PluginHostEvent::ReplacePermissions(command_panel_permissions()),
    );
    let effects = app.drain_effects();
    assert!(
        effects.contains(&PluginEffect::Close(PluginResource::FileWatcher {
            plugin_id: "fmt".to_owned(),
            glob: "src/**/*.rs".to_owned(),
        }))
    );
    assert!(
        effects.contains(&PluginEffect::Close(PluginResource::BackgroundWorker {
            plugin_id: "fmt".to_owned(),
            name: "rustfmt-daemon".to_owned(),
        }))
    );
    assert!(
        effects.contains(&PluginEffect::Close(PluginResource::IpcChannel {
            plugin_id: "fmt".to_owned(),
            name: "fmt/lsp".to_owned(),
        }))
    );
    assert!(matches!(
        app.drain_output(handle).first(),
        Some(PluginFrame::Delta(snapshot))
            if snapshot.commands.contains("Format file")
                && snapshot.file_watchers.is_empty()
                && snapshot.workers.is_empty()
                && snapshot.ipc_channels.is_empty()
    ));
}

#[test]
fn workspace_change_disables_and_reenables_plugin_resources() {
    let (mut app, handle) = open_formatter();

    app.apply_plugin_event(
        handle,
        PluginHostEvent::SetWorkspaceKind(WorkspaceKind::Markdown),
    );
    let close_effects = app.drain_effects();
    assert!(close_effects.iter().all(|effect| matches!(
        effect,
        PluginEffect::Close(PluginResource::Command { .. })
            | PluginEffect::Close(PluginResource::Panel { .. })
            | PluginEffect::Close(PluginResource::FileWatcher { .. })
            | PluginEffect::Close(PluginResource::BackgroundWorker { .. })
            | PluginEffect::Close(PluginResource::IpcChannel { .. })
    )));
    assert!(matches!(
        app.drain_output(handle).first(),
        Some(PluginFrame::Delta(snapshot))
            if snapshot.workspace_kind == WorkspaceKind::Markdown
                && snapshot.commands.is_empty()
                && snapshot.panels.is_empty()
    ));

    app.apply_plugin_event(
        handle,
        PluginHostEvent::SetWorkspaceKind(WorkspaceKind::Rust),
    );
    let open_effects = app.drain_effects();
    assert!(
        open_effects.contains(&PluginEffect::Open(PluginResource::Command {
            plugin_id: "fmt".to_owned(),
            name: "Format file".to_owned(),
        }))
    );
    assert!(matches!(
        app.drain_output(handle).first(),
        Some(PluginFrame::Delta(snapshot))
            if snapshot.workspace_kind == WorkspaceKind::Rust
                && snapshot.commands.contains("Format file")
                && snapshot.panels.contains("Formatter")
    ));
}

#[test]
fn disable_closes_all_capabilities_and_clears_output() {
    let (mut app, handle) = open_formatter();

    app.disable_plugin(handle);
    let effects = app.drain_effects();
    assert!(
        effects.contains(&PluginEffect::Close(PluginResource::Command {
            plugin_id: "fmt".to_owned(),
            name: "Format file".to_owned(),
        }))
    );
    assert!(
        effects.contains(&PluginEffect::Close(PluginResource::Panel {
            plugin_id: "fmt".to_owned(),
            name: "Formatter".to_owned(),
        }))
    );
    assert!(
        effects.contains(&PluginEffect::Close(PluginResource::FileWatcher {
            plugin_id: "fmt".to_owned(),
            glob: "src/**/*.rs".to_owned(),
        }))
    );
    assert!(
        effects.contains(&PluginEffect::Close(PluginResource::BackgroundWorker {
            plugin_id: "fmt".to_owned(),
            name: "rustfmt-daemon".to_owned(),
        }))
    );
    assert!(
        effects.contains(&PluginEffect::Close(PluginResource::IpcChannel {
            plugin_id: "fmt".to_owned(),
            name: "fmt/lsp".to_owned(),
        }))
    );
    assert!(app.drain_output(handle).contains(&PluginFrame::Cleared));
}

#[test]
fn capability_lifecycle_trace_uses_showcase_contract() {
    let trace = capability_lifecycle_showcase_trace();

    assert_eq!(trace.showcase, "plugin-host");
    assert_eq!(trace.script, "capability-lifecycle");
    assert_eq!(trace.replay.status, "passed");
    assert_eq!(
        trace
            .steps
            .iter()
            .map(|step| step.name.as_str())
            .collect::<Vec<_>>(),
        vec![
            "manifest-change",
            "revoke-permissions",
            "unsupported-workspace",
            "supported-workspace",
            "restore-permissions",
            "disable-plugin",
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
fn seeded_bug_capsule_detects_stale_plugin_capabilities() {
    let report = run_bug_capsule("plugin-disable-closes-capabilities").unwrap();

    assert_eq!(report.status, "pass");
    assert!(report.expected_failures_detected);
    assert_eq!(available_bug_capsules().len(), 1);
}
