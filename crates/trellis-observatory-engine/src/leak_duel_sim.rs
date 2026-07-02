use std::collections::{BTreeMap, BTreeSet};

use crate::leak_duel::ChatInputs;

#[derive(Clone)]
pub(crate) struct ChaosEvent {
    pub(crate) kind: EventKind,
    pub(crate) label: String,
    pub(crate) detail: String,
    noisy: bool,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum EventKind {
    JoinRoom,
    LeaveRoom,
    GrantRoom,
    RevokeRoom,
    SwitchWorkspace,
    Offline,
    Reconnect,
    ToggleFollow,
}

pub(crate) struct Rng(u64);

pub(crate) fn initial_inputs() -> ChatInputs {
    ChatInputs {
        workspace: "atlas".to_owned(),
        joined_rooms: set(&["ops", "design"]),
        permission_grants: set(&["ops", "design"]),
        follows: set(&["ava", "bo"]),
        network_online: true,
    }
}

pub(crate) fn desired_attachments(inputs: &ChatInputs) -> BTreeSet<String> {
    if !inputs.network_online {
        return BTreeSet::new();
    }
    let mut out = BTreeSet::new();
    for room in inputs.joined_rooms.intersection(&inputs.permission_grants) {
        for user in &inputs.follows {
            out.insert(format!("attachment:{}:{}:{user}", inputs.workspace, room));
        }
    }
    out
}

pub(crate) fn next_event(rng: &mut Rng, tick: u32, chaos: u8, inputs: &ChatInputs) -> ChaosEvent {
    let roll = ((rng.next() + tick as u64 + chaos as u64) % 8) as u8;
    let noisy = rng.next().is_multiple_of(3) || chaos > 7;
    match roll {
        0 => event(
            EventKind::JoinRoom,
            "Join room",
            format!("joined {}", pick_room(rng)),
            noisy,
        ),
        1 => event(
            EventKind::LeaveRoom,
            "Leave room",
            format!("left {}", existing_room(rng, inputs)),
            noisy,
        ),
        2 => event(
            EventKind::GrantRoom,
            "Grant permission",
            format!("granted {}", pick_room(rng)),
            noisy,
        ),
        3 => event(
            EventKind::RevokeRoom,
            "Revoke permission",
            format!("revoked {}", existing_grant(rng, inputs)),
            noisy,
        ),
        4 => event(
            EventKind::SwitchWorkspace,
            "Switch workspace",
            "workspace changed while opens were in flight".to_owned(),
            noisy,
        ),
        5 => event(
            EventKind::Offline,
            "Network offline",
            "socket dropped; all live attachment streams should close".to_owned(),
            noisy,
        ),
        6 => event(
            EventKind::Reconnect,
            "Network reconnect",
            "socket reconnect replayed open callbacks".to_owned(),
            true,
        ),
        _ => event(
            EventKind::ToggleFollow,
            "Follow set changed",
            format!("toggled {}", pick_user(rng)),
            noisy,
        ),
    }
}

pub(crate) fn apply_event(inputs: &mut ChatInputs, event: &ChaosEvent) {
    match event.kind {
        EventKind::JoinRoom => {
            inputs.joined_rooms.insert(last_word(&event.detail));
        }
        EventKind::LeaveRoom => {
            inputs.joined_rooms.remove(&last_word(&event.detail));
        }
        EventKind::GrantRoom => {
            inputs.permission_grants.insert(last_word(&event.detail));
        }
        EventKind::RevokeRoom => {
            inputs.permission_grants.remove(&last_word(&event.detail));
        }
        EventKind::SwitchWorkspace => {
            inputs.workspace = if inputs.workspace == "atlas" {
                "beacon"
            } else {
                "atlas"
            }
            .to_owned();
        }
        EventKind::Offline => inputs.network_online = false,
        EventKind::Reconnect => inputs.network_online = true,
        EventKind::ToggleFollow => {
            let user = last_word(&event.detail);
            if !inputs.follows.remove(&user) {
                inputs.follows.insert(user);
            }
        }
    }
}

pub(crate) fn apply_naive(
    open: &mut BTreeMap<String, u32>,
    before: &BTreeSet<String>,
    after: &BTreeSet<String>,
    event: &ChaosEvent,
    chaos: u8,
) -> String {
    let mut notes = Vec::new();
    for key in after.difference(before) {
        *open.entry(key.clone()).or_insert(0) += 1;
    }
    let skip_close = chaos >= 4
        && event.noisy
        && matches!(
            event.kind,
            EventKind::LeaveRoom
                | EventKind::RevokeRoom
                | EventKind::SwitchWorkspace
                | EventKind::Offline
        );
    for key in before.difference(after) {
        if skip_close {
            notes.push(format!("missed close for {}", short_key(key)));
        } else {
            close_one(open, key);
        }
    }
    if chaos >= 6 && event.kind == EventKind::Reconnect {
        for key in after.iter().take((chaos / 4).max(1) as usize) {
            *open.entry(key.clone()).or_insert(0) += 1;
            notes.push(format!("double-opened {}", short_key(key)));
        }
    }
    if notes.is_empty() {
        "handlers happened to balance for this event".to_owned()
    } else {
        notes.join("; ")
    }
}

pub(crate) fn counted(keys: &BTreeSet<String>) -> BTreeMap<String, u32> {
    keys.iter().map(|key| (key.clone(), 1)).collect()
}

fn close_one(open: &mut BTreeMap<String, u32>, key: &str) {
    if let Some(count) = open.get_mut(key) {
        *count = count.saturating_sub(1);
        if *count == 0 {
            open.remove(key);
        }
    }
}

fn set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn event(kind: EventKind, label: &str, detail: String, noisy: bool) -> ChaosEvent {
    ChaosEvent {
        kind,
        label: label.to_owned(),
        detail,
        noisy,
    }
}

fn pick_room(rng: &mut Rng) -> String {
    pick(rng, &["ops", "design", "infra", "billing"])
}

fn existing_room(rng: &mut Rng, inputs: &ChatInputs) -> String {
    inputs
        .joined_rooms
        .iter()
        .next()
        .cloned()
        .unwrap_or_else(|| pick_room(rng))
}

fn existing_grant(rng: &mut Rng, inputs: &ChatInputs) -> String {
    inputs
        .permission_grants
        .iter()
        .next()
        .cloned()
        .unwrap_or_else(|| pick_room(rng))
}

fn pick_user(rng: &mut Rng) -> String {
    pick(rng, &["ava", "bo", "cy", "dee"])
}

fn pick(rng: &mut Rng, values: &[&str]) -> String {
    values[(rng.next() as usize) % values.len()].to_owned()
}

fn last_word(value: &str) -> String {
    value.split_whitespace().last().unwrap_or(value).to_owned()
}

pub(crate) fn short_key(key: &str) -> String {
    key.strip_prefix("attachment:")
        .unwrap_or(key)
        .replace(':', " / ")
}

impl Rng {
    pub(crate) fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }
}
