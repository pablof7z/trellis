use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use trellis_core::{
    DependencyList, Graph, InputNode, ResourceCommand, ResourceKey, ResourcePlan, TransactionResult,
};

use crate::leak_duel_present::{receipt_for, rows, stats};
use crate::leak_duel_sim::{
    Rng, apply_event, apply_naive, counted, desired_attachments, initial_inputs, next_event,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeakDuelRequest {
    pub seed: Option<u64>,
    pub chaos: Option<u8>,
    pub ticks: Option<u32>,
    pub selected: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeakDuelState {
    pub seed: u64,
    pub chaos: u8,
    pub tick: u32,
    pub inputs: ChatInputs,
    pub should_open: Vec<String>,
    pub rows: Vec<AttachmentRow>,
    pub naive: SideStats,
    pub trellis: SideStats,
    pub activity: Vec<Activity>,
    pub selected_receipt: Receipt,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatInputs {
    pub workspace: String,
    pub joined_rooms: BTreeSet<String>,
    pub permission_grants: BTreeSet<String>,
    pub follows: BTreeSet<String>,
    pub network_online: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SideStats {
    pub open: u32,
    pub should_open: u32,
    pub delta: i32,
    pub orphaned: u32,
    pub duplicate_handles: u32,
    pub verdict: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentRow {
    pub key: String,
    pub label: String,
    pub should_open: bool,
    pub naive_open: u32,
    pub trellis_open: u32,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub tick: u32,
    pub label: String,
    pub detail: String,
    pub naive_note: String,
    pub trellis_note: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Receipt {
    pub key: String,
    pub title: String,
    pub status: String,
    pub steps: Vec<ReceiptStep>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptStep {
    pub label: String,
    pub detail: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AttachmentCommand {
    key: String,
}

struct TrellisHarness {
    graph: Graph<AttachmentCommand>,
    inputs: InputNode<ChatInputs>,
    last_action: BTreeMap<String, String>,
}

pub fn run(request: LeakDuelRequest) -> LeakDuelState {
    let seed = request.seed.unwrap_or(1337);
    let chaos = request.chaos.unwrap_or(7).min(10);
    let ticks = request.ticks.unwrap_or(18).min(120);
    let mut rng = Rng::new(seed ^ ((chaos as u64) << 32));
    let mut inputs = initial_inputs();
    let initial = desired_attachments(&inputs);
    let mut naive_open = counted(&initial);
    let (mut trellis, mut trellis_open) = TrellisHarness::new(&inputs);
    let mut activity = vec![Activity {
        tick: 0,
        label: "Open initial workspace".to_owned(),
        detail: "atlas workspace derives joined rooms, grants, follows, and network into attachment streams.".to_owned(),
        naive_note: "callback ledger starts correct".to_owned(),
        trellis_note: "initial resource plan opens the same desired set".to_owned(),
    }];

    for tick in 1..=ticks {
        let before = desired_attachments(&inputs);
        let event = next_event(&mut rng, tick, chaos, &inputs);
        apply_event(&mut inputs, &event);
        let after = desired_attachments(&inputs);
        let naive_note = apply_naive(&mut naive_open, &before, &after, &event, chaos);
        let trellis_note = trellis.apply(&inputs, &mut trellis_open, &event.label);
        activity.push(Activity {
            tick,
            label: event.label,
            detail: event.detail,
            naive_note,
            trellis_note,
        });
    }

    let should = desired_attachments(&inputs);
    let selected = request
        .selected
        .filter(|key| should.contains(key) || naive_open.contains_key(key))
        .or_else(|| should.iter().next().cloned())
        .or_else(|| naive_open.keys().next().cloned())
        .unwrap_or_else(|| "attachment:none".to_owned());

    LeakDuelState {
        seed,
        chaos,
        tick: ticks,
        inputs: inputs.clone(),
        should_open: should.iter().cloned().collect(),
        rows: rows(&should, &naive_open, &trellis_open),
        naive: stats(&naive_open, &should, "drift"),
        trellis: stats(&trellis_open, &should, "reconciled"),
        activity: activity.into_iter().rev().take(9).collect(),
        selected_receipt: receipt_for(&selected, &inputs, &should, &trellis.last_action),
    }
}

impl TrellisHarness {
    fn new(inputs: &ChatInputs) -> (Self, BTreeMap<String, u32>) {
        let mut graph = Graph::<AttachmentCommand>::new_with_command_type();
        let mut tx = graph.begin_transaction().unwrap();
        let scope = tx.create_scope("chat-session").unwrap();
        let input = tx.input::<ChatInputs>("chatInputs").unwrap();
        tx.set_input(input, inputs.clone()).unwrap();
        let desired = tx
            .set_collection(
                "desiredAttachments",
                DependencyList::new([input.id()]).unwrap(),
                move |ctx| Ok(desired_attachments(ctx.input(input)?)),
            )
            .unwrap();
        tx.set_resource_planner(desired, scope, move |ctx| {
            let mut plan = ResourcePlan::new();
            for added in &ctx.diff().added {
                plan.open(
                    ResourceKey::new(added.value.clone()),
                    ctx.scope(),
                    AttachmentCommand {
                        key: added.value.clone(),
                    },
                );
            }
            for removed in &ctx.diff().removed {
                plan.close(ResourceKey::new(removed.value.clone()), ctx.scope());
            }
            Ok(plan)
        })
        .unwrap();
        let result = tx.commit().unwrap();
        drop(tx);

        let mut harness = Self {
            graph,
            inputs: input,
            last_action: BTreeMap::new(),
        };
        let mut open = BTreeMap::new();
        harness.apply_plan(&result, &mut open, "bootstrap");
        (harness, open)
    }

    fn apply(
        &mut self,
        inputs: &ChatInputs,
        open: &mut BTreeMap<String, u32>,
        label: &str,
    ) -> String {
        let mut tx = self.graph.begin_transaction().unwrap();
        tx.set_input(self.inputs, inputs.clone()).unwrap();
        let result = tx.commit().unwrap();
        drop(tx);
        self.apply_plan(&result, open, label)
    }

    fn apply_plan(
        &mut self,
        result: &TransactionResult<AttachmentCommand>,
        open: &mut BTreeMap<String, u32>,
        label: &str,
    ) -> String {
        let mut opens = 0;
        let mut closes = 0;
        for command in result.resource_plan.commands() {
            match command {
                ResourceCommand::Open { key, command, .. } => {
                    open.insert(key.as_str().to_owned(), 1);
                    self.last_action.insert(
                        key.as_str().to_owned(),
                        format!(
                            "tx {} opened after {label} ({})",
                            result.transaction_id.get(),
                            command.key
                        ),
                    );
                    opens += 1;
                }
                ResourceCommand::Close { key, .. } => {
                    open.remove(key.as_str());
                    self.last_action.insert(
                        key.as_str().to_owned(),
                        format!("tx {} closed after {label}", result.transaction_id.get()),
                    );
                    closes += 1;
                }
                _ => {}
            }
        }
        format!("{opens} open, {closes} close from the resource plan")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_produces_same_snapshot() {
        let request = LeakDuelRequest {
            seed: Some(1337),
            chaos: Some(8),
            ticks: Some(24),
            selected: None,
        };
        let left = serde_json::to_string(&run(request.clone())).unwrap();
        let right = serde_json::to_string(&run(request)).unwrap();
        assert_eq!(left, right);
    }

    #[test]
    fn trellis_side_stays_reconciled_under_high_chaos() {
        let state = run(LeakDuelRequest {
            seed: Some(42),
            chaos: Some(9),
            ticks: Some(32),
            selected: None,
        });
        assert_eq!(state.trellis.delta, 0);
        assert_eq!(state.trellis.orphaned, 0);
        assert_eq!(state.trellis.duplicate_handles, 0);
    }

    #[test]
    fn naive_side_drifts_for_replayable_chaos() {
        let state = run(LeakDuelRequest {
            seed: Some(1337),
            chaos: Some(8),
            ticks: Some(24),
            selected: None,
        });
        assert!(
            state.naive.delta != 0 || state.naive.orphaned > 0 || state.naive.duplicate_handles > 0
        );
    }
}
