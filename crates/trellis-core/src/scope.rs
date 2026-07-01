use crate::ScopeId;

/// Inspectable metadata for a graph scope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScopeMeta {
    id: ScopeId,
    debug_name: String,
    parent: Option<ScopeId>,
    closed: bool,
}

impl ScopeMeta {
    pub(crate) fn new(id: ScopeId, debug_name: impl Into<String>, parent: Option<ScopeId>) -> Self {
        Self {
            id,
            debug_name: debug_name.into(),
            parent,
            closed: false,
        }
    }

    /// Returns this scope's id.
    pub fn id(&self) -> ScopeId {
        self.id
    }

    /// Returns this scope's debug name.
    pub fn debug_name(&self) -> &str {
        &self.debug_name
    }

    /// Returns this scope's parent, if any.
    pub fn parent(&self) -> Option<ScopeId> {
        self.parent
    }

    /// Returns whether this scope has been marked closed.
    pub fn is_closed(&self) -> bool {
        self.closed
    }

    pub(crate) fn close(&mut self) {
        self.closed = true;
    }
}
