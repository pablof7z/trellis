//! Proof examples built outside `trellis-core`.
//!
//! These examples intentionally keep domain vocabulary in this crate. The core
//! crate remains domain-neutral and only sees inputs, nodes, collections,
//! resource plans, scopes, transactions, and output frames.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Workspace-driven sync proof shape.
pub mod workspace_sync;

/// Mini language-server proof shape.
pub mod mini_language_server;

/// Telemetry dashboard proof shape.
pub mod telemetry_dashboard;
