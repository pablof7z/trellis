use trellis_core::{OutputFrameKind, ResourceCommand, TransactionResult};

use super::engine::FleetPulseApp;
use super::graph::target_from_key;
use super::types::{FleetCommand, FleetEffect, FleetFrame, FleetSnapshot};

impl FleetPulseApp {
    pub(super) fn apply_result(&mut self, result: TransactionResult<FleetCommand>) {
        for command in result.resource_plan.commands() {
            match command {
                ResourceCommand::Open {
                    key,
                    scope,
                    command: FleetCommand::Open(target),
                }
                | ResourceCommand::Replace {
                    key,
                    scope,
                    command: FleetCommand::Open(target),
                }
                | ResourceCommand::Refresh {
                    key,
                    scope,
                    command: FleetCommand::Open(target),
                } => {
                    self.status_runtime
                        .record_live(key.clone(), *scope, result.revision);
                    self.effects.push_back(match command {
                        ResourceCommand::Open { .. } => FleetEffect::Open(target.clone()),
                        ResourceCommand::Replace { .. } => FleetEffect::Replace(target.clone()),
                        ResourceCommand::Refresh { .. } => FleetEffect::Replace(target.clone()),
                        ResourceCommand::Close { .. } => unreachable!(),
                    });
                }
                ResourceCommand::Close { key, scope } => {
                    self.status_runtime
                        .record_closed(key.clone(), *scope, result.revision);
                    if let Some(target) = target_from_key(key) {
                        self.effects.push_back(FleetEffect::Close(target));
                    }
                }
            }
        }

        for frame in &result.output_frames {
            let frame = match &frame.kind {
                OutputFrameKind::Baseline(snapshot) => {
                    FleetFrame::Baseline(snapshot_from(snapshot))
                }
                OutputFrameKind::Delta(snapshot) => FleetFrame::Delta(snapshot_from(snapshot)),
                OutputFrameKind::Rebaseline(snapshot, _) => {
                    FleetFrame::Rebaseline(snapshot_from(snapshot))
                }
                OutputFrameKind::Clear(_) => FleetFrame::Cleared,
            };
            self.output_queue.push_back(frame);
        }

        let mut trace = result.trace();
        trace
            .invariant_results
            .push(trellis_core::InvariantResultTrace {
                name: "incremental_equals_full_recompute".to_owned(),
                passed: self.graph.graph.full_recompute_check().is_ok(),
            });
        self.diagnostic_traces.push_back(trace);
    }
}

fn snapshot_from(payload: &trellis_core::OutputPayload) -> FleetSnapshot {
    payload
        .get::<FleetSnapshot>()
        .expect("fleet output payload type")
        .clone()
}
