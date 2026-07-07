//! Workspace Sync Board flagship showcase.

mod engine;
mod graph;
mod sample;
mod scripts;
mod types;

#[cfg(test)]
mod tests;

pub use engine::WorkspaceBoardApp;
pub use scripts::switch_workspace_showcase_trace;
pub use types::{
    BoardColumn, BoardFrame, BoardRow, BoardSnapshot, BoardView, IssueRecord, ProjectRecord,
    SyncEffect, SyncTarget, WorkspaceBoardEvent, WorkspaceBoardHandle, WorkspaceBoardParams,
    WorkspaceBoardUpdate, WorkspaceDataset,
};
