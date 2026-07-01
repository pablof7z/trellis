use core::fmt;
use core::num::NonZeroU64;

/// Stable graph-local identity for a node.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct NodeId(NonZeroU64);

impl NodeId {
    pub(crate) fn from_index(index: u64) -> Self {
        let value = NonZeroU64::new(index).expect("node ids start at 1");
        Self(value)
    }

    /// Returns the opaque numeric value for deterministic inspection.
    pub fn get(self) -> u64 {
        self.0.get()
    }
}

impl fmt::Debug for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeId({})", self.get())
    }
}

/// Stable graph-local identity for a scope.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ScopeId(NonZeroU64);

impl ScopeId {
    pub(crate) fn from_index(index: u64) -> Self {
        let value = NonZeroU64::new(index).expect("scope ids start at 1");
        Self(value)
    }

    /// Returns the opaque numeric value for deterministic inspection.
    pub fn get(self) -> u64 {
        self.0.get()
    }
}

impl fmt::Debug for ScopeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ScopeId({})", self.get())
    }
}

/// Monotonic graph revision marker.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Revision(u64);

impl Revision {
    /// Creates a revision from a numeric value.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the revision value.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Monotonic transaction identity marker.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TransactionId(u64);

impl TransactionId {
    /// Creates a transaction id from a numeric value.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the transaction id value.
    pub const fn get(self) -> u64 {
        self.0
    }
}
