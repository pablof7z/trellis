//! Core graph skeleton for Trellis.
//!
//! This crate currently defines typed identities, graph metadata, scope
//! metadata, declared dependencies, deterministic inspection, input
//! transactions, and pure derived node recomputation. It does not implement
//! collection diffs, resource plans, or materialized outputs.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod debug;
mod dependency;
mod derive;
mod error;
mod graph;
mod ids;
mod input;
mod node;
mod read;
mod scope;
mod transaction;
mod transaction_build;
mod transaction_types;

pub use dependency::DependencyList;
pub use derive::{DeriveContext, DeriveError, FullRecomputeCheck};
pub use error::{GraphError, GraphResult};
pub use graph::Graph;
pub use ids::{NodeId, Revision, ScopeId, TransactionId};
pub use node::{CollectionNode, DerivedNode, InputNode, NodeHandle, NodeKind, NodeMeta};
pub use scope::ScopeMeta;
pub use transaction::Transaction;
pub use transaction_types::{AuditEntry, AuditEvent, TransactionOptions, TransactionResult};
