use std::collections::BTreeSet;

use super::types::{PluginManifest, PluginPermission, WorkspaceKind};

/// Builds a sorted set from literal values.
pub fn ids<const N: usize>(values: [&str; N]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

/// All permissions used by the PluginHost showcase.
pub fn all_permissions() -> BTreeSet<PluginPermission> {
    [
        PluginPermission::FileSystemRead,
        PluginPermission::BackgroundWorker,
        PluginPermission::Ipc,
    ]
    .into_iter()
    .collect()
}

/// Permission set that leaves commands and panels but denies hidden capabilities.
pub fn command_panel_permissions() -> BTreeSet<PluginPermission> {
    BTreeSet::new()
}

/// Initial Rust formatter plugin manifest.
pub fn rust_formatter_manifest() -> PluginManifest {
    PluginManifest {
        plugin_id: "fmt".to_owned(),
        compatible_workspaces: [WorkspaceKind::Rust].into_iter().collect(),
        commands: ids(["Format file"]),
        panels: ids(["Formatter"]),
        file_watchers: ids(["src/**/*.rs"]),
        workers: ids(["rustfmt-daemon"]),
        ipc_channels: ids(["fmt/lsp"]),
    }
}

/// Updated Rust formatter plugin manifest with additional contributions.
pub fn rust_formatter_manifest_v2() -> PluginManifest {
    PluginManifest {
        plugin_id: "fmt".to_owned(),
        compatible_workspaces: [WorkspaceKind::Rust].into_iter().collect(),
        commands: ids(["Format file", "Format selection"]),
        panels: ids(["Formatter", "Format preview"]),
        file_watchers: ids(["src/**/*.rs", "Cargo.toml"]),
        workers: ids(["rustfmt-daemon", "format-cache"]),
        ipc_channels: ids(["fmt/lsp", "fmt/preview"]),
    }
}
