//! Core graph skeleton for Trellis.
//!
//! This crate currently defines typed identities, graph metadata, scope
//! metadata, declared dependencies, deterministic inspection, input
//! transactions, pure derived node recomputation, collection diffs, and
//! data-only resource plans with recursive scope teardown and materialized
//! output frames. Transaction results include deterministic phase traces, and
//! failures expose typed categories.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod audit;
mod audit_types;
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
mod model;
mod node;
mod oracle;
mod output;
mod output_build;
mod output_reconcile;
mod read;
mod resource;
mod resource_build;
mod resource_reconcile;
mod scope;
mod scope_lifecycle;
mod trace;
mod transaction;
mod transaction_build;
mod transaction_types;

pub(crate) use audit_types::AuditState;
pub use audit_types::{
    NodeChangeExplanation, OutputFrameExplanation, ResourceCommandCause,
    ResourceCommandExplanation, ScopeResourceInventory,
};
pub use collection::CollectionContext;
pub use collection_diff::{Added, MapDiff, Removed, SetDiff, Unchanged, Updated};
pub use dependency::DependencyList;
pub use derive::{DeriveContext, DeriveError};
pub use error::{
    ErrorAuditEvent, ErrorCategory, ErrorTarget, GraphError, GraphResult, HostResourceStatus,
    OutputError, PlanError,
};
pub use graph::Graph;
pub use ids::{NodeId, OutputKey, Revision, ScopeId, TransactionId};
pub use model::{ModelGenerator, ModelScript, ModelStep, ModelTopology};
pub use node::{CollectionNode, DerivedNode, InputNode, NodeHandle, NodeKind, NodeMeta};
pub use oracle::FullRecomputeCheck;
pub use output::{
    ClearReason, MaterializedOutput, OutputContext, OutputFrame, OutputFrameKind, OutputMeta,
    OutputOptions, RebaselineReason,
};
pub use resource::{PlanContext, ResourceCommand, ResourceKey, ResourcePlan, ResourcePlanner};
pub use scope::ScopeMeta;
pub use trace::{
    OutputFrameKindTrace, OutputFrameTrace, ResourceCommandKind, ResourceCommandTrace,
    TraceMismatch, TransactionTrace, assert_transaction_traces_match,
};
pub use transaction::Transaction;
pub use transaction_types::{
    AuditEntry, AuditEvent, TransactionOptions, TransactionPhase, TransactionResult,
};
