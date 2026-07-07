use crate::showcase_trace::{ShowcaseStep, ShowcaseTrace, build_showcase_trace};

use super::PluginHostApp;
use super::sample::rust_formatter_manifest_v2;
use super::sample::{all_permissions, command_panel_permissions, rust_formatter_manifest};
use super::types::{PluginHostEvent, WorkspaceKind};

/// Runs the headless `capability-lifecycle` showcase script.
pub fn capability_lifecycle_showcase_trace() -> ShowcaseTrace {
    build_showcase_trace(
        "plugin-host",
        "capability-lifecycle",
        &[
            "cargo",
            "run",
            "-p",
            "trellis-examples",
            "--example",
            "plugin_host",
            "--",
            "--script",
            "capability-lifecycle",
        ],
        || {
            let mut app = PluginHostApp::new(WorkspaceKind::Rust, all_permissions());
            let fmt = app.enable_plugin(rust_formatter_manifest());
            app.drain_effects();
            app.drain_output(fmt);
            app.drain_diagnostic_traces();

            app.apply_plugin_event(
                fmt,
                PluginHostEvent::ReplaceManifest(rust_formatter_manifest_v2()),
            );
            let manifest_change = pop_trace(&mut app, "manifest-change");

            app.apply_plugin_event(
                fmt,
                PluginHostEvent::ReplacePermissions(command_panel_permissions()),
            );
            let revoke_permissions = pop_trace(&mut app, "revoke-permissions");

            app.apply_plugin_event(
                fmt,
                PluginHostEvent::SetWorkspaceKind(WorkspaceKind::Markdown),
            );
            let unsupported_workspace = pop_trace(&mut app, "unsupported-workspace");

            app.apply_plugin_event(fmt, PluginHostEvent::SetWorkspaceKind(WorkspaceKind::Rust));
            let supported_workspace = pop_trace(&mut app, "supported-workspace");

            app.apply_plugin_event(fmt, PluginHostEvent::ReplacePermissions(all_permissions()));
            let restore_permissions = pop_trace(&mut app, "restore-permissions");

            app.disable_plugin(fmt);
            let disable_plugin = pop_trace(&mut app, "disable-plugin");

            vec![
                manifest_change,
                revoke_permissions,
                unsupported_workspace,
                supported_workspace,
                restore_permissions,
                disable_plugin,
            ]
        },
    )
}

fn pop_trace(app: &mut PluginHostApp, name: &str) -> ShowcaseStep {
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
