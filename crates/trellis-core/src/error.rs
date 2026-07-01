use crate::{DeriveError, NodeId, OutputKey, ScopeId, TransactionId};
use core::fmt;

/// Result type used by graph metadata operations.
pub type GraphResult<T> = Result<T, GraphError>;

/// Top-level error category for deterministic failure handling.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ErrorCategory {
    /// Public API misuse or invalid graph references.
    ProgrammerError,
    /// User-defined derivation failed.
    DeriveError,
    /// User-defined resource planning failed.
    PlanError,
    /// User-defined output materialization failed.
    OutputError,
    /// Host-reported resource status, modeled as canonical input.
    HostResourceStatus,
}

/// Deterministic audit event for a failed transaction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ErrorAuditEvent {
    /// Error category.
    pub category: ErrorCategory,
    /// Stable target involved in the error.
    pub target: ErrorTarget,
}

/// Stable graph target involved in an error.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ErrorTarget {
    /// No narrower target exists.
    Graph,
    /// A node was involved.
    Node(NodeId),
    /// A scope was involved.
    Scope(ScopeId),
    /// A transaction was involved.
    Transaction(TransactionId),
    /// A materialized output was involved.
    Output(OutputKey),
}

/// User-defined resource planning failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlanError {
    /// Application-defined planning failure.
    Message(String),
}

impl PlanError {
    /// Creates an application-defined planning failure.
    pub fn message(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }
}

/// User-defined output materialization failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OutputError {
    /// A materializer read failed.
    Read(DeriveError),
    /// Application-defined output failure.
    Message(String),
}

impl OutputError {
    /// Creates an application-defined output failure.
    pub fn message(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }
}

impl From<DeriveError> for OutputError {
    fn from(error: DeriveError) -> Self {
        Self::Read(error)
    }
}

/// Host-observed resource status that can be written as canonical input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HostResourceStatus {
    /// The host has not reported a resource outcome.
    Unknown,
    /// The resource is live according to the host.
    Open,
    /// The resource failed outside graph propagation.
    Failed(String),
    /// The resource is closed according to the host.
    Closed,
}

impl HostResourceStatus {
    /// Returns the model category for host-reported resource status.
    pub const fn category(&self) -> ErrorCategory {
        ErrorCategory::HostResourceStatus
    }
}

