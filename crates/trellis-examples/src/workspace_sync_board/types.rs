use std::collections::{BTreeMap, BTreeSet};

/// Board column visible in the workspace board.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum BoardColumn {
    /// Work not started.
    Todo,
    /// Work in progress.
    Doing,
    /// Work completed.
    Done,
}

/// Project metadata known to the local-first replica.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectRecord {
    /// Stable project id.
    pub id: String,
    /// Organization id.
    pub org: String,
    /// Workspace id.
    pub workspace: String,
    /// Display name.
    pub name: String,
    /// Whether the project is active.
    pub active: bool,
}

/// Cached issue row owned by one project.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IssueRecord {
    /// Stable issue id.
    pub id: String,
    /// Project id containing the issue.
    pub project_id: String,
    /// Display title.
    pub title: String,
    /// Board column.
    pub column: BoardColumn,
    /// Assigned user id.
    pub assignee: String,
}

/// Input data owned by the host application.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WorkspaceDataset {
    /// Project ids visible to each user.
    pub permissions: BTreeMap<String, BTreeSet<String>>,
    /// Project metadata by project id.
    pub projects: BTreeMap<String, ProjectRecord>,
    /// Cached issues by project id.
    pub issues: BTreeMap<String, Vec<IssueRecord>>,
}

/// Board mode selected by the host application.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BoardView {
    /// Organization workspace view.
    OrgWorkspace {
        /// Organization id.
        org: String,
        /// Workspace id.
        workspace: String,
        /// Whether inactive projects should be hidden.
        active_only: bool,
    },
    /// Personal assigned-to-me view.
    PersonalAssigned {
        /// User whose assigned issues are visible.
        user: String,
    },
}

/// Parameters for opening or switching a board.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceBoardParams {
    /// Active user id.
    pub user: String,
    /// Selected board view.
    pub view: BoardView,
    /// Visible board columns.
    pub visible_columns: BTreeSet<BoardColumn>,
}

impl WorkspaceBoardParams {
    /// Returns all board columns.
    pub fn all_columns() -> BTreeSet<BoardColumn> {
        [BoardColumn::Todo, BoardColumn::Doing, BoardColumn::Done]
            .into_iter()
            .collect()
    }
}

/// Domain event accepted by the board wrapper.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorkspaceBoardEvent {
    /// Switches the board to another view.
    SwitchView(WorkspaceBoardParams),
    /// Revokes one project from the current user.
    RevokeProjectPermission {
        /// Project id to remove from the active user's permission set.
        project_id: String,
    },
    /// Replaces the visible column set and asks for a rebaseline.
    SetVisibleColumns(BTreeSet<BoardColumn>),
    /// Replaces cached issues for one project.
    ReplaceProjectIssues {
        /// Project whose cached issues changed.
        project_id: String,
        /// New cached issues.
        issues: Vec<IssueRecord>,
    },
}

/// Opaque handle for the public board API.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct WorkspaceBoardHandle(pub(super) u64);

/// Host-visible sync target.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SyncTarget {
    /// Project sync window.
    Project {
        /// Project id to synchronize.
        project_id: String,
    },
    /// Comment sync window for a project.
    Comments {
        /// Project id whose comments should synchronize.
        project_id: String,
    },
    /// Assignee profile hydration window.
    Profile {
        /// User id whose profile should hydrate.
        user: String,
    },
}

/// Host effect emitted by the board wrapper.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SyncEffect {
    /// Open a sync target.
    Open(SyncTarget),
    /// Replace an existing sync target.
    Replace(SyncTarget),
    /// Close a sync target.
    Close(SyncTarget),
}

/// One issue row in a domain board frame.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct BoardRow {
    /// Issue id.
    pub issue_id: String,
    /// Project id.
    pub project_id: String,
    /// Issue title.
    pub title: String,
    /// Assignee user id.
    pub assignee: String,
}

/// Current materialized board state.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BoardSnapshot {
    /// Rows grouped by visible board column.
    pub columns: BTreeMap<BoardColumn, Vec<BoardRow>>,
}

impl BoardSnapshot {
    /// Returns true when the board has no visible rows.
    pub fn is_empty(&self) -> bool {
        self.columns.values().all(Vec::is_empty)
    }

    /// Returns all visible project ids in deterministic order.
    pub fn project_ids(&self) -> BTreeSet<String> {
        self.columns
            .values()
            .flat_map(|rows| rows.iter().map(|row| row.project_id.clone()))
            .collect()
    }
}

/// Public output frame emitted by the board wrapper.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BoardFrame {
    /// Complete current board state.
    Baseline(BoardSnapshot),
    /// Replacement board state after ordinary changes.
    Delta(BoardSnapshot),
    /// Complete current board state after explicit rebaseline.
    Rebaseline(BoardSnapshot),
    /// Terminal clear frame after board close.
    Cleared,
}

/// Summary returned by domain API methods.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct WorkspaceBoardUpdate {
    /// Number of sync effects queued by the method.
    pub emitted_effects: usize,
    /// Number of board frames queued by the method.
    pub emitted_frames: usize,
}

/// Private Trellis resource command payload for sync windows.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum SyncCommand {
    /// Open a sync target.
    Open(SyncTarget),
}
