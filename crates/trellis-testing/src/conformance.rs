use std::collections::{BTreeMap, BTreeSet};

mod runner;

pub use runner::{
    ConformanceCheckReport, ConformanceCheckResult, ConformanceFailure, ConformanceRunner,
    conformance,
};

/// Opt-in conformance levels for application graph tests.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ConformanceLevel {
    /// Deterministic transaction trace and phase order.
    DeterministicTrace = 1,
    /// Scope and resource lifecycle.
    ScopeResourceLifecycle = 2,
    /// Materialized output coherence.
    MaterializedOutput = 3,
    /// Full-recompute oracle equivalence.
    FullRecomputeOracle = 4,
    /// Generated/model sequence checks.
    GeneratedModelSequences = 5,
    /// Performance/allocation smoke checks.
    PerformanceSmoke = 6,
}

impl ConformanceLevel {
    /// All currently defined conformance levels in ascending order.
    pub const ALL: [Self; 6] = [
        Self::DeterministicTrace,
        Self::ScopeResourceLifecycle,
        Self::MaterializedOutput,
        Self::FullRecomputeOracle,
        Self::GeneratedModelSequences,
        Self::PerformanceSmoke,
    ];
}

/// Report of supported and unsupported conformance levels.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ConformanceReport {
    supported: BTreeSet<ConformanceLevel>,
    unsupported: BTreeSet<ConformanceLevel>,
    unsupported_reasons: BTreeMap<ConformanceLevel, Vec<String>>,
    checks: Vec<ConformanceCheckReport>,
}

impl ConformanceReport {
    /// Creates an empty report.
    pub fn new() -> Self {
        Self::default()
    }

    /// Marks a level as supported by the test target.
    pub fn support(mut self, level: ConformanceLevel) -> Self {
        self.unsupported.remove(&level);
        self.unsupported_reasons.remove(&level);
        self.supported.insert(level);
        self
    }

    /// Marks a level as explicitly unsupported by the test target.
    pub fn unsupported(mut self, level: ConformanceLevel) -> Self {
        self.unsupported_reasons
            .entry(level)
            .or_default()
            .push("explicitly unsupported".to_owned());
        self.supported.remove(&level);
        self.unsupported.insert(level);
        self
    }

    /// Marks a level as explicitly unsupported by the target with a reason.
    pub fn unsupported_with_reason(
        mut self,
        level: ConformanceLevel,
        reason: impl Into<String>,
    ) -> Self {
        self.supported.remove(&level);
        self.unsupported.insert(level);
        self.unsupported_reasons
            .entry(level)
            .or_default()
            .push(reason.into());
        self
    }

    /// Records an executed check in this report.
    pub fn record_check(mut self, check: ConformanceCheckReport) -> Self {
        self.checks.push(check);
        self
    }

    /// Returns supported levels.
    pub fn supported_levels(&self) -> &BTreeSet<ConformanceLevel> {
        &self.supported
    }

    /// Returns explicitly unsupported levels.
    pub fn unsupported_levels(&self) -> &BTreeSet<ConformanceLevel> {
        &self.unsupported
    }

    /// Returns unsupported reasons by conformance level.
    pub fn unsupported_reasons(&self) -> &BTreeMap<ConformanceLevel, Vec<String>> {
        &self.unsupported_reasons
    }

    /// Returns executed check summaries.
    pub fn check_results(&self) -> &[ConformanceCheckReport] {
        &self.checks
    }

    /// Returns true if a level is supported.
    pub fn supports(&self, level: ConformanceLevel) -> bool {
        self.supported.contains(&level)
    }
}

/// Opt-in set of conformance levels an application wants to exercise.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConformanceSuite {
    required: BTreeSet<ConformanceLevel>,
}

impl ConformanceSuite {
    /// Creates an empty conformance suite.
    pub fn new() -> Self {
        Self {
            required: BTreeSet::new(),
        }
    }

    /// Creates a suite requiring all currently defined levels.
    pub fn all() -> Self {
        let mut suite = Self::new();
        for level in ConformanceLevel::ALL {
            suite = suite.require(level);
        }
        suite
    }

    /// Adds a required level to this suite.
    pub fn require(mut self, level: ConformanceLevel) -> Self {
        self.required.insert(level);
        self
    }

    /// Returns required levels.
    pub fn required_levels(&self) -> &BTreeSet<ConformanceLevel> {
        &self.required
    }

    /// Builds a report, marking required but unsupported levels explicitly.
    pub fn report(&self, supported: &[ConformanceLevel]) -> ConformanceReport {
        let mut report = ConformanceReport::new();
        let supported = supported.iter().copied().collect::<BTreeSet<_>>();
        for level in &self.required {
            if supported.contains(level) {
                report = report.support(*level);
            } else {
                report = report.unsupported(*level);
            }
        }
        report
    }

    /// Creates an executable runner for this conformance suite.
    pub fn runner(self) -> ConformanceRunner {
        ConformanceRunner::new(self)
    }
}

impl Default for ConformanceSuite {
    fn default() -> Self {
        Self::new()
    }
}
