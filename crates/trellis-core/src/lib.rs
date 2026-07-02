//! Core graph skeleton for Trellis.
//!
//! This crate currently defines typed identities, graph metadata, scope
//! metadata, declared dependencies, deterministic inspection, input
//! transactions, pure derived node recomputation, collection diffs, and
//! data-only resource plans with recursive scope teardown and materialized
//! output frames. Transaction results include deterministic phase traces, and
//! failures expose typed categories.
//!
//! # API stability
//!
//! Trellis is pre-1.0. Core semantics are intended to be more stable than item
//! names and exact signatures: resource plans are data, graph mutation is
//! transactional, dependencies are explicit, scopes own lifecycle, outputs are
//! revisioned, and incremental behavior must remain checkable against full
//! recompute.

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
mod dependency_validate;
mod derive;
mod error;
mod graph;
mod graph_support;
mod host_status;
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
mod topology;
mod trace;
mod transaction;
mod transaction_build;
mod transaction_trace_build;
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
    ErrorAuditEvent, ErrorCategory, ErrorTarget, FullRecomputeOutputMismatch,
    FullRecomputeResourceMismatch, GraphError, GraphResult, OutputError, PlanError,
};
pub use graph::Graph;
pub use host_status::{HostResourceOutcome, HostResourceStatus};
pub use ids::{NodeId, OutputKey, Revision, ScopeId, TransactionId};
pub use node::{CollectionNode, DerivedNode, InputNode, NodeHandle, NodeKind, NodeMeta};
pub use oracle::FullRecomputeCheck;
pub use output::{
    ClearReason, MaterializedOutput, OutputContext, OutputFrame, OutputFrameKind, OutputMeta,
    OutputOptions, RebaselineReason,
};
pub use resource::{PlanContext, ResourceCommand, ResourceKey, ResourcePlan};
pub use scope::ScopeMeta;
pub use trace::{
    OutputFrameKindTrace, OutputFrameTrace, ResourceCommandKind, ResourceCommandTrace,
    ResourceTransitionPolicy, TraceMismatch, TransactionTrace, assert_transaction_traces_match,
};
pub use transaction::Transaction;
pub use transaction_types::{
    AuditEntry, AuditEvent, CollectionDiffKind, CollectionDiffTrace, InvariantResultTrace,
    ScopeLifecycleKind, ScopeLifecycleTrace, StagedInputChange, StagedInputOutcome,
    TransactionOptions, TransactionPhase, TransactionResult,
};

/// Deterministic model-test helpers for oracle and replay checks.
pub mod testing {
    pub use crate::model::{ModelGenerator, ModelScript, ModelStep, ModelTopology};
}
