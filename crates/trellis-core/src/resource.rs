use crate::{Graph, GraphResult, NodeId, ResourceKey, ScopeId};
use std::sync::Arc;

/// Data-only command describing an external resource lifecycle change.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

type PlannerFn<C> = dyn Fn(&Graph<C>) -> GraphResult<ResourcePlan<C>> + Send + Sync;

pub(crate) struct ResourcePlanner<C> {
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
        run: impl Fn(&Graph<C>) -> GraphResult<ResourcePlan<C>> + Send + Sync + 'static,
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
