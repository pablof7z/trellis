use crate::{
    AuditEntry, AuditEvent, NodeId, OutputFrameKindTrace, OutputKey, ResourceCommandKind,
    ResourceKey, Revision, ScopeId, TransactionId,
};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct AuditState {
    pub(crate) log: Vec<AuditEntry>,
    pub(crate) node_changes: BTreeMap<NodeId, NodeChangeExplanation>,
    pub(crate) resource_commands: BTreeMap<ResourceKey, ResourceCommandExplanation>,
    pub(crate) output_frames: BTreeMap<OutputKey, OutputFrameExplanation>,
    pub(crate) pending_resource_causes: Vec<ResourceCommandCause>,
}

/// Explanation for why a node last changed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeChangeExplanation {
    /// Node that changed.
    pub node: NodeId,
    /// Transaction that produced the change.
    pub transaction_id: TransactionId,
    /// Revision after the transaction committed.
    pub revision: Revision,
    /// Audit event that recorded the change.
    pub event: AuditEvent,
    /// Canonical input nodes that caused this node change.
    pub input_causes: Vec<NodeId>,
    /// Dependency paths from changed inputs to this node.
    pub dependency_paths: Vec<Vec<NodeId>>,
}

/// Explanation for the latest command emitted for a resource key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceCommandExplanation {
    /// Resource key.
    pub key: ResourceKey,
    /// Scope associated with the command.
    pub scope: ScopeId,
    /// Transaction that emitted the command.
    pub transaction_id: TransactionId,
    /// Revision carried by the transaction.
    pub revision: Revision,
    /// Command operation without application payload.
    pub kind: ResourceCommandKind,
    /// Graph cause that produced the command.
    pub cause: ResourceCommandCause,
    /// Collection diffs consumed by the command's planner, if any.
    pub collection_diffs: Vec<NodeId>,
    /// Nodes changed in the transaction.
    pub changed_nodes: Vec<NodeId>,
    /// Canonical input causes for the command.
    pub input_causes: Vec<NodeId>,
    /// Dependency paths from input causes to collection diffs.
    pub dependency_paths: Vec<Vec<NodeId>>,
}

/// Graph-visible cause for an emitted resource command.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ResourceCommandCause {
    /// A resource planner ran because a collection diff was available.
    Planner {
        /// Collection whose diff was consumed.
        collection: NodeId,
    },
    /// Scope teardown removed ownership.
    ScopeClosed {
        /// Scope being closed.
        scope: ScopeId,
    },
}

/// Explanation for the latest frame emitted for an output key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputFrameExplanation {
    /// Output key.
    pub output_key: OutputKey,
    /// Scope that owned the output frame.
    pub scope: ScopeId,
    /// Transaction that emitted the frame.
    pub transaction_id: TransactionId,
    /// Revision carried by the frame.
    pub revision: Revision,
    /// Frame kind without materialized payload.
    pub kind: OutputFrameKindTrace,
    /// Declared dependencies for the output, when still live.
    pub dependencies: Vec<NodeId>,
    /// Output dependencies that changed in the transaction.
    pub changed_dependencies: Vec<NodeId>,
    /// Canonical input causes for the frame.
    pub input_causes: Vec<NodeId>,
    /// Dependency paths from input causes to changed output dependencies.
    pub dependency_paths: Vec<Vec<NodeId>>,
}

/// Deterministic resource inventory for a scope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScopeResourceInventory {
    /// Scope being inspected.
    pub scope: ScopeId,
    /// Resources currently owned by the scope.
    pub resources: Vec<ResourceKey>,
}
