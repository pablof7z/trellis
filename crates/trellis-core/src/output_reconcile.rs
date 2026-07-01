use crate::{
    ClearReason, Graph, GraphError, GraphResult, NodeId, OutputContext, OutputFrame,
    OutputFrameKind, OutputKey, RebaselineReason, Revision, ScopeId, TransactionId,
};
use std::collections::{BTreeMap, BTreeSet};

impl<C, O> Graph<C, O>
where
    O: Clone + PartialEq,
{
    pub(crate) fn produce_output_frames(
        &mut self,
        changed_nodes: &[NodeId],
        closed_scopes: &[ScopeId],
        rebaselines: &BTreeMap<OutputKey, RebaselineReason>,
        transaction_id: TransactionId,
        revision: Revision,
    ) -> GraphResult<Vec<OutputFrame<O>>> {
        let mut frames = Vec::new();
        let cleared = self.clear_closed_scope_outputs(closed_scopes, transaction_id, revision);
        frames.extend(cleared);

        let mut emitted = BTreeSet::new();
        for (key, reason) in rebaselines {
            if !self.outputs.contains_key(key) {
                continue;
            }
            let frame = self.rebaseline_output_frame(*key, *reason, transaction_id, revision)?;
            emitted.insert(*key);
            frames.push(frame);
        }

        let changed: BTreeSet<NodeId> = changed_nodes.iter().copied().collect();
        let keys: Vec<OutputKey> = self.outputs.keys().copied().collect();
        for key in keys {
            if emitted.contains(&key) {
                continue;
            }
            let Some(meta) = self.outputs.get(&key) else {
                continue;
            };
            let has_value = self.output_values.contains_key(&key);
            let dependencies_changed = meta
                .dependencies()
                .as_slice()
                .iter()
                .any(|dependency| changed.contains(dependency));
            if (!has_value || dependencies_changed)
                && let Some(frame) = self.incremental_output_frame(key, transaction_id, revision)?
            {
                frames.push(frame);
            }
        }

        Ok(frames)
    }

    fn clear_closed_scope_outputs(
        &mut self,
        closed_scopes: &[ScopeId],
        transaction_id: TransactionId,
        revision: Revision,
    ) -> Vec<OutputFrame<O>> {
        let mut frames = Vec::new();
        for scope in closed_scopes {
            let keys: Vec<OutputKey> = self
                .outputs
                .values()
                .filter_map(|meta| (meta.scope() == *scope).then_some(meta.key()))
                .collect();
            for key in keys {
                if let Some(meta) = self.outputs.remove(&key) {
                    self.output_specs.remove(&key);
                    self.output_values.remove(&key);
                    frames.push(OutputFrame {
                        output_key: key,
                        scope: meta.scope(),
                        transaction_id,
                        revision,
                        kind: OutputFrameKind::Clear(ClearReason::ScopeClosed),
                    });
                }
            }
        }
        frames
    }

    fn rebaseline_output_frame(
        &mut self,
        key: OutputKey,
        reason: RebaselineReason,
        transaction_id: TransactionId,
        revision: Revision,
    ) -> GraphResult<OutputFrame<O>> {
        let payload = self.compute_output(key)?;
        self.output_values.insert(key, payload.clone());
        let scope = self
            .outputs
            .get(&key)
            .ok_or(GraphError::UnknownOutput(key))?
            .scope();
        Ok(OutputFrame {
            output_key: key,
            scope,
            transaction_id,
            revision,
            kind: OutputFrameKind::Rebaseline(payload, reason),
        })
    }

    fn incremental_output_frame(
        &mut self,
        key: OutputKey,
        transaction_id: TransactionId,
        revision: Revision,
    ) -> GraphResult<Option<OutputFrame<O>>> {
        let previous = self.output_values.get(&key).cloned();
        let payload = self.compute_output(key)?;
        let meta = self
            .outputs
            .get(&key)
            .ok_or(GraphError::UnknownOutput(key))?;
        let kind = match previous {
            None => OutputFrameKind::Baseline(payload.clone()),
            Some(previous) if previous != payload || meta.options().emit_equal => {
                OutputFrameKind::Delta(payload.clone())
            }
            Some(_) => return Ok(None),
        };
        self.output_values.insert(key, payload);
        Ok(Some(OutputFrame {
            output_key: key,
            scope: meta.scope(),
            transaction_id,
            revision,
            kind,
        }))
    }

    fn compute_output(&self, key: OutputKey) -> GraphResult<O> {
        let meta = self
            .outputs
            .get(&key)
            .ok_or(GraphError::UnknownOutput(key))?;
        let spec = self
            .output_specs
            .get(&key)
            .ok_or(GraphError::UnknownOutput(key))?;
        let ctx = OutputContext::new(self, meta.dependencies().as_slice());
        spec.materialize(&ctx)
            .map_err(|error| GraphError::OutputFailed(key, error))
    }
}
