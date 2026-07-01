use std::collections::BTreeSet;

use trellis_core::{
    ResourceCommand, ResourceCommandKind, ResourceTransitionPolicy, Revision, ScopeId,
    TransactionId,
};

use crate::{HostStatusEvent, ResourceCommandContext};

/// Applied resource command with transaction and generation context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceCommandRecord<C = ()> {
    /// Context common to all resource command assertions.
    pub context: ResourceCommandContext,
    /// Host-facing transition policy requested by the command.
    pub transition: ResourceTransitionPolicy,
    /// Retained host command payload for open/replace/refresh commands.
    pub command: Option<C>,
}

impl<C: Clone> ResourceCommandRecord<C> {
    pub(crate) fn from_command(
        command: &ResourceCommand<C>,
        transaction_id: TransactionId,
        revision: Revision,
        generation: u64,
    ) -> Self {
        Self {
            context: ResourceCommandContext {
                key: command.key().clone(),
                scope: command.scope(),
                transaction_id,
                revision,
                generation,
                kind: resource_command_kind(command),
            },
            transition: resource_transition_policy(command),
            command: retained_payload(command),
        }
    }
}

/// Current or historical ledger view for one resource key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceSnapshot<C = ()> {
    /// Scopes that currently own the resource.
    pub owners: BTreeSet<ScopeId>,
    /// Whether the resource is currently open.
    pub is_open: bool,
    /// Number of open commands observed.
    pub open_count: usize,
    /// Number of close commands observed.
    pub close_count: usize,
    /// Number of replace commands observed.
    pub replace_count: usize,
    /// Latest command revision observed for this key.
    pub command_revision: Revision,
    /// Monotonic command generation assigned by the ledger.
    pub generation: u64,
    /// Last transaction that emitted a command for this key.
    pub last_transaction_id: TransactionId,
    /// Last accepted host status revision for this key.
    pub last_status_revision: Option<Revision>,
    /// Latest retained host command payload for this key.
    pub current_command: Option<C>,
    /// Latest injected host status accepted for this key.
    pub injected_status: Option<HostStatusEvent>,
    /// Last applied command record for this key.
    pub last_command: ResourceCommandRecord<C>,
}

impl<C: Clone> ResourceSnapshot<C> {
    pub(crate) fn new(record: ResourceCommandRecord<C>) -> Self {
        Self {
            owners: BTreeSet::new(),
            is_open: true,
            open_count: 0,
            close_count: 0,
            replace_count: 0,
            command_revision: record.context.revision,
            generation: record.context.generation,
            last_transaction_id: record.context.transaction_id,
            last_status_revision: None,
            current_command: record.command.clone(),
            injected_status: None,
            last_command: record,
        }
    }

    pub(crate) fn record_command(&mut self, record: ResourceCommandRecord<C>) {
        self.command_revision = record.context.revision;
        self.generation = record.context.generation;
        self.last_transaction_id = record.context.transaction_id;
        self.current_command = record.command.clone();
        self.last_command = record;
    }
}

impl<C> ResourceSnapshot<C> {
    pub(crate) fn command_context(&self) -> ResourceCommandContext {
        self.last_command.context.clone()
    }
}

fn retained_payload<C: Clone>(command: &ResourceCommand<C>) -> Option<C> {
    match command {
        ResourceCommand::Open { command, .. }
        | ResourceCommand::Replace { command, .. }
        | ResourceCommand::Refresh { command, .. } => Some(command.clone()),
        ResourceCommand::Close { .. } => None,
    }
}

fn resource_command_kind<C>(command: &ResourceCommand<C>) -> ResourceCommandKind {
    match command {
        ResourceCommand::Open { .. } => ResourceCommandKind::Open,
        ResourceCommand::Close { .. } => ResourceCommandKind::Close,
        ResourceCommand::Replace { .. } => ResourceCommandKind::Replace,
        ResourceCommand::Refresh { .. } => ResourceCommandKind::Refresh,
    }
}

fn resource_transition_policy<C>(command: &ResourceCommand<C>) -> ResourceTransitionPolicy {
    match command {
        ResourceCommand::Open { .. } => ResourceTransitionPolicy::Open,
        ResourceCommand::Close { .. } => ResourceTransitionPolicy::Close,
        ResourceCommand::Replace { .. } => ResourceTransitionPolicy::ReplaceAtomically,
        ResourceCommand::Refresh { .. } => ResourceTransitionPolicy::Refresh,
    }
}
