//! Proof examples built outside `trellis-core`.
//!
//! These examples intentionally keep domain vocabulary in this crate. The core
//! crate remains domain-neutral and only sees inputs, nodes, collections,
//! resource plans, scopes, transactions, and output frames.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Workspace-driven sync proof shape.
pub mod workspace_sync;

/// Workspace Sync Board flagship showcase.
pub mod workspace_sync_board;

/// Shared headless showcase trace contract.
pub mod showcase_trace;

/// Shared seeded-bug capsule report contract.
pub mod seeded_bugs;

/// Mini language-server proof shape.
pub mod mini_language_server;

/// Telemetry dashboard proof shape.
pub mod telemetry_dashboard;

/// FleetPulse telemetry dashboard flagship showcase.
pub mod fleetpulse;

/// CollabCanvas document lifecycle secondary showcase.
pub mod collab_canvas;

/// PluginHost capability lifecycle secondary showcase.
pub mod plugin_host;

/// MarketDesk live market-data terminal secondary showcase.
pub mod market_desk;

/// PhotoStream smart-album hydrator secondary showcase.
pub mod photo_stream;

/// SearchOps live search/index dashboard secondary showcase.
pub mod search_ops;

/// PipelineLab visual data-pipeline previewer secondary showcase.
pub mod pipeline_lab;

/// Wrapper-friendly protocol subscription proof shape.
pub mod protocol_subscription;

/// Internal alpha acceptance coverage.
pub mod internal_alpha;
