use std::collections::BTreeSet;

use trellis_core::{DependencyList, Graph, InputNode, MaterializedOutput, ResourceKey, ScopeId};

use super::types::{
    PluginCommand, PluginManifest, PluginPermission, PluginResource, PluginShellSnapshot,
    WorkspaceKind,
};

pub(super) struct PluginHostGraph {
    pub(super) graph: Graph<PluginCommand>,
    pub(super) workspace: InputNode<WorkspaceKind>,
    pub(super) permissions: InputNode<BTreeSet<PluginPermission>>,
    pub(super) primary: PluginGraph,
    pub(super) secondary: PluginGraph,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(super) enum PluginSlot {
    Primary,
    Secondary,
}

pub(super) struct PluginGraph {
    pub(super) manifest: InputNode<Option<PluginManifest>>,
    pub(super) scope: ScopeId,
    pub(super) output: MaterializedOutput<PluginShellSnapshot>,
}

pub(super) fn build_graph(
    workspace_kind: WorkspaceKind,
    permissions: BTreeSet<PluginPermission>,
) -> PluginHostGraph {
    let mut graph = Graph::<PluginCommand>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let workspace = tx.input::<WorkspaceKind>("plugin-host-workspace").unwrap();
    tx.set_input(workspace, workspace_kind).unwrap();
    let permission_input = tx
        .input::<BTreeSet<PluginPermission>>("plugin-host-permissions")
        .unwrap();
    tx.set_input(permission_input, permissions).unwrap();
    let primary = add_plugin_graph(&mut tx, "primary-plugin", workspace, permission_input);
    let secondary = add_plugin_graph(&mut tx, "secondary-plugin", workspace, permission_input);
    tx.commit().unwrap();
    drop(tx);

    PluginHostGraph {
        graph,
        workspace,
        permissions: permission_input,
        primary,
        secondary,
    }
}

pub(super) fn resource_key(resource: &PluginResource) -> ResourceKey {
    match resource {
        PluginResource::Command { plugin_id, name } => ResourceKey::from_segments([
            "plugin-host",
            plugin_id.as_str(),
            "command",
            name.as_str(),
        ]),
        PluginResource::Panel { plugin_id, name } => {
            ResourceKey::from_segments(["plugin-host", plugin_id.as_str(), "panel", name.as_str()])
        }
        PluginResource::FileWatcher { plugin_id, glob } => ResourceKey::from_segments([
            "plugin-host",
            plugin_id.as_str(),
            "watcher",
            glob.as_str(),
        ]),
        PluginResource::BackgroundWorker { plugin_id, name } => {
            ResourceKey::from_segments(["plugin-host", plugin_id.as_str(), "worker", name.as_str()])
        }
        PluginResource::IpcChannel { plugin_id, name } => {
            ResourceKey::from_segments(["plugin-host", plugin_id.as_str(), "ipc", name.as_str()])
        }
    }
}

pub(super) fn resource_from_key(key: &ResourceKey) -> Option<PluginResource> {
    let segments = key.segments().collect::<Vec<_>>();
    match segments.as_slice() {
        ["plugin-host", plugin_id, "command", name] => Some(PluginResource::Command {
            plugin_id: (*plugin_id).to_owned(),
            name: (*name).to_owned(),
        }),
        ["plugin-host", plugin_id, "panel", name] => Some(PluginResource::Panel {
            plugin_id: (*plugin_id).to_owned(),
            name: (*name).to_owned(),
        }),
        ["plugin-host", plugin_id, "watcher", glob] => Some(PluginResource::FileWatcher {
            plugin_id: (*plugin_id).to_owned(),
            glob: (*glob).to_owned(),
        }),
        ["plugin-host", plugin_id, "worker", name] => Some(PluginResource::BackgroundWorker {
            plugin_id: (*plugin_id).to_owned(),
            name: (*name).to_owned(),
        }),
        ["plugin-host", plugin_id, "ipc", name] => Some(PluginResource::IpcChannel {
            plugin_id: (*plugin_id).to_owned(),
            name: (*name).to_owned(),
        }),
        _ => None,
    }
}

impl PluginHostGraph {
    pub(super) fn plugin(&self, slot: PluginSlot) -> &PluginGraph {
        match slot {
            PluginSlot::Primary => &self.primary,
            PluginSlot::Secondary => &self.secondary,
        }
    }
}

fn add_plugin_graph(
    tx: &mut trellis_core::Transaction<'_, PluginCommand>,
    name: &str,
    workspace: InputNode<WorkspaceKind>,
    permissions: InputNode<BTreeSet<PluginPermission>>,
) -> PluginGraph {
    let scope = tx.create_scope(name).unwrap();
    let manifest = tx
        .input::<Option<PluginManifest>>(format!("{name}-manifest"))
        .unwrap();
    tx.set_input(manifest, None).unwrap();

    let capabilities = tx
        .set_collection(
            format!("{name}-capabilities"),
            DependencyList::new([manifest.id(), workspace.id(), permissions.id()]).unwrap(),
            move |ctx| {
                Ok(capability_demand(
                    ctx.input(manifest)?,
                    ctx.input(workspace)?,
                    ctx.input(permissions)?,
                ))
            },
        )
        .unwrap();

    tx.open_close_planner(capabilities, scope, resource_key, |resource| {
        PluginCommand::Open(resource.clone())
    })
    .unwrap();

    let output = tx
        .materialized_output(
            format!("{name}-shell-output"),
            scope,
            DependencyList::new([manifest.id(), workspace.id(), capabilities.id()]).unwrap(),
            move |ctx| {
                Ok(shell_snapshot(
                    ctx.input(manifest)?,
                    ctx.input(workspace)?,
                    ctx.set_collection(capabilities)?,
                ))
            },
        )
        .unwrap();

    PluginGraph {
        manifest,
        scope,
        output,
    }
}

fn capability_demand(
    manifest: &Option<PluginManifest>,
    workspace: &WorkspaceKind,
    permissions: &BTreeSet<PluginPermission>,
) -> BTreeSet<PluginResource> {
    let Some(manifest) = manifest else {
        return BTreeSet::new();
    };
    if !manifest.compatible_workspaces.contains(workspace) {
        return BTreeSet::new();
    }

    let plugin_id = manifest.plugin_id.clone();
    let mut demand = BTreeSet::new();
    demand.extend(
        manifest
            .commands
            .iter()
            .cloned()
            .map(|name| PluginResource::Command {
                plugin_id: plugin_id.clone(),
                name,
            }),
    );
    demand.extend(
        manifest
            .panels
            .iter()
            .cloned()
            .map(|name| PluginResource::Panel {
                plugin_id: plugin_id.clone(),
                name,
            }),
    );
    if permissions.contains(&PluginPermission::FileSystemRead) {
        demand.extend(manifest.file_watchers.iter().cloned().map(|glob| {
            PluginResource::FileWatcher {
                plugin_id: plugin_id.clone(),
                glob,
            }
        }));
    }
    if permissions.contains(&PluginPermission::BackgroundWorker) {
        demand.extend(manifest.workers.iter().cloned().map(|name| {
            PluginResource::BackgroundWorker {
                plugin_id: plugin_id.clone(),
                name,
            }
        }));
    }
    if permissions.contains(&PluginPermission::Ipc) {
        demand.extend(manifest.ipc_channels.iter().cloned().map(|name| {
            PluginResource::IpcChannel {
                plugin_id: plugin_id.clone(),
                name,
            }
        }));
    }
    demand
}

fn shell_snapshot(
    manifest: &Option<PluginManifest>,
    workspace: &WorkspaceKind,
    capabilities: &BTreeSet<PluginResource>,
) -> PluginShellSnapshot {
    let mut snapshot = PluginShellSnapshot {
        plugin_id: manifest.as_ref().map(|manifest| manifest.plugin_id.clone()),
        workspace_kind: *workspace,
        ..PluginShellSnapshot::default()
    };
    for capability in capabilities {
        match capability {
            PluginResource::Command { name, .. } => {
                snapshot.commands.insert(name.clone());
            }
            PluginResource::Panel { name, .. } => {
                snapshot.panels.insert(name.clone());
            }
            PluginResource::FileWatcher { glob, .. } => {
                snapshot.file_watchers.insert(glob.clone());
            }
            PluginResource::BackgroundWorker { name, .. } => {
                snapshot.workers.insert(name.clone());
            }
            PluginResource::IpcChannel { name, .. } => {
                snapshot.ipc_channels.insert(name.clone());
            }
        }
    }
    snapshot
}
