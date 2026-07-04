use crate::input::{StoredInput, boxed_input};
use crate::transaction_trace_build::{scope_events, stable_node_union};
use crate::{
    Graph, GraphError, GraphResult, InputNode, NodeId, OutputKey, RebaselineReason, ScopeId,
    TransactionId,
    transaction_types::{
        AuditEntry, AuditEvent, StagedInputChange, StagedInputOutcome, TransactionOptions,
        TransactionPhase, TransactionResult,
    },
};
use std::collections::{BTreeMap, BTreeSet};

/// Staged canonical input transaction.
pub struct Transaction<'graph, C = ()> {
    pub(crate) graph: &'graph mut Graph<C>,
    pub(crate) working: Graph<C>,
    id: TransactionId,
    options: TransactionOptions,
    staged_inputs: BTreeMap<NodeId, Box<dyn StoredInput>>,
    pub(crate) staged_events: Vec<AuditEvent>,
    pub(crate) staged_resource_planner_collections: Vec<NodeId>,
    pub(crate) staged_output_rebaselines: BTreeMap<OutputKey, RebaselineReason>,
    pub(crate) graph_mutated: bool,
    pub(crate) failed: Option<GraphError>,
    closed: bool,
}

impl<'graph, C> Transaction<'graph, C> {
    pub(crate) fn new(
        graph: &'graph mut Graph<C>,
        id: TransactionId,
        options: TransactionOptions,
    ) -> Self
    where
        C: Clone,
    {
        let mut working = graph.clone();
        working.transaction_open = false;
        Self {
            graph,
            working,
            id,
            options,
            staged_inputs: BTreeMap::new(),
            staged_events: Vec::new(),
            staged_resource_planner_collections: Vec::new(),
            staged_output_rebaselines: BTreeMap::new(),
            graph_mutated: false,
            failed: None,
            closed: false,
        }
    }

    /// Returns this transaction's id.
    pub fn id(&self) -> TransactionId {
        self.id
    }

    /// Stages a typed canonical input change.
    pub fn set_input<T>(&mut self, input: InputNode<T>, value: T) -> GraphResult<()>
    where
        T: Clone + PartialEq + Send + Sync + 'static,
    {
        self.set_input_by_id(input.id(), value)
    }

    /// Stages a canonical input change by node id.
    pub fn set_input_by_id<T>(&mut self, node: NodeId, value: T) -> GraphResult<()>
    where
        T: Clone + PartialEq + Send + Sync + 'static,
    {
        self.ensure_open()?;
        if let Err(error) = self.working.validate_input_write::<T>(node) {
            self.failed.get_or_insert_with(|| error.clone());
            return Err(error);
        }
        self.staged_inputs.insert(node, boxed_input(value));
        Ok(())
    }

    /// Commits staged input changes atomically.
    pub fn commit(&mut self) -> GraphResult<TransactionResult<C>>
    where
        C: Clone + PartialEq,
    {
        let (result, closed_scopes) = self.run_pipeline()?;
        self.working.reclaim_closed_scopes(&closed_scopes);
        std::mem::swap(self.graph, &mut self.working);
        self.close();
        Ok(result)
    }

    /// Runs the full commit pipeline on the working copy and returns the
    /// resulting [`TransactionResult`] **without committing** — the real graph
    /// is left untouched and the transaction is consumed.
    ///
    /// This is a near-free simulation: "what would committing these staged
    /// inputs produce?" It is the Terraform-plan / `--dry-run` for a Trellis
    /// application. The cost is already paid: [`Transaction`] begins by cloning
    /// the graph into a private working copy, and the entire pipeline
    /// (recompute, structural diffs, resource plans, output frames, audit) runs
    /// on that working copy. The only mutation of the real graph performed by
    /// [`commit`](Self::commit) is the final swap, which `preview` simply skips.
    ///
    /// The working copy — including any recomputed values, revision bump, and
    /// recorded audit — is dropped when the transaction is dropped, so the real
    /// graph's state, revision, and audit log are all left exactly as they were.
    pub fn preview(mut self) -> GraphResult<TransactionResult<C>>
    where
        C: Clone + PartialEq,
    {
        let (result, _closed_scopes) = self.run_pipeline()?;
        Ok(result)
    }

