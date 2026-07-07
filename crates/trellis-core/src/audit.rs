use crate::{
    AuditEvent, AuditExplanationLevel, AuditExplanations, Graph, GraphError, GraphResult,
    NodeChangeExplanation, NodeHandle, NodeId, OutputFrameExplanation, OutputFrameKindTrace,
    OutputKey, ResourceCommandCause, ResourceCommandExplanation, ResourceCommandKind, ResourceKey,
    ScopeId, ScopeResourceInventory, TransactionResult,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

impl<C> Graph<C> {
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
        let downstream = self.reverse_dependency_index();
        shortest_dependency_path(&downstream, from, to)
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

    pub(crate) fn record_transaction_audit(
        &mut self,
        result: &TransactionResult<C>,
        level: AuditExplanationLevel,
    ) -> AuditExplanations {
        let causes = std::mem::take(&mut self.audit.pending_resource_causes);
        if level == AuditExplanationLevel::Disabled {
            self.audit.clear_explanations();
            return AuditExplanations::with_level(result.transaction_id, result.revision, level);
        }

        let mut explanations =
            AuditExplanations::with_level(result.transaction_id, result.revision, level);
        let changed_inputs = result.changed_inputs.clone();
        let changed_nodes = changed_nodes(result);
        let downstream = (level == AuditExplanationLevel::DependencyPaths)
            .then(|| self.reverse_dependency_index());
        for entry in &result.audit_log {
            if let Some(node) = event_node(&entry.event) {
                let dependency_paths =
                    paths_from_inputs_to_targets(downstream.as_ref(), &changed_inputs, &[node]);
                let input_causes = input_causes_from_paths(&dependency_paths);
                let explanation = NodeChangeExplanation {
                    node,
                    transaction_id: entry.transaction_id,
                    revision: entry.revision,
                    event: entry.event.clone(),
                    input_causes,
                    dependency_paths,
                };
                self.audit.node_changes.insert(node, explanation.clone());
                explanations.node_changes.insert(node, explanation);
            }
        }

        for (index, command) in result.resource_plan.commands().iter().enumerate() {
            let cause = causes
                .get(index)
                .copied()
                .expect("resource command cause recorded during reconciliation");
            let collection_diffs = cause.collection().into_iter().collect::<Vec<_>>();
            let dependency_paths = paths_from_inputs_to_targets(
                downstream.as_ref(),
                &changed_inputs,
                &collection_diffs,
            );
            let input_causes = input_causes_from_paths(&dependency_paths);
            let key = command.key().clone();
            let explanation = ResourceCommandExplanation {
                key: key.clone(),
                scope: command.scope(),
                transaction_id: result.transaction_id,
                revision: result.revision,
                kind: ResourceCommandKind::from_command(command),
                cause,
                collection_diffs,
                changed_nodes: changed_nodes.clone(),
                input_causes,
                dependency_paths,
            };
            self.audit
                .resource_commands
                .insert(key.clone(), explanation.clone());
            explanations.resource_commands.insert(key, explanation);
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
            let dependency_paths = paths_from_inputs_to_targets(
                downstream.as_ref(),
                &changed_inputs,
                &changed_dependencies,
            );
            let input_causes = input_causes_from_paths(&dependency_paths);
            let explanation = OutputFrameExplanation {
                output_key: frame.output_key,
                scope: frame.scope,
                transaction_id: frame.transaction_id,
                revision: frame.revision,
                kind: OutputFrameKindTrace::from_kind(&frame.kind),
                dependencies,
                changed_dependencies,
                input_causes,
                dependency_paths,
            };
            self.audit
                .output_frames
                .insert(frame.output_key, explanation.clone());
            explanations
                .output_frames
                .insert(frame.output_key, explanation);
        }
        explanations
    }

    fn reverse_dependency_index(&self) -> BTreeMap<NodeId, Vec<NodeId>> {
        let mut downstream: BTreeMap<NodeId, Vec<NodeId>> = BTreeMap::new();
        for meta in self.nodes.values() {
            for dependency in meta.dependencies().as_slice() {
                downstream.entry(*dependency).or_default().push(meta.id());
            }
        }
        for nodes in downstream.values_mut() {
            nodes.sort();
        }
        downstream
    }
}

fn paths_from_inputs_to_targets(
    downstream: Option<&BTreeMap<NodeId, Vec<NodeId>>>,
    inputs: &[NodeId],
    targets: &[NodeId],
) -> Vec<Vec<NodeId>> {
    let Some(downstream) = downstream else {
        return Vec::new();
    };

    inputs
        .iter()
        .flat_map(|input| {
            targets
                .iter()
                .filter_map(|target| shortest_dependency_path(downstream, *input, *target))
        })
        .collect()
}

fn shortest_dependency_path(
    downstream: &BTreeMap<NodeId, Vec<NodeId>>,
    from: NodeId,
    to: NodeId,
) -> Option<Vec<NodeId>> {
    if from == to {
        return Some(vec![from]);
    }

    let mut queue = VecDeque::from([from]);
    let mut visited = BTreeSet::from([from]);
    let mut previous = BTreeMap::new();

    while let Some(current) = queue.pop_front() {
        for next in downstream.get(&current).into_iter().flatten().copied() {
            if !visited.insert(next) {
                continue;
            }
            previous.insert(next, current);
            if next == to {
                return Some(reconstruct_path(from, to, &previous));
            }
            queue.push_back(next);
        }
    }

    None
}

fn reconstruct_path(from: NodeId, to: NodeId, previous: &BTreeMap<NodeId, NodeId>) -> Vec<NodeId> {
    let mut path = vec![to];
    let mut current = to;
    while current != from {
        current = previous[&current];
        path.push(current);
    }
    path.reverse();
    path
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

fn event_node(event: &AuditEvent) -> Option<NodeId> {
    match event {
        AuditEvent::InputChanged(node)
        | AuditEvent::DerivedChanged(node)
        | AuditEvent::CollectionChanged(node)
        | AuditEvent::NodeCreated(node) => Some(*node),
        AuditEvent::NodeAttached { node, .. } => Some(*node),
        AuditEvent::InputUnchanged(_)
        | AuditEvent::ScopeCreated(_)
        | AuditEvent::ScopeClosed(_)
        | AuditEvent::ResourceOpenCoalesced { .. } => None,
    }
}
