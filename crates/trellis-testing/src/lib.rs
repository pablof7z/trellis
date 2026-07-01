//! Companion testing helpers for Trellis.
//!
//! This crate is intentionally narrow. It helps tests inspect transaction
//! traces, resource lifecycle plans, materialized output frames, and
//! conformance support levels without executing host resources or replacing
//! Rust's existing testing ecosystem.
//!
//! The Cargo package is named `trellis-testing` to avoid the crates.io
//! normalized-name collision with `trellis_test`.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod audit;
mod conformance;
mod host;
mod host_status;
mod oracle;
mod output_ledger;
mod resource_error;
mod resource_ledger;
mod scenario;

pub use audit::{
    AuditAssertionError, assert_dependency_path_exists, assert_every_output_frame_has_revision,
    assert_every_output_frame_has_scope, assert_every_resource_command_has_cause,
    assert_no_unexplained_output_frame, assert_no_unexplained_plan,
};
pub use conformance::{ConformanceLevel, ConformanceReport, ConformanceSuite};
pub use host::{FakeHost, FakeHostEvent};
pub use host_status::{HostStatusClass, HostStatusEvent, HostStatusRecord};
pub use oracle::{
    FullRecomputeOracle, OracleCheck, OracleMismatch, assert_incremental_equals_full,
};
pub use output_ledger::{OutputLedger, OutputLedgerError, OutputSnapshot};
pub use resource_error::ResourceLedgerError;
pub use resource_ledger::{ResourceLedger, ResourceSnapshot};
pub use scenario::{NoRedaction, Scenario, ScenarioError, ScenarioStep, TraceRedactor};

#[cfg(feature = "proptest")]
pub mod proptest;
