//! Core graph skeleton for Trellis.
//!
//! This crate currently defines typed identities, graph metadata, scope
//! metadata, declared dependencies, deterministic inspection, input
//! transactions, pure derived node recomputation, collection diffs, and
//! data-only resource plans. It does not implement materialized outputs.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod collection;
mod collection_build;
mod collection_diff;
mod collection_recompute;
mod debug;
mod dependency;
mod derive;
mod error;
mod graph;
mod graph_support;
mod ids;
mod input;
mod node;
mod oracle;
mod read;
mod resource;
mod resource_build;
mod resource_reconcile;
mod scope;
mod transaction;
mod transaction_build;
mod transaction_types;

pub use collection::CollectionContext;
pub use collection_diff::{Added, MapDiff, Removed, SetDiff, Unchanged, Updated};
pub use dependency::DependencyList;
pub use derive::{DeriveContext, DeriveError};
pub use error::{GraphError, GraphResult};
pub use graph::Graph;
pub use ids::{NodeId, Revision, ScopeId, TransactionId};
pub use node::{CollectionNode, DerivedNode, InputNode, NodeHandle, NodeKind, NodeMeta};
pub use oracle::FullRecomputeCheck;
pub use resource::{PlanContext, ResourceCommand, ResourceKey, ResourcePlan, ResourcePlanner};
pub use scope::ScopeMeta;
pub use transaction::Transaction;
pub use transaction_types::{AuditEntry, AuditEvent, TransactionOptions, TransactionResult};
