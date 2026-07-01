use crate::{Graph, GraphResult, NodeId, ScopeId};
use core::fmt;
use std::sync::Arc;

/// Stable identity for a desired external resource.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ResourceKey(Box<str>);

impl ResourceKey {
    /// Creates a resource key from deterministic host-chosen identity.
    pub fn new(key: impl Into<Box<str>>) -> Self {
        Self(key.into())
    }

    /// Returns this key as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ResourceKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ResourceKey").field(&self.0).finish()
    }
}

/// Data-only command describing an external resource lifecycle change.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResourceCommand<C> {
    /// Open a resource with an application-defined command payload.
    Open {
        /// Resource identity understood by the graph.
        key: ResourceKey,
        /// Scope requesting ownership.
        scope: ScopeId,
        /// Host-defined command payload.
        command: C,
    },
    /// Close a resource after its final graph-visible owner is removed.
    Close {
        /// Resource identity understood by the graph.
        key: ResourceKey,
        /// Scope whose ownership was removed.
        scope: ScopeId,
    },
    /// Replace a live resource with an application-defined command payload.
    Replace {
        /// Resource identity understood by the graph.
        key: ResourceKey,
        /// Scope requesting replacement.
        scope: ScopeId,
        /// Host-defined command payload.
        command: C,
    },
    /// Refresh a live resource with an application-defined command payload.
    Refresh {
        /// Resource identity understood by the graph.
        key: ResourceKey,
        /// Scope requesting refresh.
        scope: ScopeId,
        /// Host-defined command payload.
        command: C,
    },
}

impl<C> ResourceCommand<C> {
    /// Returns the resource key for this command.
    pub fn key(&self) -> &ResourceKey {
        match self {
            Self::Open { key, .. }
            | Self::Close { key, .. }
            | Self::Replace { key, .. }
            | Self::Refresh { key, .. } => key,
        }
    }

    /// Returns the scope associated with this command.
    pub fn scope(&self) -> ScopeId {
        match self {
            Self::Open { scope, .. }
            | Self::Close { scope, .. }
            | Self::Replace { scope, .. }
            | Self::Refresh { scope, .. } => *scope,
        }
    }
}

/// Ordered data-only resource plan returned from graph propagation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourcePlan<C> {
    commands: Vec<ResourceCommand<C>>,
}

impl<C> ResourcePlan<C> {
    /// Creates an empty resource plan.
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    /// Adds an open command.
    pub fn open(&mut self, key: ResourceKey, scope: ScopeId, command: C) {
        self.commands.push(ResourceCommand::Open {
            key,
            scope,
            command,
        });
    }

    /// Adds a close command.
    pub fn close(&mut self, key: ResourceKey, scope: ScopeId) {
        self.commands.push(ResourceCommand::Close { key, scope });
    }

    /// Adds a replace command.
    pub fn replace(&mut self, key: ResourceKey, scope: ScopeId, command: C) {
        self.commands.push(ResourceCommand::Replace {
            key,
            scope,
            command,
        });
    }

    /// Adds a refresh command.
    pub fn refresh(&mut self, key: ResourceKey, scope: ScopeId, command: C) {
        self.commands.push(ResourceCommand::Refresh {
            key,
            scope,
            command,
        });
    }

    /// Returns ordered commands in this plan.
    pub fn commands(&self) -> &[ResourceCommand<C>] {
        &self.commands
    }

    /// Consumes the plan into ordered commands.
    pub fn into_commands(self) -> Vec<ResourceCommand<C>> {
        self.commands
    }

    pub(crate) fn append(&mut self, other: ResourcePlan<C>) {
        self.commands.extend(other.commands);
    }
}

impl<C> Default for ResourcePlan<C> {
    fn default() -> Self {
        Self::new()
    }
}

/// Read-only context passed to resource planners.
pub struct PlanContext<'graph, D> {
    scope: ScopeId,
    diff: &'graph D,
}

impl<'graph, D> PlanContext<'graph, D> {
    pub(crate) fn new(scope: ScopeId, diff: &'graph D) -> Self {
        Self { scope, diff }
    }

    /// Scope that owns resource demand produced by this planner.
    pub fn scope(&self) -> ScopeId {
        self.scope
    }

    /// Structural diff consumed by this planner.
    pub fn diff(&self) -> &'graph D {
        self.diff
    }
}

type PlannerFn<C> = dyn Fn(&Graph<C>) -> GraphResult<ResourcePlan<C>>;

/// Registered pure resource planner.
pub struct ResourcePlanner<C> {
    pub(crate) collection: NodeId,
    pub(crate) scope: ScopeId,
    run: Arc<PlannerFn<C>>,
}

impl<C> Clone for ResourcePlanner<C> {
    fn clone(&self) -> Self {
        Self {
            collection: self.collection,
            scope: self.scope,
            run: Arc::clone(&self.run),
        }
    }
}

impl<C> ResourcePlanner<C> {
    pub(crate) fn new(
        collection: NodeId,
        scope: ScopeId,
        run: impl Fn(&Graph<C>) -> GraphResult<ResourcePlan<C>> + 'static,
    ) -> Self {
        Self {
            collection,
            scope,
            run: Arc::new(run),
        }
    }

    pub(crate) fn run(&self, graph: &Graph<C>) -> GraphResult<ResourcePlan<C>> {
        (self.run)(graph)
    }
}
