use std::collections::{BTreeMap, BTreeSet};

use trellis_core::{
    ResourceCommand, ResourceCommandTrace, ResourceKey, Revision, ScopeId, TransactionResult,
};

use crate::ResourceLedgerError;
use crate::host_status::{HostStatusClass, HostStatusEvent, HostStatusIdentity, HostStatusRecord};

/// Current or historical ledger view for one resource key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceSnapshot {
    /// Scopes that currently own the resource.
    pub owners: BTreeSet<ScopeId>,
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
}

/// Fake resource lifecycle ledger for applying Trellis resource plans.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ResourceLedger {
    resources: BTreeMap<ResourceKey, ResourceSnapshot>,
    history: BTreeMap<ResourceKey, ResourceSnapshot>,
    duplicate_closes: BTreeSet<ResourceKey>,
    forbidden: BTreeSet<ResourceKey>,
    forbidden_opened: BTreeSet<ResourceKey>,
    accepted_status: BTreeSet<HostStatusIdentity>,
    status_records: Vec<HostStatusRecord>,
    command_trace: Vec<ResourceCommandTrace>,
}

impl ResourceLedger {
    /// Creates an empty ledger.
    pub fn new() -> Self {
        Self::default()
    }

    /// Marks a key as forbidden unless the application explicitly permits it.
    pub fn mark_forbidden_unless_explicit(&mut self, key: ResourceKey) {
        self.forbidden.insert(key);
    }

    /// Applies all resource commands from a transaction result.
    pub fn apply_result<C, O>(&mut self, result: &TransactionResult<C, O>) {
        self.command_trace.extend(result.trace().resource_commands);
        for command in result.resource_plan.commands() {
            self.apply_command(command, result.revision);
        }
    }

    /// Returns the current snapshot for a resource.
    pub fn snapshot(&self, key: &ResourceKey) -> Option<&ResourceSnapshot> {
        self.resources.get(key)
    }

    /// Returns the latest live or closed snapshot for a resource.
    pub fn history(&self, key: &ResourceKey) -> Option<&ResourceSnapshot> {
        self.history.get(key)
    }

    /// Classifies a host status event without mutating graph state.
    pub fn classify_status(&mut self, status: HostStatusEvent) -> HostStatusClass {
        let class = self.classify_status_ref(&status);
        if class == HostStatusClass::Current {
            self.accepted_status
                .insert(HostStatusIdentity::from(&status));
        }
        self.status_records.push(HostStatusRecord { status, class });
        class
    }

    /// Returns status classifications in delivery order.
    pub fn status_records(&self) -> &[HostStatusRecord] {
        &self.status_records
    }

    /// Returns applied resource command traces in delivery order.
    pub fn command_trace(&self) -> &[ResourceCommandTrace] {
        &self.command_trace
    }

    /// Asserts the full applied resource command order.
    pub fn assert_command_order(
        &self,
        expected: &[ResourceCommandTrace],
    ) -> Result<(), ResourceLedgerError> {
        if self.command_trace == expected {
            Ok(())
        } else {
            Err(ResourceLedgerError::CommandOrderMismatch {
                expected: expected.to_vec(),
                actual: self.command_trace.clone(),
            })
        }
    }

    /// Asserts every tracked resource still has at least one owner.
    pub fn assert_all_resources_have_owner(&self) -> Result<(), ResourceLedgerError> {
        for (key, snapshot) in &self.resources {
            if snapshot.owners.is_empty() {
                return Err(ResourceLedgerError::Orphan(key.clone()));
            }
        }
        Ok(())
    }

    /// Asserts every tracked live resource has at least one owner.
    pub fn assert_no_orphan_resources(&self) -> Result<(), ResourceLedgerError> {
        self.assert_all_resources_have_owner()
    }

    /// Asserts no duplicate close was observed.
    pub fn assert_no_duplicate_close(&self) -> Result<(), ResourceLedgerError> {
        if let Some(key) = self.duplicate_closes.iter().next() {
            Err(ResourceLedgerError::DuplicateClose(key.clone()))
        } else {
            Ok(())
        }
    }

    /// Asserts no forbidden resource key was opened.
    pub fn assert_no_forbidden_opened(&self) -> Result<(), ResourceLedgerError> {
        if let Some(key) = self.forbidden_opened.iter().next() {
            Err(ResourceLedgerError::ForbiddenOpen(key.clone()))
        } else {
            Ok(())
        }
    }

    /// Asserts a resource is no longer open.
    pub fn assert_resource_not_open(&self, key: &ResourceKey) -> Result<(), ResourceLedgerError> {
        if self.resources.contains_key(key) {
            Err(ResourceLedgerError::StillOpen(key.clone()))
        } else {
            Ok(())
        }
    }

    /// Asserts a closed scope owns no live resources.
    pub fn assert_closed_scope_owns_no_resources(
        &self,
        scope: ScopeId,
    ) -> Result<(), ResourceLedgerError> {
        let resources = self
            .resources
            .iter()
            .filter(|(_, snapshot)| snapshot.owners.contains(&scope))
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        if resources.is_empty() {
            Ok(())
        } else {
            Err(ResourceLedgerError::ClosedScopeOwnsResources { scope, resources })
        }
    }

