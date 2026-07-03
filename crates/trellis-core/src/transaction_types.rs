use crate::{
    NodeId, OutputFrame, ResourceCoalescedTrace, ResourceKey, ResourcePlan, Revision, ScopeId,
    TransactionId,
};

/// Configuration for committing input changes.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct TransactionOptions {
    /// When true, setting an input to an equal value does not advance revision.
    pub skip_equal_inputs: bool,
    /// Amount of graph-retained audit explanation state to update on commit.
    pub audit_explanations: AuditExplanationLevel,
}

impl Default for TransactionOptions {
    fn default() -> Self {
        Self {
            skip_equal_inputs: true,
            audit_explanations: AuditExplanationLevel::Summary,
        }
    }
}

impl TransactionOptions {
    /// Returns these options with equal input writes either skipped or emitted.
    pub fn with_skip_equal_inputs(mut self, skip_equal_inputs: bool) -> Self {
        self.skip_equal_inputs = skip_equal_inputs;
        self
    }

    /// Returns these options with a different audit explanation level.
    pub fn with_audit_explanations(mut self, level: AuditExplanationLevel) -> Self {
        self.audit_explanations = level;
        self
    }
}

/// Amount of latest audit explanation state retained on the graph.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum AuditExplanationLevel {
    /// Clear graph-retained explanations and skip explanation indexing.
    Disabled,
    /// Retain bounded latest node/resource/output summaries without dependency paths.
    Summary,
    /// Retain summaries plus shortest dependency paths from changed inputs.
    DependencyPaths,
}

/// Deterministic audit record for an input transaction.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AuditEntry {
    /// Transaction that produced this audit entry.
    pub transaction_id: TransactionId,
    /// Graph revision after the transaction committed.
    pub revision: Revision,
    /// Audited event.
    pub event: AuditEvent,
}

/// Deterministic transaction event emitted in the audit log.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AuditEvent {
    /// An input value changed or was configured to count as changed.
    InputChanged(NodeId),
    /// An equal input write was skipped by transaction options.
    InputUnchanged(NodeId),
    /// A derived value changed.
    DerivedChanged(NodeId),
    /// A collection value changed and produced a structural diff.
    CollectionChanged(NodeId),
    /// A scope was created.
    ScopeCreated(ScopeId),
    /// A scope was closed.
    ScopeClosed(ScopeId),
    /// A node was created.
    NodeCreated(NodeId),
    /// A node was attached to a scope.
    NodeAttached {
        /// Attached node.
        node: NodeId,
        /// Owning scope.
        scope: ScopeId,
    },
    /// An Open joined an already live resource with an equal payload.
    ResourceOpenCoalesced {
        /// Resource identity joined by the scope.
        key: ResourceKey,
        /// Scope that joined the existing resource.
        scope: ScopeId,
        /// Number of owners present before the join.
        existing_owner_count: usize,
    },
}

/// Test-observable transaction propagation phase.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TransactionPhase {
    /// Staged operations were accepted for commit processing.
    StageOperations,
    /// Staged operations and transaction failure state were validated.
    ValidateTransaction,
    /// Canonical input values were committed into the candidate graph.
    CommitCanonicalInputs,
    /// Dirty roots were identified from changed inputs and newly created nodes.
    MarkDirtyNodes,
    /// Dirty scalar derived nodes were recomputed.
    RecomputeDerivedNodes,
    /// Dirty collection nodes were recomputed and structural diffs were stored.
    RecomputeCollectionNodes,
    /// Late planner registration baselines were materialized as collection diffs.
    ComputeStructuralDiffs,
    /// Scope lifecycle changes were resolved for planning.
    ResolveScopeLifecycle,
    /// Resource plans were produced from final graph state.
    ProduceResourcePlans,
    /// Materialized output frames were produced from final graph state.
    ProduceOutputFrames,
    /// Graph revision and candidate state were committed.
    CommitGraphRevision,
    /// The transaction result was assembled and returned.
    ReturnTransactionResult,
}

/// A staged canonical input write accepted by a transaction.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StagedInputChange {
    /// Input node that was staged.
    pub node: NodeId,
    /// Whether the staged value changed committed state.
    pub outcome: StagedInputOutcome,
}

/// Test-observable outcome for a staged input write.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StagedInputOutcome {
    /// The staged write changed committed input state or was configured to count.
    Changed,
    /// The staged write was equal to committed state and skipped by options.
    Unchanged,
}

/// Payload-neutral structural summary for a collection diff.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CollectionDiffTrace {
    /// Collection node that produced the diff.
    pub node: NodeId,
    /// Collection shape that produced the diff.
    pub kind: CollectionDiffKind,
    /// Number of added members or entries.
    pub added: usize,
    /// Number of removed members or entries.
    pub removed: usize,
    /// Number of updated map entries.
    pub updated: usize,
    /// Number of unchanged members or entries.
    pub unchanged: usize,
}

/// Payload-neutral collection diff shape.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CollectionDiffKind {
    /// Set collection diff.
    Set,
    /// Map collection diff.
    Map,
}

/// Scope lifecycle event emitted by a transaction.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ScopeLifecycleTrace {
    /// Scope whose lifecycle changed.
    pub scope: ScopeId,
    /// Lifecycle transition that occurred.
    pub kind: ScopeLifecycleKind,
}

/// Test-observable scope lifecycle transition.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ScopeLifecycleKind {
    /// Scope was created.
    Created,
    /// Scope was closed.
    Closed,
}

/// Optional invariant result layered by testing support.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InvariantResultTrace {
    /// Stable invariant name.
    pub name: String,
    /// Whether the invariant passed.
    pub passed: bool,
}

/// Result returned by a committed input transaction.
#[derive(Clone, Debug, PartialEq)]
pub struct TransactionResult<C = ()> {
    /// Committed transaction id.
    pub transaction_id: TransactionId,
    /// Graph revision after commit.
    pub revision: Revision,
    /// Staged input writes in stable node-id order.
    pub staged_input_changes: Vec<StagedInputChange>,
    /// Input nodes that changed in stable node-id order.
    pub changed_inputs: Vec<NodeId>,
    /// Initial dirty roots in stable node-id order.
    pub dirty_roots: Vec<NodeId>,
    /// Derived nodes recomputed in deterministic topological order.
    pub recomputed_derived_nodes: Vec<NodeId>,
    /// Derived nodes that changed in deterministic topological order.
    pub changed_derived_nodes: Vec<NodeId>,
    /// Collection nodes recomputed in deterministic topological order.
    pub recomputed_collection_nodes: Vec<NodeId>,
    /// Collection nodes that changed in deterministic topological order.
    pub changed_collection_nodes: Vec<NodeId>,
    /// Payload-neutral collection diff summaries in stable node-id order.
    pub collection_diffs: Vec<CollectionDiffTrace>,
    /// Data-only resource commands produced by graph propagation.
    pub resource_plan: ResourcePlan<C>,
    /// Shared-key Open joins that produced no outgoing resource command.
    pub resource_coalescences: Vec<ResourceCoalescedTrace>,
    /// Data-only materialized output frames produced by graph propagation.
    pub output_frames: Vec<OutputFrame>,
    /// Scope lifecycle events emitted by this transaction.
    pub scope_events: Vec<ScopeLifecycleTrace>,
    /// Deterministic audit entries for this transaction.
    pub audit_log: Vec<AuditEntry>,
    /// Deterministic transaction phase trace.
    pub phase_trace: Vec<TransactionPhase>,
    /// Optional invariant results layered by test support.
    pub invariant_results: Vec<InvariantResultTrace>,
}
