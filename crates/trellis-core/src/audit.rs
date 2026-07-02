use crate::{
    AuditEntry, AuditEvent, Graph, GraphError, GraphResult, NodeChangeExplanation, NodeHandle,
    NodeId, OutputFrameExplanation, OutputFrameKindTrace, OutputKey, ResourceCommandCause,
    ResourceCommandExplanation, ResourceCommandKind, ResourceKey, ScopeId, ScopeResourceInventory,
    TransactionResult,
};

impl<C> Graph<C> {
    /// Returns the committed audit log.
    pub fn audit_log(&self) -> &[AuditEntry] {
        &self.audit.log
    }

    /// Explains why a typed node last changed.
    pub fn why_changed<H: NodeHandle>(&self, node: H) -> Option<&NodeChangeExplanation> {
        self.why_changed_by_id(node.id())
    }

    /// Explains why a node id last changed.
    pub fn why_changed_by_id(&self, node: NodeId) -> Option<&NodeChangeExplanation> {
        self.audit.node_changes.get(&node)
    }

    /// Explains the latest resource command for a resource key.
    pub fn why_resource_command(&self, key: &ResourceKey) -> Option<&ResourceCommandExplanation> {
        self.audit.resource_commands.get(key)
    }

    /// Explains the latest output frame for an output key.
    pub fn why_output_frame(&self, key: OutputKey) -> Option<&OutputFrameExplanation> {
        self.audit.output_frames.get(&key)
    }

    /// Returns a dependency path from an upstream node to a downstream node.
    pub fn dependency_path(&self, from: NodeId, to: NodeId) -> Option<Vec<NodeId>> {
        if !self.nodes.contains_key(&from) || !self.nodes.contains_key(&to) {
            return None;
        }
        let mut path = vec![from];
        let mut visited = std::collections::BTreeSet::new();
        self.dependency_path_inner(from, to, &mut visited, &mut path)
            .then_some(path)
    }

    /// Returns resources currently owned by a scope.
    pub fn scope_resource_inventory(&self, scope: ScopeId) -> GraphResult<ScopeResourceInventory> {
        self.scope_meta(scope)
            .ok_or(GraphError::UnknownScope(scope))?;
        let resources = self
            .resource_owners
            .iter()
            .filter(|(_, owners)| owners.contains(&scope))
            .map(|(key, _)| key.clone())
            .collect();
        Ok(ScopeResourceInventory { scope, resources })
    }

    pub(crate) fn record_transaction_audit(&mut self, result: &TransactionResult<C>) {
        self.audit.log.extend(result.audit_log.iter().cloned());
        let changed_inputs = result.changed_inputs.clone();
        let changed_nodes = changed_nodes(result);
        for entry in &result.audit_log {
            if let Some(node) = event_node(entry.event) {
                let dependency_paths = self.paths_from_inputs_to_targets(&changed_inputs, &[node]);
                let input_causes = input_causes_from_paths(&dependency_paths);
                self.audit.node_changes.insert(
                    node,
                    NodeChangeExplanation {
                        node,
                        transaction_id: entry.transaction_id,
                        revision: entry.revision,
                        event: entry.event,
                        input_causes,
                        dependency_paths,
                    },
                );
            }
        }

        let causes = std::mem::take(&mut self.audit.pending_resource_causes);
        debug_assert_eq!(causes.len(), result.resource_plan.commands().len());
        for (index, command) in result.resource_plan.commands().iter().enumerate() {
            let cause = causes
                .get(index)
                .copied()
                .expect("resource command cause recorded during reconciliation");
            let collection_diffs = cause.collection().into_iter().collect::<Vec<_>>();
            let dependency_paths =
                self.paths_from_inputs_to_targets(&changed_inputs, &collection_diffs);
            let input_causes = input_causes_from_paths(&dependency_paths);
            self.audit.resource_commands.insert(
                command.key().clone(),
                ResourceCommandExplanation {
                    key: command.key().clone(),
                    scope: command.scope(),
                    transaction_id: result.transaction_id,
                    revision: result.revision,
                    kind: ResourceCommandKind::from_command(command),
                    cause,
                    collection_diffs,
                    changed_nodes: changed_nodes.clone(),
                    input_causes,
                    dependency_paths,
                },
            );
        }

        for frame in &result.output_frames {
            let dependencies = self
                .output_meta(frame.output_key)
                .map(|meta| meta.dependencies().as_slice().to_vec())
                .unwrap_or_default();
            let changed_dependencies = dependencies
                .iter()
                .copied()
                .filter(|node| changed_nodes.contains(node))
                .collect::<Vec<_>>();
            let dependency_paths =
                self.paths_from_inputs_to_targets(&changed_inputs, &changed_dependencies);
            let input_causes = input_causes_from_paths(&dependency_paths);
            self.audit.output_frames.insert(
                frame.output_key,
                OutputFrameExplanation {
                    output_key: frame.output_key,
                    scope: frame.scope,
                    transaction_id: frame.transaction_id,
                    revision: frame.revision,
                    kind: OutputFrameKindTrace::from_kind(&frame.kind),
                    dependencies,
                    changed_dependencies,
                    input_causes,
                    dependency_paths,
                },
            );
        }
    }

    fn paths_from_inputs_to_targets(
        &self,
        inputs: &[NodeId],
        targets: &[NodeId],
    ) -> Vec<Vec<NodeId>> {
        inputs
            .iter()
            .flat_map(|input| {
                targets
                    .iter()
                    .filter_map(|target| self.dependency_path(*input, *target))
            })
            .collect()
    }

    fn dependency_path_inner(
        &self,
        current: NodeId,
        target: NodeId,
        visited: &mut std::collections::BTreeSet<NodeId>,
        path: &mut Vec<NodeId>,
    ) -> bool {
        if current == target {
            return true;
        }
        if !visited.insert(current) {
            return false;
        }
        for next in self.downstream_nodes(current) {
            path.push(next);
            if self.dependency_path_inner(next, target, visited, path) {
                return true;
            }
            path.pop();
        }
        false
    }

    fn downstream_nodes(&self, node: NodeId) -> Vec<NodeId> {
        self.nodes
            .values()
            .filter_map(|meta| {
                meta.dependencies()
                    .as_slice()
                    .contains(&node)
                    .then_some(meta.id())
            })
            .collect()
    }
}

impl ResourceCommandCause {
    fn collection(self) -> Option<NodeId> {
        match self {
            Self::Planner { collection } => Some(collection),
            Self::ScopeClosed { .. } => None,
        }
    }
}

fn input_causes_from_paths(paths: &[Vec<NodeId>]) -> Vec<NodeId> {
    let mut causes = Vec::new();
    for path in paths {
        if let Some(input) = path.first()
            && !causes.contains(input)
        {
            causes.push(*input);
        }
    }
    causes
}

fn changed_nodes<C>(result: &TransactionResult<C>) -> Vec<NodeId> {
    let mut nodes = result.changed_inputs.clone();
    nodes.extend(result.changed_derived_nodes.iter().copied());
    nodes.extend(result.changed_collection_nodes.iter().copied());
    nodes
}

fn event_node(event: AuditEvent) -> Option<NodeId> {
    match event {
        AuditEvent::InputChanged(node)
        | AuditEvent::DerivedChanged(node)
        | AuditEvent::CollectionChanged(node)
        | AuditEvent::NodeCreated(node) => Some(node),
        AuditEvent::NodeAttached { node, .. } => Some(node),
        AuditEvent::InputUnchanged(_)
        | AuditEvent::ScopeCreated(_)
        | AuditEvent::ScopeClosed(_) => None,
    }
}
