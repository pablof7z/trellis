use crate::{
    CollectionNode, GraphResult, MapDiff, PlanContext, ResourcePlan, ResourcePlanner, SetDiff,
    Transaction,
};

impl<C: 'static> Transaction<'_, C> {
    /// Stages a map-diff resource planner.
    pub fn map_resource_planner<K, V>(
        &mut self,
        collection: CollectionNode<K, V>,
        scope: crate::ScopeId,
        planner: impl for<'ctx> Fn(&PlanContext<'ctx, MapDiff<K, V>>) -> GraphResult<ResourcePlan<C>>
        + 'static,
    ) -> GraphResult<()>
    where
        K: Clone + Ord + 'static,
        V: Clone + PartialEq + 'static,
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
            planner(&ctx)
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
        planner: impl for<'ctx> Fn(&PlanContext<'ctx, SetDiff<K>>) -> GraphResult<ResourcePlan<C>>
        + 'static,
    ) -> GraphResult<()>
    where
        K: Clone + Ord + 'static,
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
            planner(&ctx)
        });
        self.staged_resource_planner_collections
            .push(collection.id());
        self.working.resource_planners.push(resource_planner);
        self.graph_mutated = true;
        Ok(())
    }
}
