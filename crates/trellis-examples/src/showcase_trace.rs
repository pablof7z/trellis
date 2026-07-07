//! Shared headless showcase trace contract.

use trellis_core::{
    Graph, InvariantResultTrace, TransactionResult, TransactionTrace,
    assert_transaction_traces_match,
};

/// Stable format version for headless showcase trace JSON.
pub const SHOWCASE_TRACE_FORMAT_VERSION: u32 = 1;

/// Stable contract identifier for headless showcase trace JSON.
pub const SHOWCASE_TRACE_CONTRACT: &str = "trellis.showcase.trace";

/// One complete headless showcase script run.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ShowcaseTrace {
    /// Stable contract identifier.
    pub contract: String,
    /// Showcase trace format version.
    pub format_version: u32,
    /// Human-readable showcase name.
    pub showcase: String,
    /// Stable script name passed through `--script`.
    pub script: String,
    /// Command that reproduces this run.
    pub command: Vec<String>,
    /// Deterministic replay comparison metadata.
    pub replay: ReplayMetadata,
    /// Seeded-bug status for the same contract shape.
    pub seeded_bug: SeededBugStatus,
    /// Named transaction steps emitted by the script.
    pub steps: Vec<ShowcaseStep>,
}

/// One named transaction step in a headless showcase script.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ShowcaseStep {
    /// Stable step name.
    pub name: String,
    /// Host statuses observed or applied by the script step.
    pub host_statuses: Vec<ShowcaseHostStatus>,
    /// Payload-neutral transaction trace for the committed step.
    pub trace: TransactionTrace,
}

/// Host status metadata attached to one showcase step.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ShowcaseHostStatus {
    /// Domain resource or effect target.
    pub target: String,
    /// Domain status label.
    pub status: String,
    /// Optional command revision the status acknowledges.
    pub command_revision: Option<u64>,
}

/// Deterministic replay comparison metadata.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ReplayMetadata {
    /// Replay status, usually `passed`.
    pub status: String,
    /// Number of fresh runs compared.
    pub compared_runs: u32,
    /// Failure reason when replay did not pass.
    pub reason: Option<String>,
}

/// Seeded-bug status attached to a showcase trace.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct SeededBugStatus {
    /// Seeded-bug status for this script.
    pub status: String,
    /// Tracking issue for seeded-bug capsules.
    pub issue: String,
    /// Human-readable reason.
    pub reason: String,
}

/// Builds a showcase trace and compares it to a fresh deterministic rerun.
pub fn build_showcase_trace(
    showcase: &str,
    script: &str,
    command: &[&str],
    run_once: impl Fn() -> Vec<ShowcaseStep>,
) -> ShowcaseTrace {
    let steps = run_once();
    let replay_steps = run_once();
    let replay = replay_metadata(&steps, &replay_steps);

    ShowcaseTrace {
        contract: SHOWCASE_TRACE_CONTRACT.to_owned(),
        format_version: SHOWCASE_TRACE_FORMAT_VERSION,
        showcase: showcase.to_owned(),
        script: script.to_owned(),
        command: command.iter().map(|part| (*part).to_owned()).collect(),
        replay,
        seeded_bug: SeededBugStatus {
            status: "not_included".to_owned(),
            issue: "#93".to_owned(),
            reason: "seeded bug capsules are tracked separately".to_owned(),
        },
        steps,
    }
}

/// Builds a showcase step from a committed result and records oracle status.
pub fn step_with_oracle<C>(
    name: &str,
    graph: &Graph<C>,
    result: &TransactionResult<C>,
) -> ShowcaseStep
where
    C: Clone + PartialEq,
{
    let mut trace = result.trace();
    trace.invariant_results.push(InvariantResultTrace {
        name: "incremental_equals_full_recompute".to_owned(),
        passed: graph.full_recompute_check().is_ok(),
    });
    ShowcaseStep {
        name: name.to_owned(),
        host_statuses: Vec::new(),
        trace,
    }
}

/// Serializes a showcase trace with deterministic pretty JSON formatting.
pub fn to_pretty_json(trace: &ShowcaseTrace) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(trace)
}

fn replay_metadata(first: &[ShowcaseStep], second: &[ShowcaseStep]) -> ReplayMetadata {
    let first_traces = first
        .iter()
        .map(|step| step.trace.clone())
        .collect::<Vec<_>>();
    let second_traces = second
        .iter()
        .map(|step| step.trace.clone())
        .collect::<Vec<_>>();

    match assert_transaction_traces_match(&first_traces, &second_traces) {
        Ok(()) => ReplayMetadata {
            status: "passed".to_owned(),
            compared_runs: 2,
            reason: None,
        },
        Err(error) => ReplayMetadata {
            status: "failed".to_owned(),
            compared_runs: 2,
            reason: Some(format!("{error:?}")),
        },
    }
}
