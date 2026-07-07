use std::collections::{BTreeMap, BTreeSet};

use trellis_core::{DependencyList, Graph, InputNode, MaterializedOutput, ResourceKey, ScopeId};

use super::types::{
    BoardRow, BoardSnapshot, BoardView, IssueRecord, SyncCommand, SyncTarget, WorkspaceBoardParams,
    WorkspaceDataset,
};

pub(super) struct WorkspaceBoardGraph {
    pub(super) graph: Graph<SyncCommand>,
    pub(super) params: InputNode<Option<WorkspaceBoardParams>>,
    pub(super) dataset: InputNode<WorkspaceDataset>,
    pub(super) scope: ScopeId,
    pub(super) output: MaterializedOutput<BoardSnapshot>,
}

pub(super) fn build_graph(dataset: WorkspaceDataset) -> WorkspaceBoardGraph {
    let mut graph = Graph::<SyncCommand>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("workspace-board").unwrap();
    let params = tx
        .input::<Option<WorkspaceBoardParams>>("board-params")
        .unwrap();
    let dataset_input = tx.input::<WorkspaceDataset>("workspace-dataset").unwrap();
    tx.set_input(params, None).unwrap();
    tx.set_input(dataset_input, dataset).unwrap();

    let visible_projects = tx
        .set_collection(
            "visible-projects",
            DependencyList::new([params.id(), dataset_input.id()]).unwrap(),
            move |ctx| {
                Ok(visible_project_ids(
                    ctx.input(params)?,
                    ctx.input(dataset_input)?,
                ))
            },
        )
        .unwrap();

    let sync_targets = tx
        .set_collection(
            "sync-targets",
            DependencyList::new([visible_projects.id(), dataset_input.id()]).unwrap(),
            move |ctx| {
                Ok(sync_targets_for(
                    ctx.set_collection(visible_projects)?,
                    ctx.input(dataset_input)?,
                ))
            },
        )
        .unwrap();

    tx.open_close_planner(sync_targets, scope, sync_key, |target| {
        SyncCommand::Open(target.clone())
    })
    .unwrap();

    let output = tx
        .materialized_output(
            "workspace-board-output",
            scope,
            DependencyList::new([params.id(), dataset_input.id(), visible_projects.id()]).unwrap(),
            move |ctx| {
                Ok(board_snapshot(
                    ctx.input(params)?,
                    ctx.input(dataset_input)?,
                    ctx.set_collection(visible_projects)?,
                ))
            },
        )
        .unwrap();
    tx.commit().unwrap();
    drop(tx);

    WorkspaceBoardGraph {
        graph,
        params,
        dataset: dataset_input,
        scope,
        output,
    }
}

pub(super) fn sync_key(target: &SyncTarget) -> ResourceKey {
    match target {
        SyncTarget::Project { project_id } => {
            ResourceKey::from_segments(["sync", "project", project_id])
        }
        SyncTarget::Comments { project_id } => {
            ResourceKey::from_segments(["sync", "comments", project_id])
        }
        SyncTarget::Profile { user } => ResourceKey::from_segments(["sync", "profile", user]),
    }
}

pub(super) fn target_from_key(key: &ResourceKey) -> Option<SyncTarget> {
    let segments = key.segments().collect::<Vec<_>>();
    match segments.as_slice() {
        ["sync", "project", project_id] => Some(SyncTarget::Project {
            project_id: (*project_id).to_owned(),
        }),
        ["sync", "comments", project_id] => Some(SyncTarget::Comments {
            project_id: (*project_id).to_owned(),
        }),
        ["sync", "profile", user] => Some(SyncTarget::Profile {
            user: (*user).to_owned(),
        }),
        _ => None,
    }
}

fn visible_project_ids(
    params: &Option<WorkspaceBoardParams>,
    dataset: &WorkspaceDataset,
) -> BTreeSet<String> {
    let Some(params) = params else {
        return BTreeSet::new();
    };
    let allowed = dataset
        .permissions
        .get(&params.user)
        .cloned()
        .unwrap_or_default();

    dataset
        .projects
        .values()
        .filter(|project| allowed.contains(&project.id))
        .filter(|project| project_matches_view(project, &params.view, dataset))
        .map(|project| project.id.clone())
        .collect()
}

fn project_matches_view(
    project: &super::types::ProjectRecord,
    view: &BoardView,
    dataset: &WorkspaceDataset,
) -> bool {
    match view {
        BoardView::OrgWorkspace {
            org,
            workspace,
            active_only,
        } => {
            project.org == *org
                && project.workspace == *workspace
                && (!active_only || project.active)
        }
        BoardView::PersonalAssigned { user } => dataset
            .issues
            .get(&project.id)
            .is_some_and(|issues| issues.iter().any(|issue| issue.assignee == *user)),
    }
}

fn sync_targets_for(
    projects: &BTreeSet<String>,
    dataset: &WorkspaceDataset,
) -> BTreeSet<SyncTarget> {
    let mut targets = BTreeSet::new();
    for project_id in projects {
        targets.insert(SyncTarget::Project {
            project_id: project_id.clone(),
        });
        targets.insert(SyncTarget::Comments {
            project_id: project_id.clone(),
        });
        for assignee in assignees_for_project(project_id, dataset) {
            targets.insert(SyncTarget::Profile { user: assignee });
        }
    }
    targets
}

fn board_snapshot(
    params: &Option<WorkspaceBoardParams>,
    dataset: &WorkspaceDataset,
    projects: &BTreeSet<String>,
) -> BoardSnapshot {
    let Some(params) = params else {
        return BoardSnapshot::default();
    };
    let mut columns = params
        .visible_columns
        .iter()
        .cloned()
        .map(|column| (column, Vec::new()))
        .collect::<BTreeMap<_, _>>();

    for issue in visible_issues(params, dataset, projects) {
        if let Some(rows) = columns.get_mut(&issue.column) {
            rows.push(BoardRow {
                issue_id: issue.id.clone(),
                project_id: issue.project_id.clone(),
                title: issue.title.clone(),
                assignee: issue.assignee.clone(),
            });
        }
    }
    BoardSnapshot { columns }
}

fn visible_issues<'a>(
    params: &'a WorkspaceBoardParams,
    dataset: &'a WorkspaceDataset,
    projects: &'a BTreeSet<String>,
) -> impl Iterator<Item = &'a IssueRecord> {
    projects
        .iter()
        .filter_map(|project| dataset.issues.get(project))
        .flat_map(|issues| issues.iter())
        .filter(move |issue| match &params.view {
            BoardView::PersonalAssigned { user } => issue.assignee == *user,
            BoardView::OrgWorkspace { .. } => true,
        })
}

fn assignees_for_project(project_id: &str, dataset: &WorkspaceDataset) -> BTreeSet<String> {
    dataset
        .issues
        .get(project_id)
        .into_iter()
        .flatten()
        .map(|issue| issue.assignee.clone())
        .collect()
}
