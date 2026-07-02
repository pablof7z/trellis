use std::collections::{BTreeMap, BTreeSet};

use trellis_core::{
    HostResourceCommandState, ResourceCommand, ResourceCommandTrace, ResourceKey, Revision,
    TransactionResult, classify_host_resource_status,
};

use crate::host_status::{HostStatusClass, HostStatusEvent, HostStatusIdentity, HostStatusRecord};
use crate::{ResourceCommandContext, ResourceCommandRecord, ResourceSnapshot};

/// Fake resource lifecycle ledger for applying Trellis resource plans.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceLedger<C = ()> {
    pub(crate) resources: BTreeMap<ResourceKey, ResourceSnapshot<C>>,
    pub(crate) history: BTreeMap<ResourceKey, ResourceSnapshot<C>>,
    pub(crate) duplicate_closes: Vec<ResourceCommandContext>,
    pub(crate) forbidden: BTreeSet<ResourceKey>,
    pub(crate) forbidden_opened: Vec<ResourceCommandContext>,
    pub(crate) accepted_status: BTreeSet<HostStatusIdentity>,
    pub(crate) status_records: Vec<HostStatusRecord>,
    pub(crate) command_trace: Vec<ResourceCommandTrace>,
    pub(crate) command_records: Vec<ResourceCommandRecord<C>>,
}

impl<C> Default for ResourceLedger<C> {
    fn default() -> Self {
        Self {
            resources: BTreeMap::new(),
            history: BTreeMap::new(),
            duplicate_closes: Vec::new(),
            forbidden: BTreeSet::new(),
            forbidden_opened: Vec::new(),
            accepted_status: BTreeSet::new(),
            status_records: Vec::new(),
            command_trace: Vec::new(),
            command_records: Vec::new(),
        }
    }
}

impl<C> ResourceLedger<C> {
    /// Creates an empty ledger.
    pub fn new() -> Self {
        Self::default()
    }

    /// Marks a key as forbidden unless the application explicitly permits it.
    pub fn mark_forbidden_unless_explicit(&mut self, key: ResourceKey) {
        self.forbidden.insert(key);
    }

    /// Returns the current snapshot for a resource.
    pub fn snapshot(&self, key: &ResourceKey) -> Option<&ResourceSnapshot<C>> {
        self.resources.get(key)
    }

    /// Returns the latest live or closed snapshot for a resource.
    pub fn history(&self, key: &ResourceKey) -> Option<&ResourceSnapshot<C>> {
        self.history.get(key)
    }

    /// Returns status classifications in delivery order.
    pub fn status_records(&self) -> &[HostStatusRecord] {
        &self.status_records
    }

    /// Returns applied resource command traces in delivery order.
    pub fn command_trace(&self) -> &[ResourceCommandTrace] {
        &self.command_trace
    }

    /// Returns applied command records with transaction/revision context.
    pub fn command_records(&self) -> &[ResourceCommandRecord<C>] {
        &self.command_records
    }

    pub(crate) fn context_for_key(&self, key: &ResourceKey) -> Option<ResourceCommandContext> {
        self.resources
            .get(key)
            .or_else(|| self.history.get(key))
            .map(ResourceSnapshot::command_context)
    }
}

impl<C: Clone> ResourceLedger<C> {
    /// Applies all resource commands from a transaction result.
    pub fn apply_result<O>(&mut self, result: &TransactionResult<C, O>) {
        self.command_trace.extend(result.trace().resource_commands);
        for command in result.resource_plan.commands() {
            self.apply_command(command, result.transaction_id, result.revision);
        }
    }

    /// Classifies a host status event without mutating graph state.
    pub fn classify_status(&mut self, status: HostStatusEvent) -> HostStatusClass {
        let (class, last_transaction_id, last_command_revision) = self.classify_status_ref(&status);
        if class == HostStatusClass::Current {
            self.accepted_status
                .insert(HostStatusIdentity::from(&status));
            if let Some(snapshot) = self.resources.get_mut(&status.resource_key) {
                snapshot.last_status_revision = Some(status.status_revision);
                snapshot.injected_status = Some(status.clone());
            }
            self.record_history(&status.resource_key);
        }
        self.status_records.push(HostStatusRecord {
            status,
            class,
            last_transaction_id,
            last_command_revision,
        });
        class
    }

