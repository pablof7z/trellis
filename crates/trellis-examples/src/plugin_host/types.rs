use std::collections::BTreeSet;

/// Opaque handle for an enabled plugin.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct PluginHandle(pub u64);

/// Workspace kind used by the plugin host to gate manifest contributions.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub enum WorkspaceKind {
    /// Rust project workspace.
    #[default]
    Rust,
    /// Markdown or documentation workspace.
    Markdown,
}

/// Host permission that gates plugin capabilities.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum PluginPermission {
    /// Permission to create file watchers.
    FileSystemRead,
    /// Permission to run background workers.
    BackgroundWorker,
    /// Permission to open plugin IPC channels.
    Ipc,
}

/// Plugin manifest supplied by the app-level plugin registry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginManifest {
    /// Stable plugin id.
    pub plugin_id: String,
    /// Workspace kinds where this manifest can contribute capabilities.
    pub compatible_workspaces: BTreeSet<WorkspaceKind>,
    /// Command palette entries contributed by the plugin.
    pub commands: BTreeSet<String>,
    /// Panels contributed to the host shell.
    pub panels: BTreeSet<String>,
    /// File watcher globs contributed by the plugin.
    pub file_watchers: BTreeSet<String>,
    /// Background workers contributed by the plugin.
    pub workers: BTreeSet<String>,
    /// IPC channel names contributed by the plugin.
    pub ipc_channels: BTreeSet<String>,
}

/// Domain event applied to an enabled plugin host.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PluginHostEvent {
    /// Replace a plugin manifest and diff its contributions.
    ReplaceManifest(PluginManifest),
    /// Change the active workspace kind for the host.
    SetWorkspaceKind(WorkspaceKind),
    /// Replace the host permission grants.
    ReplacePermissions(BTreeSet<PluginPermission>),
    /// Revoke one host permission grant.
    RevokePermission(PluginPermission),
}

/// Concrete capability controlled by the plugin host.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum PluginResource {
    /// Command palette contribution.
    Command {
        /// Stable contributing plugin id.
        plugin_id: String,
        /// Command palette label.
        name: String,
    },
    /// Host panel contribution.
    Panel {
        /// Stable contributing plugin id.
        plugin_id: String,
        /// Panel label.
        name: String,
    },
    /// File watcher contribution.
    FileWatcher {
        /// Stable contributing plugin id.
        plugin_id: String,
        /// Watched file glob.
        glob: String,
    },
    /// Background worker contribution.
    BackgroundWorker {
        /// Stable contributing plugin id.
        plugin_id: String,
        /// Worker name.
        name: String,
    },
    /// IPC channel contribution.
    IpcChannel {
        /// Stable contributing plugin id.
        plugin_id: String,
        /// IPC channel name.
        name: String,
    },
}

/// Host command payload used by Trellis resource planning.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum PluginCommand {
    /// Open the given plugin capability.
    Open(PluginResource),
}

/// Typed effect emitted to the plugin host executor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PluginEffect {
    /// Open the given capability.
    Open(PluginResource),
    /// Close the given capability.
    Close(PluginResource),
}

/// Materialized command palette and shell contribution snapshot.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PluginShellSnapshot {
    /// Plugin id reflected by this output frame.
    pub plugin_id: Option<String>,
    /// Workspace kind used for the current derivation.
    pub workspace_kind: WorkspaceKind,
    /// Visible command palette entries.
    pub commands: BTreeSet<String>,
    /// Visible host panels.
    pub panels: BTreeSet<String>,
    /// Active file watcher globs.
    pub file_watchers: BTreeSet<String>,
    /// Active background worker names.
    pub workers: BTreeSet<String>,
    /// Active IPC channel names.
    pub ipc_channels: BTreeSet<String>,
}

/// Materialized plugin shell output frame.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PluginFrame {
    /// Initial baseline frame.
    Baseline(PluginShellSnapshot),
    /// Incremental delta frame.
    Delta(PluginShellSnapshot),
    /// Explicit rebaseline frame.
    Rebaseline(PluginShellSnapshot),
    /// Clear frame emitted when the plugin scope closes.
    Cleared,
}

/// Count of wrapper effects and output frames emitted by an action.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PluginHostUpdate {
    /// Number of lifecycle effects queued.
    pub emitted_effects: usize,
    /// Number of shell frames queued.
    pub emitted_frames: usize,
}
