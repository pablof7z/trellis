use crate::{NodeId, ScopeId, TransactionId};
use core::fmt;

/// Result type used by graph metadata operations.
pub type GraphResult<T> = Result<T, GraphError>;

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
    /// A transaction is already open.
    NestedTransaction,
    /// A transaction was already closed and cannot be reused.
    TransactionClosed(TransactionId),
    /// A node is not an input node.
    NotInputNode(NodeId),
    /// An input write used the wrong value type for the node.
    WrongInputType(NodeId),
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
            Self::NestedTransaction => write!(f, "a transaction is already open"),
            Self::TransactionClosed(id) => write!(f, "transaction already closed: {id:?}"),
            Self::NotInputNode(id) => write!(f, "node is not an input: {id:?}"),
            Self::WrongInputType(id) => write!(f, "wrong input value type for node: {id:?}"),
        }
    }
}

impl std::error::Error for GraphError {}
