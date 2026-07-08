//! PluginHost capability lifecycle secondary showcase.

mod bug_capsules;
mod engine;
mod graph;
mod sample;
mod scripts;
mod types;

#[cfg(test)]
mod tests;

pub use bug_capsules::{available_bug_capsules, run_all_bug_capsules, run_bug_capsule};
pub use engine::PluginHostApp;
pub use sample::{
    all_permissions, command_panel_permissions, ids, rust_formatter_manifest,
    rust_formatter_manifest_v2,
};
pub use scripts::capability_lifecycle_showcase_trace;
pub use types::{
    PluginEffect, PluginFrame, PluginHandle, PluginHostEvent, PluginHostUpdate, PluginManifest,
    PluginPermission, PluginResource, PluginShellSnapshot, WorkspaceKind,
};
