use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use super::{ConformanceLevel, ConformanceReport, ConformanceSuite};

/// Result returned by one executable conformance check.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConformanceCheckResult {
    /// The check passed.
    Passed,
    /// The check was skipped because the app does not support the hook.
    Unsupported(String),
    /// The check ran and failed with scenario/trace/invariant detail.
    Failed(String),
}

impl ConformanceCheckResult {
    /// Creates a passed result.
    pub const fn passed() -> Self {
        Self::Passed
    }

    /// Creates an unsupported result with a reason.
    pub fn unsupported(reason: impl Into<String>) -> Self {
        Self::Unsupported(reason.into())
    }

    /// Creates a failed result with diagnostic detail.
    pub fn failed(detail: impl Into<String>) -> Self {
        Self::Failed(detail.into())
    }
}

/// Executed conformance check summary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConformanceCheckReport {
    /// Conformance level exercised by this check.
    pub level: ConformanceLevel,
    /// Stable invariant or scenario name.
    pub invariant: String,
    /// Result returned by the check.
    pub result: ConformanceCheckResult,
}

/// Failure returned by an executable conformance suite.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConformanceFailure {
    /// Conformance level that failed.
    pub level: ConformanceLevel,
    /// Stable invariant or scenario name that failed.
    pub invariant: String,
    /// Scenario, trace, or assertion detail from the failing check.
    pub detail: String,
}

impl fmt::Display for ConformanceFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} conformance failed for {}: {}",
            self.level, self.invariant, self.detail
        )
    }
}

impl std::error::Error for ConformanceFailure {}

/// Creates an empty executable conformance runner.
pub fn conformance() -> ConformanceRunner {
    ConformanceSuite::new().runner()
}

/// Executable opt-in conformance suite for application graphs.
pub struct ConformanceRunner {
    suite: ConformanceSuite,
    checks: Vec<ConformanceCheck>,
    unsupported: BTreeMap<ConformanceLevel, Vec<String>>,
}

impl ConformanceRunner {
    /// Creates a runner from a required-level suite.
    pub fn new(suite: ConformanceSuite) -> Self {
        Self {
            suite,
            checks: Vec::new(),
            unsupported: BTreeMap::new(),
        }
    }

    /// Requires a level even if no check has been registered yet.
    pub fn require(mut self, level: ConformanceLevel) -> Self {
        self.suite = self.suite.require(level);
        self
    }

    /// Registers an executable conformance check.
    pub fn check(
        mut self,
        level: ConformanceLevel,
        invariant: impl Into<String>,
        run: impl FnMut() -> ConformanceCheckResult + 'static,
    ) -> Self {
        self.suite = self.suite.require(level);
        self.checks.push(ConformanceCheck {
            level,
            invariant: invariant.into(),
            run: Box::new(run),
        });
        self
    }

    /// Marks a level unsupported with an explicit reason.
    pub fn unsupported(mut self, level: ConformanceLevel, reason: impl Into<String>) -> Self {
        self.suite = self.suite.require(level);
        self.unsupported
            .entry(level)
            .or_default()
            .push(reason.into());
        self
    }

    /// Runs all checks and returns supported/unsupported level reporting.
    pub fn run(mut self) -> Result<ConformanceReport, ConformanceFailure> {
        let mut report = ConformanceReport::new();
        let mut seen = BTreeSet::new();
        let mut unsupported = self.unsupported;

        for check in &mut self.checks {
            seen.insert(check.level);
            let result = (check.run)();
            match &result {
                ConformanceCheckResult::Passed => {}
                ConformanceCheckResult::Unsupported(reason) => {
                    unsupported
                        .entry(check.level)
                        .or_default()
                        .push(format!("{}: {reason}", check.invariant));
                }
                ConformanceCheckResult::Failed(detail) => {
                    return Err(ConformanceFailure {
                        level: check.level,
                        invariant: check.invariant.clone(),
                        detail: detail.clone(),
                    });
                }
            }
            report = report.record_check(ConformanceCheckReport {
                level: check.level,
                invariant: check.invariant.clone(),
                result,
            });
        }

        for level in self.suite.required_levels() {
            if let Some(reasons) = unsupported.remove(level) {
                for reason in reasons {
                    report = report.unsupported_with_reason(*level, reason);
                }
            } else if seen.contains(level) {
                report = report.support(*level);
            } else {
                report = report.unsupported_with_reason(
                    *level,
                    "no conformance check registered for required level",
                );
            }
        }
        Ok(report)
    }
}

struct ConformanceCheck {
    level: ConformanceLevel,
    invariant: String,
    run: Box<dyn FnMut() -> ConformanceCheckResult>,
}
