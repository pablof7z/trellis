//! Core graph skeleton for Trellis.
//!
//! This crate currently defines typed identities, graph metadata, scope
//! metadata, declared dependencies, and deterministic inspection. It does not
//! implement propagation, transactions, resource plans, or materialized outputs.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod debug;
mod dependency;
mod error;
mod graph;
mod ids;
mod node;
mod scope;

pub use dependency::DependencyList;
pub use error::{GraphError, GraphResult};
pub use graph::Graph;
pub use ids::{NodeId, Revision, ScopeId, TransactionId};
pub use node::{CollectionNode, DerivedNode, InputNode, NodeHandle, NodeKind, NodeMeta};
pub use scope::ScopeMeta;
