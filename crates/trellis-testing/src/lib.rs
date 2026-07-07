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
mod harness;
mod harness_step;
mod host;
mod host_conformance;
mod host_status;
mod oracle;
mod output_ledger;
mod output_ledger_dump;
mod resource_assertions;
mod resource_error;
mod resource_ledger;
mod resource_ledger_dump;
mod resource_state;
mod scenario;
mod scenario_redaction;
mod script;
mod serialized;

pub use audit::{
    AuditAssertionError, OutputAuditContext, ResourceAuditContext, assert_dependency_path_exists,
    assert_no_unexplained_output_frame, assert_no_unexplained_plan,
};
pub use conformance::{
    ConformanceCheckReport, ConformanceCheckResult, ConformanceFailure, ConformanceLevel,
    ConformanceReport, ConformanceRunner, ConformanceSuite, conformance,
};
pub use harness::{ScenarioTarget, TrellisHarness};
pub use harness_step::HarnessStep;
pub use host::{FakeHost, FakeHostEvent};
pub use host_conformance::{
    HostConformanceError, HostConformanceLedger, HostEffectRecord, HostPlanRecord,
};
pub use host_status::{HostStatusClass, HostStatusEvent, HostStatusRecord};
pub use oracle::{
    FullRecomputeOracle, OracleCheck, OracleMismatch, assert_incremental_equals_full,
};
pub use output_ledger::{OutputLedger, OutputLedgerError, OutputSnapshot};
pub use resource_error::{ResourceCommandContext, ResourceLedgerError, ResourceStatusContext};
pub use resource_ledger::ResourceLedger;
pub use resource_state::{ResourceCommandRecord, ResourceSnapshot};
pub use scenario::{NoRedaction, Scenario, ScenarioError, ScenarioStep, TraceRedactor};
pub use script::{TransactionScript, TransactionScriptStep, TransactionScriptStepBuilder};
pub use serialized::{
    DataScriptStep, DataScriptStepBuilder, DataTransactionScript, SerializedScenario,
    SerializedScenarioStep, TRACE_FORMAT_VERSION,
};

pub(crate) use script::StageOperation;

#[cfg(feature = "proptest")]
pub mod proptest;