    fn apply_command(
        &mut self,
        command: &ResourceCommand<C>,
        transaction_id: trellis_core::TransactionId,
        revision: Revision,
    ) {
        let generation = self.next_generation(command.key());
        let record =
            ResourceCommandRecord::from_command(command, transaction_id, revision, generation);
        self.command_records.push(record.clone());
        match command {
            ResourceCommand::Open { key, scope, .. } => {
                if self.forbidden.contains(key) {
                    self.forbidden_opened.push(record.context.clone());
                }
                let snapshot = self.ensure_snapshot(key, record);
                snapshot.owners.insert(*scope);
                snapshot.is_open = true;
                snapshot.open_count += 1;
                self.record_history(key);
            }
            ResourceCommand::Close { key, scope } => {
                let Some(snapshot) = self.resources.get_mut(key) else {
                    self.duplicate_closes.push(record.context);
                    return;
                };
                if !snapshot.owners.remove(scope) {
                    self.duplicate_closes.push(record.context.clone());
                }
                snapshot.close_count += 1;
                snapshot.record_command(record);
                if snapshot.owners.is_empty() {
                    snapshot.is_open = false;
                    self.record_history(key);
                    self.resources.remove(key);
                } else {
                    self.record_history(key);
                }
            }
            ResourceCommand::Replace { key, scope, .. } => {
                let snapshot = self.ensure_snapshot(key, record);
                snapshot.owners.insert(*scope);
                snapshot.is_open = true;
                snapshot.replace_count += 1;
                self.record_history(key);
            }
            ResourceCommand::Refresh { key, .. } => {
                if let Some(snapshot) = self.resources.get_mut(key) {
                    snapshot.record_command(record);
                    self.record_history(key);
                }
            }
        }
    }

    fn ensure_snapshot(
        &mut self,
        key: &ResourceKey,
        record: ResourceCommandRecord<C>,
    ) -> &mut ResourceSnapshot<C> {
        let previous = self.history.get(key).cloned();
        let snapshot = self
            .resources
            .entry(key.clone())
            .or_insert_with(|| previous.unwrap_or_else(|| ResourceSnapshot::new(record.clone())));
        snapshot.record_command(record);
        snapshot
    }

    fn classify_status_ref(
        &self,
        status: &HostStatusEvent,
    ) -> (
        HostStatusClass,
        Option<trellis_core::TransactionId>,
        Option<Revision>,
    ) {
        let known = self.resources.get(&status.resource_key);
        let historical = known.or_else(|| self.history.get(&status.resource_key));
        let last_transaction_id = historical.map(|snapshot| snapshot.last_transaction_id);
        let last_command_revision = historical.map(|snapshot| snapshot.command_revision);
        let state = if let Some(snapshot) = known {
            Some(HostResourceCommandState {
                scope: snapshot.last_command.context.scope,
                command_revision: snapshot.command_revision,
                resource_is_live: true,
                scope_owns_resource: snapshot.owners.contains(&status.scope),
            })
        } else {
            historical.map(|snapshot| HostResourceCommandState {
                scope: snapshot.last_command.context.scope,
                command_revision: snapshot.command_revision,
                resource_is_live: false,
                scope_owns_resource: false,
            })
        };
        let duplicate = self
            .accepted_status
            .contains(&HostStatusIdentity::from(status));
        (
            classify_host_resource_status(status, state, duplicate),
            last_transaction_id,
            last_command_revision,
        )
    }

    fn next_generation(&self, key: &ResourceKey) -> u64 {
        self.resources
            .get(key)
            .or_else(|| self.history.get(key))
            .map_or(1, |snapshot| snapshot.generation + 1)
    }

    fn record_history(&mut self, key: &ResourceKey) {
        if let Some(snapshot) = self.resources.get(key) {
            self.history.insert(key.clone(), snapshot.clone());
        }
    }
}
