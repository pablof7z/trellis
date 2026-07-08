use std::collections::{BTreeMap, BTreeSet};

/// Opaque handle for an open PipelineLab workspace.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct PipelineHandle(pub u64);

/// Current pipeline workspace inputs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PipelineSession {
    /// Active user id.
    pub user: String,
    /// Nodes with open preview panels.
    pub selected_nodes: BTreeSet<String>,
    /// Selected nodes currently hidden in the UI.
    pub hidden_nodes: BTreeSet<String>,
}

/// Kind-specific pipeline node metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PipelineNodeKind {
    /// External source node.
    Source {
        /// Source credential id.
        source_id: String,
    },
    /// Transform node with editable expression text.
    Transform {
        /// Transform expression.
        expression: String,
        /// Monotonic transform revision used in lineage keys.
        revision: u64,
    },
}

/// One app-owned pipeline graph node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PipelineNode {
    /// Stable node id.
    pub id: String,
    /// Display label.
    pub label: String,
    /// Node kind.
    pub kind: PipelineNodeKind,
    /// Upstream node ids.
    pub upstream: BTreeSet<String>,
}

/// App-owned pipeline graph.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PipelineGraphSpec {
    /// Nodes by stable id.
    pub nodes: BTreeMap<String, PipelineNode>,
}

/// Credential state owned by the host application.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CredentialStore {
    /// Source ids authorized by user id.
    pub sources_by_user: BTreeMap<String, BTreeSet<String>>,
}

/// Host job status modeled as canonical input.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum PipelineJobStatus {
    /// Job has not completed yet.
    #[default]
    Pending,
    /// Job is running.
    Running,
    /// Job completed successfully.
    Succeeded,
    /// Job failed with a host-provided reason.
    Failed(String),
}

/// Domain event applied to an open pipeline workspace.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PipelineLabEvent {
    /// Replace the selected preview panel set.
    SelectNodes(BTreeSet<String>),
    /// Hide one selected node preview panel.
    HideNode(String),
    /// Show one selected node preview panel.
    ShowNode(String),
    /// Edit one transform expression.
    EditTransform {
        /// Edited transform node id.
        node_id: String,
        /// Replacement expression.
        expression: String,
    },
    /// Revoke current-user credentials for one source.
    RevokeSourceCredential {
        /// Revoked source id.
        source_id: String,
    },
    /// Apply host job status as canonical input.
    ApplyJobStatus {
        /// Status target node id.
        node_id: String,
        /// Host status.
        status: PipelineJobStatus,
    },
    /// Replace the app-owned graph.
    ReplaceGraph(PipelineGraphSpec),
    /// Replace the credential store.
    ReplaceCredentials(CredentialStore),
}

/// Host resource controlled by PipelineLab.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum PipelineResource {
    /// External source connection.
    SourceConnection {
        /// Source credential id.
        source_id: String,
    },
    /// Panel preview query.
    PreviewQuery {
        /// Previewed node id.
        node_id: String,
        /// Lineage fingerprint for the preview.
        lineage_key: String,
    },
    /// Downstream compute job.
    ComputeJob {
        /// Computed node id.
        node_id: String,
        /// Lineage fingerprint for the job.
        lineage_key: String,
    },
}

/// Host command payload used by Trellis resource planning.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum PipelineCommand {
    /// Open the given pipeline resource.
    Open(PipelineResource),
}

/// Typed effect emitted to the pipeline host executor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PipelineEffect {
    /// Open the given resource.
    Open(PipelineResource),
    /// Close the given resource.
    Close(PipelineResource),
}

/// One graph node row in the materialized preview output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PipelineGraphNodeView {
    /// Node id.
    pub node_id: String,
    /// Display label.
    pub label: String,
    /// Display kind.
    pub kind: String,
    /// Whether the node is selected.
    pub selected: bool,
    /// Whether the preview panel is hidden.
    pub hidden: bool,
    /// Whether all required sources are authorized.
    pub authorized: bool,
}

/// One preview row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewRow {
    /// Row cells by column name.
    pub cells: BTreeMap<String, String>,
}

/// One materialized preview panel.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewPanel {
    /// Previewed node id.
    pub node_id: String,
    /// Display label.
    pub label: String,
    /// Lineage fingerprint for the preview.
    pub lineage_key: String,
    /// Current host job status for the preview.
    pub status: PipelineJobStatus,
    /// Bounded preview rows.
    pub rows: Vec<PreviewRow>,
}

/// Materialized PipelineLab output.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PipelineSnapshot {
    /// App graph rendered beside the Trellis trace.
    pub graph_nodes: Vec<PipelineGraphNodeView>,
    /// Visible preview panels.
    pub panels: Vec<PreviewPanel>,
}

impl PipelineSnapshot {
    /// Returns visible preview node ids in deterministic order.
    pub fn panel_node_ids(&self) -> BTreeSet<String> {
        self.panels
            .iter()
            .map(|panel| panel.node_id.clone())
            .collect()
    }
}

/// Public output frame emitted by the PipelineLab wrapper.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PipelineFrame {
    /// Initial baseline frame.
    Baseline(PipelineSnapshot),
    /// Incremental delta frame.
    Delta(PipelineSnapshot),
    /// Explicit rebaseline frame.
    Rebaseline(PipelineSnapshot),
    /// Clear frame emitted when the workspace closes.
    Cleared,
}

/// Count of wrapper effects and output frames emitted by an action.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PipelineLabUpdate {
    /// Number of pipeline lifecycle effects queued.
    pub emitted_effects: usize,
    /// Number of preview frames queued.
    pub emitted_frames: usize,
}
