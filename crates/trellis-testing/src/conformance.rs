use std::collections::BTreeSet;

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

/// Report of supported and unsupported conformance levels.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ConformanceReport {
    supported: BTreeSet<ConformanceLevel>,
    unsupported: BTreeSet<ConformanceLevel>,
}

impl ConformanceReport {
    /// Creates an empty report.
    pub fn new() -> Self {
        Self::default()
    }

    /// Marks a level as supported by the test target.
    pub fn support(mut self, level: ConformanceLevel) -> Self {
        self.unsupported.remove(&level);
        self.supported.insert(level);
        self
    }

    /// Marks a level as explicitly unsupported by the test target.
    pub fn unsupported(mut self, level: ConformanceLevel) -> Self {
        self.supported.remove(&level);
        self.unsupported.insert(level);
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
        for level in [
            ConformanceLevel::DeterministicTrace,
            ConformanceLevel::ScopeResourceLifecycle,
            ConformanceLevel::MaterializedOutput,
            ConformanceLevel::FullRecomputeOracle,
            ConformanceLevel::GeneratedModelSequences,
            ConformanceLevel::PerformanceSmoke,
        ] {
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
}

impl Default for ConformanceSuite {
    fn default() -> Self {
        Self::new()
    }
}