    /// Runs the shared pre-swap commit pipeline against `self.working`, leaving
    /// the working copy fully baked (recomputed, revision-bumped, audit
    /// recorded) and returning the [`TransactionResult`] together with the
    /// scopes closed during the transaction (needed by [`commit`](Self::commit)
    /// to reclaim them before swapping). Every mutation lands on `self.working`
    /// only; the real graph (`self.graph`) is never touched here, which is what
    /// makes both [`commit`](Self::commit) and [`preview`](Self::preview) able
    /// to share it.
    fn run_pipeline(&mut self) -> GraphResult<(TransactionResult<C>, Vec<ScopeId>)>
    where
        C: Clone + PartialEq,
    {
        self.ensure_open()?;
        let mut phase_trace = vec![TransactionPhase::StageOperations];
        phase_trace.push(TransactionPhase::ValidateTransaction);
        if let Some(error) = self.failed.clone() {
            self.close();
            return Err(error);
        }

        let mut changed_inputs = Vec::new();
        for (node, staged) in &self.staged_inputs {
            let changed = self
                .working
                .input_values
                .get(node)
                .is_none_or(|current| !current.equals(staged.as_ref()));
            if changed || !self.options.skip_equal_inputs {
                changed_inputs.push(*node);
            }
        }
        let changed_input_set = changed_inputs.iter().copied().collect::<BTreeSet<_>>();
        let staged_input_changes = self
            .staged_inputs
            .keys()
            .map(|node| StagedInputChange {
                node: *node,
                outcome: if changed_input_set.contains(node) {
                    StagedInputOutcome::Changed
                } else {
                    StagedInputOutcome::Unchanged
                },
            })
            .collect::<Vec<_>>();

        let next_revision = if changed_inputs.is_empty() && !self.graph_mutated {
            self.graph.revision
        } else {
            self.graph.revision.next()
        };

        let mut audit_events = self.staged_events.clone();
        for node in self.staged_inputs.keys() {
            let event = if changed_input_set.contains(node) {
                AuditEvent::InputChanged(*node)
            } else {
                AuditEvent::InputUnchanged(*node)
            };
            audit_events.push(event);
        }

        phase_trace.push(TransactionPhase::CommitCanonicalInputs);
        for node in &changed_inputs {
            if let Some(staged) = self.staged_inputs.get(node) {
                self.working.input_values.insert(*node, staged.clone());
                if let Some(meta) = self.working.nodes.get_mut(node) {
                    meta.mark_changed(next_revision);
                }
            }
        }
        for event in &self.staged_events {
            if let AuditEvent::NodeCreated(node) = event
                && let Some(meta) = self.working.nodes.get_mut(node)
            {
                meta.mark_created(next_revision);
            }
        }
        let created_nodes: Vec<NodeId> = self
            .staged_events
            .iter()
            .filter_map(|event| match event {
                AuditEvent::NodeCreated(node) => Some(*node),
                _ => None,
            })
            .collect();
        let dirty_roots = stable_node_union(changed_inputs.iter().copied().chain(created_nodes));
        let mut initial_changed = dirty_roots.clone();
        phase_trace.push(TransactionPhase::MarkDirtyNodes);
        phase_trace.push(TransactionPhase::RecomputeDerivedNodes);
        let derived_trace = match self.working.recompute_dirty_derived(&initial_changed) {
            Ok(trace) => trace,
            Err(error) => {
                self.close();
                return Err(error);
            }
        };
        let recomputed_derived_nodes = derived_trace.recomputed;
        let changed_derived_nodes = derived_trace.changed;
        for node in &changed_derived_nodes {
            if let Some(meta) = self.working.nodes.get_mut(node) {
                meta.mark_changed(next_revision);
            }
            audit_events.push(AuditEvent::DerivedChanged(*node));
        }
        initial_changed.extend(changed_derived_nodes.iter().copied());
        phase_trace.push(TransactionPhase::RecomputeCollectionNodes);
        let collection_recompute = match self.working.recompute_dirty_collections(&initial_changed)
        {
            Ok(trace) => trace,
            Err(error) => {
                self.close();
                return Err(error);
            }
        };
        let recomputed_collection_nodes = collection_recompute.recomputed;
        let changed_collection_nodes = collection_recompute.changed;
        for node in &changed_collection_nodes {
            if let Some(meta) = self.working.nodes.get_mut(node) {
                meta.mark_changed(next_revision);
            }
            audit_events.push(AuditEvent::CollectionChanged(*node));
        }
        phase_trace.push(TransactionPhase::ComputeStructuralDiffs);
        self.working
            .baseline_collection_diffs(&self.staged_resource_planner_collections);
        let collection_diffs = self
            .working
            .collection_diffs
            .iter()
            .map(|(node, diff)| diff.trace(*node))
            .collect::<Vec<_>>();
        phase_trace.push(TransactionPhase::ResolveScopeLifecycle);
        let closed_scopes: Vec<_> = self
            .staged_events
            .iter()
            .filter_map(|event| match event {
                AuditEvent::ScopeClosed(scope) => Some(*scope),
                _ => None,
            })
            .collect();
        let scope_events = scope_events(&audit_events);
        phase_trace.push(TransactionPhase::ProduceResourcePlans);
        let resource_plan = match self.working.produce_resource_plan(&closed_scopes) {
            Ok(plan) => plan,
            Err(error) => {
                self.close();
                return Err(error);
            }
        };
        let resource_coalescences = self.working.take_pending_resource_coalescences();
        audit_events.extend(resource_coalescences.iter().map(|coalesced| {
            AuditEvent::ResourceOpenCoalesced {
                key: coalesced.key.clone(),
                scope: coalesced.scope,
                existing_owner_count: coalesced.existing_owner_count,
            }
        }));
        let mut output_changed = initial_changed.clone();
        output_changed.extend(changed_collection_nodes.iter().copied());
        phase_trace.push(TransactionPhase::ProduceOutputFrames);
        let output_frames = match self.working.produce_output_frames(
            &output_changed,
            &closed_scopes,
            &self.staged_output_rebaselines,
            self.id,
            next_revision,
        ) {
            Ok(frames) => frames,
            Err(error) => {
                self.close();
                return Err(error);
            }
        };
        let audit_log = audit_events
            .into_iter()
            .map(|event| AuditEntry {
                transaction_id: self.id,
                revision: next_revision,
                event,
            })
            .collect();
        phase_trace.push(TransactionPhase::CommitGraphRevision);
        self.working.revision = next_revision;
        self.working.next_node_id = self.graph.next_node_id;
        self.working.next_scope_id = self.graph.next_scope_id;
        self.working.next_output_key = self.graph.next_output_key;

        phase_trace.push(TransactionPhase::ReturnTransactionResult);
        let result = TransactionResult {
            transaction_id: self.id,
            revision: next_revision,
            staged_input_changes,
            changed_inputs,
            dirty_roots,
            recomputed_derived_nodes,
            changed_derived_nodes,
            recomputed_collection_nodes,
            changed_collection_nodes,
            collection_diffs,
            resource_plan,
            resource_coalescences,
            output_frames,
            scope_events,
            audit_log,
            phase_trace,
            invariant_results: Vec::new(),
        };
        self.working
            .record_transaction_audit(&result, self.options.audit_explanations);
        Ok((result, closed_scopes))
    }

    pub(crate) fn ensure_open(&self) -> GraphResult<()> {
        if self.closed {
            Err(GraphError::TransactionClosed(self.id))
        } else {
            Ok(())
        }
    }

    fn close(&mut self) {
        self.closed = true;
        self.graph.transaction_open = false;
    }
}

impl<C> Drop for Transaction<'_, C> {
    fn drop(&mut self) {
        if !self.closed {
            self.graph.transaction_open = false;
        }
    }
}
