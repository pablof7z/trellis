//! Companion testing helpers for Trellis.
//!
//! This crate is intentionally narrow. It helps tests inspect transaction
//! traces, resource lifecycle plans, materialized output frames, and
//! conformance support levels without executing host resources or replacing
//! Rust's existing testing ecosystem.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod conformance;
mod host_status;
mod output_ledger;
mod resource_ledger;
mod scenario;

pub use conformance::{ConformanceLevel, ConformanceReport};
pub use host_status::{HostStatusClass, HostStatusEvent, HostStatusRecord};
pub use output_ledger::{OutputLedger, OutputLedgerError, OutputSnapshot};
pub use resource_ledger::{ResourceLedger, ResourceLedgerError, ResourceSnapshot};
pub use scenario::{Scenario, ScenarioError, ScenarioStep};
