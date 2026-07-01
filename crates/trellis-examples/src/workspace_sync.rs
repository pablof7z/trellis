use std::collections::{BTreeMap, BTreeSet};

use trellis_core::{DependencyList, Graph, ResourceKey, ResourcePlan};

/// Host command payload for workspace sync windows.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SyncCommand {
    /// Open a sync window for a project.
    OpenWindow(String),
}

fn key(project: &str) -> ResourceKey {
    ResourceKey::new(format!("sync:{project}"))
}

#[cfg(test)]
fn set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[cfg(test)]
fn grants(entries: &[(&str, &[&str])]) -> BTreeMap<String, BTreeSet<String>> {
    entries
        .iter()
        .map(|(workspace, projects)| ((*workspace).to_owned(), set(projects)))
        .collect()
}

fn board(projects: &BTreeSet<String>) -> Vec<String> {
    projects
        .iter()
        .map(|project| format!("{project}:ready"))
        .collect()
}

/// Built workspace-sync example graph and its public inputs.
pub struct WorkspaceSyncExample {
    /// Example graph.
    pub graph: Graph<SyncCommand, Vec<String>>,
    /// Active workspace canonical input.
    pub active_workspace: trellis_core::InputNode<Option<String>>,
    /// Permitted projects canonical input.
    pub permitted_projects: trellis_core::InputNode<BTreeMap<String, BTreeSet<String>>>,
}

/// Builds the workspace-driven sync proof graph.
pub fn build_graph(
    active: Option<&str>,
    initial_grants: BTreeMap<String, BTreeSet<String>>,
) -> WorkspaceSyncExample {
    let mut graph = Graph::<SyncCommand, Vec<String>>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("workspace").unwrap();
    let active_workspace = tx.input::<Option<String>>("active-workspace").unwrap();
    let permitted = tx
        .input::<BTreeMap<String, BTreeSet<String>>>("permitted-projects")
        .unwrap();
    tx.set_input(active_workspace, active.map(str::to_owned))
        .unwrap();
    tx.set_input(permitted, initial_grants).unwrap();
    let projects = tx
        .derived(
            "project-set",
            DependencyList::new([active_workspace.id(), permitted.id()]).unwrap(),
            move |ctx| {
                let active = ctx.input(active_workspace)?;
                let permitted = ctx.input(permitted)?;
                Ok(active
                    .as_ref()
                    .and_then(|workspace| permitted.get(workspace))
                    .cloned()
                    .unwrap_or_default())
            },
        )
        .unwrap();
    let windows = tx
        .set_collection(
            "sync-window-set",
            DependencyList::new([projects.id()]).unwrap(),
            move |ctx| Ok(ctx.derived(projects)?.clone()),
        )
        .unwrap();
    tx.set_resource_planner(windows, scope, move |ctx| {
        let mut plan = ResourcePlan::new();
        for added in &ctx.diff().added {
            plan.open(
                key(&added.value),
                ctx.scope(),
                SyncCommand::OpenWindow(added.value.clone()),
            );
        }
        for removed in &ctx.diff().removed {
            plan.close(key(&removed.value), ctx.scope());
        }
        Ok(plan)
    })
    .unwrap();
    tx.materialized_output(
        "issue-board",
        scope,
        DependencyList::new([projects.id()]).unwrap(),
        move |ctx| Ok(board(ctx.derived(projects)?)),
    )
    .unwrap();
    tx.commit().unwrap();
    drop(tx);
    WorkspaceSyncExample {
        graph,
        active_workspace,
        permitted_projects: permitted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use trellis_core::{OutputFrameKind, ResourceCommand};

    #[test]
    fn workspace_switch_closes_old_windows() {
        let mut example = build_graph(
            Some("one"),
            grants(&[("one", &["a", "b"]), ("two", &["c"])]),
        );

        let mut tx = example.graph.begin_transaction().unwrap();
        tx.set_input(example.active_workspace, Some("two".to_owned()))
            .unwrap();
        let result = tx.commit().unwrap();
        drop(tx);

        assert!(
            result
                .resource_plan
                .commands()
                .contains(&ResourceCommand::Close {
                    key: key("a"),
                    scope: result.resource_plan.commands()[0].scope(),
                })
        );
        assert!(result.resource_plan.commands().iter().any(|command| {
            matches!(command, ResourceCommand::Open { key: resource_key, .. } if resource_key == &key("c"))
        }));
        example.graph.assert_incremental_equals_full().unwrap();
    }

    #[test]
    fn permission_revoke_clears_forbidden_rows() {
        let mut example = build_graph(Some("one"), grants(&[("one", &["a", "b"])]));

        let mut tx = example.graph.begin_transaction().unwrap();
        tx.set_input(example.permitted_projects, grants(&[("one", &["a"])]))
            .unwrap();
        let result = tx.commit().unwrap();
        drop(tx);

        assert!(result.resource_plan.commands().iter().any(|command| {
            matches!(command, ResourceCommand::Close { key: resource_key, .. } if resource_key == &key("b"))
        }));
        assert!(matches!(
            &result.output_frames[0].kind,
            OutputFrameKind::Delta(rows) if rows == &vec!["a:ready".to_owned()]
        ));
        example.graph.assert_incremental_equals_full().unwrap();
    }

    #[test]
    fn empty_workspace_opens_no_windows() {
        let example = build_graph(None, grants(&[("one", &["a"])]));

        assert!(example.graph.resource_owners(&key("a")).is_none());
        example.graph.assert_incremental_equals_full().unwrap();
    }
}