    /// Asserts a resource was opened exactly once.
    pub fn assert_resource_opened_once(
        &self,
        key: &ResourceKey,
    ) -> Result<(), ResourceLedgerError> {
        self.assert_count(key, "open_count", 1, |snapshot| snapshot.open_count)
    }

    /// Asserts a resource was closed exactly once.
    pub fn assert_resource_closed_once(
        &self,
        key: &ResourceKey,
    ) -> Result<(), ResourceLedgerError> {
        self.assert_count(key, "close_count", 1, |snapshot| snapshot.close_count)
    }

    /// Asserts a resource has the expected command generation.
    pub fn assert_resource_generation(
        &self,
        key: &ResourceKey,
        expected: u64,
    ) -> Result<(), ResourceLedgerError> {
        let actual = self
            .history
            .get(key)
            .map_or(0, |snapshot| snapshot.generation);
        if actual == expected {
            Ok(())
        } else {
            Err(ResourceLedgerError::GenerationMismatch {
                key: key.clone(),
                expected,
                actual,
            })
        }
    }

    /// Asserts a resource is owned by the expected scopes.
    pub fn assert_resource_shared_by(
        &self,
        key: &ResourceKey,
        expected: BTreeSet<ScopeId>,
    ) -> Result<(), ResourceLedgerError> {
        let actual = self
            .resources
            .get(key)
            .map(|snapshot| snapshot.owners.clone())
            .unwrap_or_default();
        if actual == expected {
            Ok(())
        } else {
            Err(ResourceLedgerError::OwnerMismatch {
                key: key.clone(),
                expected,
                actual,
            })
        }
    }

    fn apply_command<C>(&mut self, command: &ResourceCommand<C>, revision: Revision) {
        match command {
            ResourceCommand::Open { key, scope, .. } => {
                if self.forbidden.contains(key) {
                    self.forbidden_opened.insert(key.clone());
                }
                let snapshot = self
                    .resources
                    .entry(key.clone())
                    .or_insert(ResourceSnapshot {
                        owners: BTreeSet::new(),
                        open_count: 0,
                        close_count: 0,
                        replace_count: 0,
                        command_revision: revision,
                        generation: 0,
                    });
                snapshot.owners.insert(*scope);
                snapshot.open_count += 1;
                snapshot.command_revision = revision;
                snapshot.generation += 1;
                self.record_history(key);
            }
            ResourceCommand::Close { key, scope } => {
                let Some(snapshot) = self.resources.get_mut(key) else {
                    self.duplicate_closes.insert(key.clone());
                    return;
                };
                if !snapshot.owners.remove(scope) {
                    self.duplicate_closes.insert(key.clone());
                }
                snapshot.close_count += 1;
                snapshot.command_revision = revision;
                snapshot.generation += 1;
                if snapshot.owners.is_empty() {
                    self.record_history(key);
                    self.resources.remove(key);
                } else {
                    self.record_history(key);
                }
            }
            ResourceCommand::Replace { key, scope, .. } => {
                let snapshot = self
                    .resources
                    .entry(key.clone())
                    .or_insert(ResourceSnapshot {
                        owners: BTreeSet::new(),
                        open_count: 0,
                        close_count: 0,
                        replace_count: 0,
                        command_revision: revision,
                        generation: 0,
                    });
                snapshot.owners.insert(*scope);
                snapshot.replace_count += 1;
                snapshot.command_revision = revision;
                snapshot.generation += 1;
                self.record_history(key);
            }
            ResourceCommand::Refresh { key, .. } => {
                if let Some(snapshot) = self.resources.get_mut(key) {
                    snapshot.command_revision = revision;
                    snapshot.generation += 1;
                    self.record_history(key);
                }
            }
        }
    }

    fn classify_status_ref(&self, status: &HostStatusEvent) -> HostStatusClass {
        let Some(snapshot) = self.resources.get(&status.resource_key) else {
            return HostStatusClass::Late;
        };
        if !snapshot.owners.contains(&status.scope) {
            return HostStatusClass::Late;
        }
        if status.command_revision < snapshot.command_revision {
            return HostStatusClass::Stale;
        }
        if status.command_revision > snapshot.command_revision {
            return HostStatusClass::Future;
        }
        if self
            .accepted_status
            .contains(&HostStatusIdentity::from(status))
        {
            return HostStatusClass::Duplicate;
        }
        HostStatusClass::Current
    }

    fn assert_count(
        &self,
        key: &ResourceKey,
        field: &'static str,
        expected: usize,
        count: impl FnOnce(&ResourceSnapshot) -> usize,
    ) -> Result<(), ResourceLedgerError> {
        let actual = self.history.get(key).map_or(0, count);
        if actual == expected {
            Ok(())
        } else {
            Err(ResourceLedgerError::CountMismatch {
                key: key.clone(),
                field,
                expected,
                actual,
            })
        }
    }

    fn record_history(&mut self, key: &ResourceKey) {
        if let Some(snapshot) = self.resources.get(key) {
            self.history.insert(key.clone(), snapshot.clone());
        }
    }
}
