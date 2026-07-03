use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
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
    pub proof: Value,
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
    last_action: BTreeMap<String, Value>,
    last_commands: Vec<Value>,
    transaction_id: u64,
    revision: u64,
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
    let mut desired_diff = diff(&BTreeSet::new(), &initial);
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
        desired_diff = diff(&before, &after);
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
    let receipt_actions = trellis
        .last_action
        .iter()
        .map(|(key, command)| {
            let cause = command
                .get("cause")
                .and_then(Value::as_str)
                .unwrap_or("no prior command")
                .to_owned();
            (key.clone(), cause)
        })
        .collect();

    LeakDuelState {
        seed,
        chaos,
        tick: ticks,
        inputs: inputs.clone(),
        should_open: should.iter().cloned().collect(),
        rows: rows(&should, &naive_open, &trellis_open),
        naive: stats(&naive_open, &should, "drift"),
        trellis: stats(&trellis_open, &should, "reconciled"),
        activity,
        selected_receipt: receipt_for(&selected, &inputs, &should, &receipt_actions),
        proof: proof_for(ProofInput {
            seed,
            chaos,
            ticks,
            selected,
            desired_diff,
            naive: stats(&naive_open, &should, "drift"),
            trellis: stats(&trellis_open, &should, "reconciled"),
            harness: &trellis,
        }),
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
            last_commands: Vec::new(),
            transaction_id: 0,
            revision: 0,
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
        let transaction_id = result.transaction_id.get();
        let revision = result.revision.get();
        let mut commands = Vec::new();
        for command in result.resource_plan.commands() {
            match command {
                ResourceCommand::Open {
                    key,
                    scope,
                    command,
                } => {
                    open.insert(key.as_str().to_owned(), 1);
                    let proof = command_proof(CommandProofInput {
                        kind: "OPEN",
                        key: key.as_str(),
                        label: command.key.as_str(),
                        scope_id: scope.get(),
                        transaction_id,
                        revision,
                        event_label: label,
                        cause: "desiredAttachments added this key",
                        applied: "open=1",
                    });
                    self.last_action
                        .insert(key.as_str().to_owned(), proof.clone());
                    commands.push(proof);
                    opens += 1;
                }
                ResourceCommand::Close { key, scope } => {
                    open.remove(key.as_str());
                    let label_text = short_label(key.as_str());
                    let proof = command_proof(CommandProofInput {
                        kind: "CLOSE",
                        key: key.as_str(),
                        label: &label_text,
                        scope_id: scope.get(),
                        transaction_id,
                        revision,
                        event_label: label,
                        cause: "desiredAttachments removed this key",
                        applied: "open=0",
                    });
                    self.last_action
                        .insert(key.as_str().to_owned(), proof.clone());
                    commands.push(proof);
                    closes += 1;
                }
                _ => {}
            }
        }
        self.last_commands = commands;
        self.transaction_id = transaction_id;
        self.revision = revision;
        format!("{opens} open, {closes} close from the resource plan")
    }
}

struct ProofInput<'a> {
    seed: u64,
    chaos: u8,
    ticks: u32,
    selected: String,
    desired_diff: Value,
    naive: SideStats,
    trellis: SideStats,
    harness: &'a TrellisHarness,
}

fn proof_for(input: ProofInput<'_>) -> Value {
    json!({
        "wasmBundle": "demos/leak-duel/engine/trellis_observatory_engine_bg.wasm",
        "rustSource": "crates/trellis-observatory-engine/src/leak_duel.rs",
        "uiSource": "demos/leak-duel/leak-duel.js",
        "inputNode": "chatInputs",
        "collection": "desiredAttachments",
        "scope": "chat-session",
        "transactionId": input.harness.transaction_id,
        "revision": input.harness.revision,
        "deterministicReplay": format!(
            "seed={} chaos={} ticks={} selected={}",
            input.seed, input.chaos, input.ticks, input.selected
        ),
        "desiredDiff": input.desired_diff,
        "currentCommands": input.harness.last_commands.clone(),
        "selectedCommand": input.harness.last_action.get(&input.selected).cloned(),
        "invariants": invariants(&input.naive, &input.trellis),
    })
}

fn diff(before: &BTreeSet<String>, after: &BTreeSet<String>) -> Value {
    json!({
        "added": after.difference(before).cloned().collect::<Vec<_>>(),
        "removed": before.difference(after).cloned().collect::<Vec<_>>(),
        "unchanged": after.intersection(before).cloned().collect::<Vec<_>>(),
    })
}

fn invariants(naive: &SideStats, trellis: &SideStats) -> Vec<Value> {
    vec![
        invariant(
            "Trellis open == shouldOpen",
            trellis.open == trellis.should_open,
        ),
        invariant("Trellis delta == 0", trellis.delta == 0),
        invariant("Trellis orphaned == 0", trellis.orphaned == 0),
        invariant(
            "Trellis duplicateHandles == 0",
            trellis.duplicate_handles == 0,
        ),
        json!({
            "name": "callback drift observed",
            "passed": naive.delta != 0 || naive.orphaned > 0 || naive.duplicate_handles > 0,
            "detail": format!(
                "delta={}, orphaned={}, duplicateHandles={}",
                naive.delta, naive.orphaned, naive.duplicate_handles
            ),
        }),
    ]
}

fn invariant(name: &str, passed: bool) -> Value {
    json!({ "name": name, "passed": passed, "detail": if passed { "pass" } else { "fail" } })
}

struct CommandProofInput<'a> {
    kind: &'a str,
    key: &'a str,
    label: &'a str,
    scope_id: u64,
    transaction_id: u64,
    revision: u64,
    event_label: &'a str,
    cause: &'a str,
    applied: &'a str,
}

fn command_proof(input: CommandProofInput<'_>) -> Value {
    json!({
        "kind": input.kind,
        "key": input.key,
        "label": input.label,
        "scope": "chat-session",
        "scopeId": input.scope_id,
        "transactionId": input.transaction_id,
        "revision": input.revision,
        "cause": format!(
            "{} -> {} -> [{}] {}",
            input.event_label, input.cause, input.kind, input.label
        ),
        "appliedLedgerResult": input.applied,
    })
}

fn short_label(key: &str) -> String {
    key.strip_prefix("attachment:")
        .unwrap_or(key)
        .replace(':', " / ")
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
