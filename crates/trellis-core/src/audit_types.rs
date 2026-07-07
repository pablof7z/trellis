use crate::{
    AuditEvent, AuditExplanationLevel, NodeId, OutputFrameKindTrace, OutputKey,
    ResourceCommandKind, ResourceKey, Revision, ScopeId, TransactionId,
};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct AuditState {
    pub(crate) node_changes: BTreeMap<NodeId, NodeChangeExplanation>,
    pub(crate) resource_commands: BTreeMap<ResourceKey, ResourceCommandExplanation>,
    pub(crate) output_frames: BTreeMap<OutputKey, OutputFrameExplanation>,
    pub(crate) pending_resource_causes: Vec<ResourceCommandCause>,
    pub(crate) pending_resource_coalescences: Vec<ResourceCoalescedTrace>,
}

impl AuditState {
    pub(crate) fn clear_explanations(&mut self) {
        self.node_changes.clear();
        self.resource_commands.clear();
        self.output_frames.clear();
        self.pending_resource_causes.clear();
        self.pending_resource_coalescences.clear();
    }
}

/// Explanation records retained by a single transaction receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditExplanations {
    /// Transaction that produced these explanations.
    pub transaction_id: TransactionId,
    /// Graph revision after the transaction committed.
    pub revision: Revision,
    /// Explanation depth requested for the transaction.
    pub level: AuditExplanationLevel,
    /// Node-change explanations keyed by node id.
    pub node_changes: BTreeMap<NodeId, NodeChangeExplanation>,
    /// Resource-command explanations keyed by resource key.
    pub resource_commands: BTreeMap<ResourceKey, ResourceCommandExplanation>,
    /// Output-frame explanations keyed by output key.
    pub output_frames: BTreeMap<OutputKey, OutputFrameExplanation>,
}

impl Default for AuditExplanations {
    fn default() -> Self {
        Self {
            transaction_id: TransactionId::default(),
            revision: Revision::default(),
            level: AuditExplanationLevel::Disabled,
            node_changes: BTreeMap::new(),
            resource_commands: BTreeMap::new(),
            output_frames: BTreeMap::new(),
        }
    }
}

impl AuditExplanations {
    /// Returns an empty explanation receipt for the requested transaction.
    pub(crate) fn with_level(
        transaction_id: TransactionId,
        revision: Revision,
        level: AuditExplanationLevel,
    ) -> Self {
        Self {
            transaction_id,
            revision,
            level,
            ..Self::default()
        }
    }
}

/// Serializable explanation records retained by a transaction trace.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AuditExplanationsTrace {
    /// Transaction that produced these explanations.
    pub transaction_id: TransactionId,
    /// Graph revision after the transaction committed.
    pub revision: Revision,
    /// Explanation depth requested for the transaction.
    pub level: AuditExplanationLevel,
    /// Node-change explanations in stable node-id order.
    pub node_changes: Vec<NodeChangeExplanation>,
    /// Resource-command explanations in stable resource-key order.
    pub resource_commands: Vec<ResourceCommandExplanation>,
    /// Output-frame explanations in stable output-key order.
    pub output_frames: Vec<OutputFrameExplanation>,
}

impl Default for AuditExplanationsTrace {
    fn default() -> Self {
        Self {
            transaction_id: TransactionId::default(),
            revision: Revision::default(),
            level: AuditExplanationLevel::Disabled,
            node_changes: Vec::new(),
            resource_commands: Vec::new(),
            output_frames: Vec::new(),
        }
    }
}

impl From<&AuditExplanations> for AuditExplanationsTrace {
    fn from(explanations: &AuditExplanations) -> Self {
        Self {
            transaction_id: explanations.transaction_id,
            revision: explanations.revision,
            level: explanations.level,
            node_changes: explanations.node_changes.values().cloned().collect(),
            resource_commands: explanations.resource_commands.values().cloned().collect(),
            output_frames: explanations.output_frames.values().cloned().collect(),
        }
    }
}

/// Explanation for why a node last changed.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NodeChangeExplanation {
    /// Node that changed.
    pub node: NodeId,
    /// Transaction that produced the change.
    pub transaction_id: TransactionId,
    /// Revision after the transaction committed.
    pub revision: Revision,
    /// Audit event that recorded the change.
    pub event: AuditEvent,
    /// Canonical input nodes that caused this node change, when path explanations are enabled.
    pub input_causes: Vec<NodeId>,
    /// Dependency paths from changed inputs to this node, when path explanations are enabled.
    pub dependency_paths: Vec<Vec<NodeId>>,
}

/// Explanation for the latest command emitted for a resource key.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    /// Canonical input causes for the command, when path explanations are enabled.
    pub input_causes: Vec<NodeId>,
    /// Dependency paths from input causes to collection diffs, when path explanations are enabled.
    pub dependency_paths: Vec<Vec<NodeId>>,
}

/// Graph-visible cause for an emitted resource command.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

/// Payload-neutral trace for a shared-key Open that joined an existing resource.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ResourceCoalescedTrace {
    /// Resource identity joined by the scope.
    pub key: ResourceKey,
    /// Scope that joined the existing resource.
    pub scope: ScopeId,
    /// Number of owners present before the join.
    pub existing_owner_count: usize,
}

/// Explanation for the latest frame emitted for an output key.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    /// Canonical input causes for the frame, when path explanations are enabled.
    pub input_causes: Vec<NodeId>,
    /// Dependency paths from input causes to changed output dependencies, when path explanations are enabled.
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
