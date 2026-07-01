use std::collections::{BTreeMap, BTreeSet};

use trellis_core::{ResourceCommand, ResourceKey, Revision, ScopeId, TransactionResult};

/// Current ledger view for one resource key.
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

/// Explicit host status event fed to tests after plan application.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostStatusEvent {
    /// Resource key targeted by the status.
    pub key: ResourceKey,
    /// Scope targeted by the status.
    pub scope: ScopeId,
    /// Command revision the host believes it is reporting for.
    pub command_revision: Revision,
    /// Revision of the status observation.
    pub status_revision: Revision,
}

/// Classification for host status delivery.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum HostStatusClass {
    /// Status matches the current resource/scope/revision.
    Current,
    /// Status duplicates the last accepted status revision.
    Duplicate,
    /// Status targets an old command revision.
    Stale,
    /// Status targets a command revision newer than the ledger has observed.
    Future,
    /// Status targets a scope that no longer owns the resource.
    Late,
}

/// Resource ledger assertion failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResourceLedgerError {
    /// Resource has no owner.
    Orphan(ResourceKey),
    /// Resource was closed without a matching owner.
    DuplicateClose(ResourceKey),
    /// Forbidden resource demand was opened.
    ForbiddenOpen(ResourceKey),
    /// Resource is still open.
    StillOpen(ResourceKey),
    /// Resource does not have the expected owners.
    OwnerMismatch {
        /// Resource key.
        key: ResourceKey,
        /// Expected owner set.
        expected: BTreeSet<ScopeId>,
        /// Actual owner set.
        actual: BTreeSet<ScopeId>,
    },
}

/// Fake resource lifecycle ledger for applying Trellis resource plans.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ResourceLedger {
    resources: BTreeMap<ResourceKey, ResourceSnapshot>,
    duplicate_closes: BTreeSet<ResourceKey>,
    forbidden: BTreeSet<ResourceKey>,
    forbidden_opened: BTreeSet<ResourceKey>,
    accepted_status: BTreeMap<ResourceKey, Revision>,
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
        for command in result.resource_plan.commands() {
            self.apply_command(command, result.revision);
        }
    }

    /// Returns the current snapshot for a resource.
    pub fn snapshot(&self, key: &ResourceKey) -> Option<&ResourceSnapshot> {
        self.resources.get(key)
    }

    /// Classifies a host status event without mutating graph state.
    pub fn classify_status(&mut self, status: HostStatusEvent) -> HostStatusClass {
        let Some(snapshot) = self.resources.get(&status.key) else {
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
        if self.accepted_status.get(&status.key) == Some(&status.status_revision) {
            return HostStatusClass::Duplicate;
        }
        self.accepted_status
            .insert(status.key, status.status_revision);
        HostStatusClass::Current
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
                    self.resources.remove(key);
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
            }
            ResourceCommand::Refresh { key, .. } => {
                if let Some(snapshot) = self.resources.get_mut(key) {
                    snapshot.command_revision = revision;
                    snapshot.generation += 1;
                }
            }
        }
    }
}
