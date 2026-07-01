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
