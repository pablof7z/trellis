use crate::{
    CollectionNode, GraphError, GraphResult, MapDiff, PlanContext, PlanError, ResourceKey,
    ResourcePlan, SetDiff, Transaction, resource::ResourcePlanner,
};

impl<C: 'static> Transaction<'_, C> {
    /// Stages a map-diff resource planner.
    pub fn map_resource_planner<K, V>(
        &mut self,
        collection: CollectionNode<K, V>,
        scope: crate::ScopeId,
        planner: impl for<'ctx> Fn(
            &PlanContext<'ctx, MapDiff<K, V>>,
        ) -> Result<ResourcePlan<C>, PlanError>
        + Send
        + Sync
        + 'static,
    ) -> GraphResult<()>
    where
        K: Clone + Ord + Send + Sync + 'static,
        V: Clone + PartialEq + Send + Sync + 'static,
    {
        self.ensure_open()?;
        self.working.require_scope_open(scope)?;
        self.working
            .validate_map_collection_read::<K, V>(collection.id())?;
        let resource_planner = ResourcePlanner::new(collection.id(), scope, move |graph| {
            let Some(diff) = graph.map_diff(collection)? else {
                return Ok(ResourcePlan::new());
            };
            let ctx = PlanContext::new(scope, diff);
            planner(&ctx).map_err(|error| GraphError::PlanFailed(scope, error))
        });
        self.staged_resource_planner_collections
            .push(collection.id());
        self.working.resource_planners.push(resource_planner);
        self.graph_mutated = true;
        Ok(())
    }

    /// Stages a set-diff resource planner.
    pub fn set_resource_planner<K>(
        &mut self,
        collection: CollectionNode<K, ()>,
        scope: crate::ScopeId,
        planner: impl for<'ctx> Fn(&PlanContext<'ctx, SetDiff<K>>) -> Result<ResourcePlan<C>, PlanError>
        + Send
        + Sync
        + 'static,
    ) -> GraphResult<()>
    where
        K: Clone + Ord + Send + Sync + 'static,
    {
        self.ensure_open()?;
        self.working.require_scope_open(scope)?;
        self.working
            .validate_set_collection_read::<K>(collection.id())?;
        let resource_planner = ResourcePlanner::new(collection.id(), scope, move |graph| {
            let Some(diff) = graph.set_diff(collection)? else {
                return Ok(ResourcePlan::new());
            };
            let ctx = PlanContext::new(scope, diff);
            planner(&ctx).map_err(|error| GraphError::PlanFailed(scope, error))
        });
        self.staged_resource_planner_collections
            .push(collection.id());
        self.working.resource_planners.push(resource_planner);
        self.graph_mutated = true;
        Ok(())
    }

    /// Stages a set-diff planner that opens added members and closes removed members.
    pub fn open_close_planner<K>(
        &mut self,
        collection: CollectionNode<K, ()>,
        scope: crate::ScopeId,
        key: impl Fn(&K) -> ResourceKey + Send + Sync + 'static,
        open: impl Fn(&K) -> C + Send + Sync + 'static,
    ) -> GraphResult<()>
    where
        K: Clone + Ord + Send + Sync + 'static,
    {
        self.set_resource_planner(collection, scope, move |ctx| {
            let mut plan = ResourcePlan::new();
            for added in &ctx.diff().added {
                plan.open(key(&added.value), ctx.scope(), open(&added.value));
            }
            for removed in &ctx.diff().removed {
                plan.close(key(&removed.value), ctx.scope());
            }
            Ok(plan)
        })
    }
}
