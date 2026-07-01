use crate::Graph;
impl<C> Clone for Graph<C> {
    fn clone(&self) -> Self {
        Self {
            next_node_id: self.next_node_id,
            next_scope_id: self.next_scope_id,
            next_transaction_id: self.next_transaction_id,
            revision: self.revision,
            nodes: self.nodes.clone(),
            scopes: self.scopes.clone(),
            input_values: self.input_values.clone(),
            derived_specs: self.derived_specs.clone(),
            derived_values: self.derived_values.clone(),
            collection_specs: self.collection_specs.clone(),
            collection_values: self.collection_values.clone(),
            previous_collection_values: self.previous_collection_values.clone(),
            collection_diffs: self.collection_diffs.clone(),
            resource_planners: self.resource_planners.clone(),
            resource_owners: self.resource_owners.clone(),
            transaction_open: self.transaction_open,
        }
    }
}

impl Graph<()> {
    /// Creates an empty graph with no resource command payload type.
    pub fn new() -> Self {
        Self::new_with_command_type()
    }
}

impl<C> Default for Graph<C> {
    fn default() -> Self {
        Self::new_with_command_type()
    }
}