/// Errors for graph metadata and input transaction operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GraphError {
    /// A node id is not present in the graph.
    UnknownNode(NodeId),
    /// A scope id is not present in the graph.
    UnknownScope(ScopeId),
    /// A dependency list contains the same node more than once.
    DuplicateDependency(NodeId),
    /// A node depends on itself.
    SelfDependency(NodeId),
    /// A node already has an owning scope.
    NodeAlreadyAttached(NodeId),
    /// A scope is closed and cannot accept new nodes.
    ScopeAlreadyClosed(ScopeId),
    /// A scope was already closed.
    ScopeClosed(ScopeId),
    /// A transaction is already open.
    NestedTransaction,
    /// A transaction was already closed and cannot be reused.
    TransactionClosed(TransactionId),
    /// A node is not an input node.
    NotInputNode(NodeId),
    /// A node is not a derived node.
    NotDerivedNode(NodeId),
    /// A node is not a collection node.
    NotCollectionNode(NodeId),
    /// An input write used the wrong value type for the node.
    WrongInputType(NodeId),
    /// A derived read used the wrong value type for the node.
    WrongDerivedType(NodeId),
    /// A collection read used the wrong key or value type for the node.
    WrongCollectionType(NodeId),
    /// An output key is not present in the graph.
    UnknownOutput(OutputKey),
    /// A materialized output computation failed.
    OutputFailed(OutputKey, OutputError),
    /// A resource planner failed.
    PlanFailed(ScopeId, PlanError),
    /// A resource command used a scope outside its registered planner scope.
    ResourceScopeMismatch(ScopeId),
    /// A resource command required an existing owned resource.
    ResourceNotOwned,
    /// A dependency cycle was detected.
    CycleDetected(NodeId),
    /// A scalar derived node declared a collection dependency.
    CollectionDependencyNotAllowed(NodeId),
    /// A pure derive function failed.
    DeriveFailed(NodeId, DeriveError),
    /// A pure collection function failed.
    CollectionFailed(NodeId, DeriveError),
    /// Incremental derived state differs from full recompute.
    FullRecomputeMismatch(NodeId),
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(id) => write!(f, "unknown node: {id:?}"),
            Self::UnknownScope(id) => write!(f, "unknown scope: {id:?}"),
            Self::DuplicateDependency(id) => write!(f, "duplicate dependency: {id:?}"),
            Self::SelfDependency(id) => write!(f, "self dependency: {id:?}"),
            Self::NodeAlreadyAttached(id) => write!(f, "node already attached: {id:?}"),
            Self::ScopeAlreadyClosed(id) => write!(f, "scope already closed: {id:?}"),
            Self::ScopeClosed(id) => write!(f, "scope already closed: {id:?}"),
            Self::NestedTransaction => write!(f, "a transaction is already open"),
            Self::TransactionClosed(id) => write!(f, "transaction already closed: {id:?}"),
            Self::NotInputNode(id) => write!(f, "node is not an input: {id:?}"),
            Self::NotDerivedNode(id) => write!(f, "node is not derived: {id:?}"),
            Self::NotCollectionNode(id) => write!(f, "node is not a collection: {id:?}"),
            Self::WrongInputType(id) => write!(f, "wrong input value type for node: {id:?}"),
            Self::WrongDerivedType(id) => write!(f, "wrong derived value type for node: {id:?}"),
            Self::WrongCollectionType(id) => {
                write!(f, "wrong collection value type for node: {id:?}")
            }
            Self::UnknownOutput(key) => write!(f, "unknown output: {key:?}"),
            Self::OutputFailed(key, error) => write!(f, "output failed for {key:?}: {error:?}"),
            Self::PlanFailed(scope, error) => {
                write!(f, "resource planner failed for {scope:?}: {error:?}")
            }
            Self::ResourceScopeMismatch(id) => write!(f, "resource scope mismatch: {id:?}"),
            Self::ResourceNotOwned => write!(f, "resource is not owned"),
            Self::CycleDetected(id) => write!(f, "dependency cycle detected at node: {id:?}"),
            Self::CollectionDependencyNotAllowed(id) => {
                write!(
                    f,
                    "collection dependency is not allowed for derived node: {id:?}"
                )
            }
            Self::DeriveFailed(id, error) => write!(f, "derive failed for {id:?}: {error:?}"),
            Self::CollectionFailed(id, error) => {
                write!(f, "collection failed for {id:?}: {error:?}")
            }
            Self::FullRecomputeMismatch(id) => {
                write!(f, "full recompute mismatch for node: {id:?}")
            }
        }
    }
}

impl GraphError {
    /// Returns this error's top-level category.
    pub const fn category(&self) -> ErrorCategory {
        match self {
            Self::DeriveFailed(_, _) | Self::CollectionFailed(_, _) => ErrorCategory::DeriveError,
            Self::PlanFailed(_, _) => ErrorCategory::PlanError,
            Self::OutputFailed(_, _) => ErrorCategory::OutputError,
            _ => ErrorCategory::ProgrammerError,
        }
    }

    /// Returns a deterministic audit event for this error.
    pub const fn audit_event(&self) -> ErrorAuditEvent {
        ErrorAuditEvent {
            category: self.category(),
            target: match self {
                Self::UnknownNode(node)
                | Self::DuplicateDependency(node)
                | Self::SelfDependency(node)
                | Self::NodeAlreadyAttached(node)
                | Self::NotInputNode(node)
                | Self::NotDerivedNode(node)
                | Self::NotCollectionNode(node)
                | Self::WrongInputType(node)
                | Self::WrongDerivedType(node)
                | Self::WrongCollectionType(node)
                | Self::CycleDetected(node)
                | Self::CollectionDependencyNotAllowed(node)
                | Self::DeriveFailed(node, _)
                | Self::CollectionFailed(node, _)
                | Self::FullRecomputeMismatch(node) => ErrorTarget::Node(*node),
                Self::UnknownScope(scope)
                | Self::ScopeAlreadyClosed(scope)
                | Self::ScopeClosed(scope)
                | Self::ResourceScopeMismatch(scope)
                | Self::PlanFailed(scope, _) => ErrorTarget::Scope(*scope),
                Self::TransactionClosed(transaction) => ErrorTarget::Transaction(*transaction),
                Self::UnknownOutput(output) | Self::OutputFailed(output, _) => {
                    ErrorTarget::Output(*output)
                }
                Self::NestedTransaction | Self::ResourceNotOwned => ErrorTarget::Graph,
            },
        }
    }
}

impl std::error::Error for GraphError {}
