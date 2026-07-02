use std::collections::{BTreeMap, BTreeSet};

use crate::leak_duel::{AttachmentRow, ChatInputs, Receipt, ReceiptStep, SideStats};
use crate::leak_duel_sim::short_key;

pub(crate) fn receipt_for(
    key: &str,
    inputs: &ChatInputs,
    should: &BTreeSet<String>,
    last_action: &BTreeMap<String, String>,
) -> Receipt {
    let present = should.contains(key);
    let title = if present {
        "why Trellis keeps this open"
    } else {
        "why Trellis does not keep this open"
    };
    let action = last_action
        .get(key)
        .cloned()
        .unwrap_or_else(|| "no open command in the current desired set".to_owned());
    Receipt {
        key: key.to_owned(),
        title: title.to_owned(),
        status: if present { "owned" } else { "not-owned" }.to_owned(),
        steps: vec![
            ReceiptStep {
                label: "canonical input".to_owned(),
                detail: format!(
                    "workspace={}, rooms={:?}, grants={:?}, follows={:?}, online={}",
                    inputs.workspace,
                    inputs.joined_rooms,
                    inputs.permission_grants,
                    inputs.follows,
                    inputs.network_online
                ),
            },
            ReceiptStep {
                label: "derived collection".to_owned(),
                detail: format!(
                    "desiredAttachments {} {}",
                    if present {
                        "contains"
                    } else {
                        "does not contain"
                    },
                    key
                ),
            },
            ReceiptStep {
                label: "resource plan".to_owned(),
                detail: action,
            },
        ],
    }
}

pub(crate) fn rows(
    should: &BTreeSet<String>,
    naive: &BTreeMap<String, u32>,
    trellis: &BTreeMap<String, u32>,
) -> Vec<AttachmentRow> {
    let mut keys = should.clone();
    keys.extend(naive.keys().cloned());
    keys.extend(trellis.keys().cloned());
    keys.into_iter()
        .map(|key| AttachmentRow {
            label: short_key(&key),
            should_open: should.contains(&key),
            naive_open: *naive.get(&key).unwrap_or(&0),
            trellis_open: *trellis.get(&key).unwrap_or(&0),
            key,
        })
        .collect()
}

pub(crate) fn stats(
    open: &BTreeMap<String, u32>,
    should: &BTreeSet<String>,
    ok_word: &str,
) -> SideStats {
    let actual = open.values().sum::<u32>();
    let orphaned = open.keys().filter(|key| !should.contains(*key)).count() as u32;
    let duplicate_handles = open.values().map(|count| count.saturating_sub(1)).sum();
    let delta = actual as i32 - should.len() as i32;
    SideStats {
        open: actual,
        should_open: should.len() as u32,
        delta,
        orphaned,
        duplicate_handles,
        verdict: if delta == 0 && orphaned == 0 && duplicate_handles == 0 {
            ok_word.to_owned()
        } else {
            "drifting".to_owned()
        },
    }
}
