use std::collections::BTreeSet;

use trellis_core::{ResourceCommandTrace, ResourceKey, Revision, ScopeId};

use crate::{
    HostStatusClass, HostStatusRecord, ResourceLedger, ResourceLedgerError, ResourceSnapshot,
    ResourceStatusContext,
};

impl<C> ResourceLedger<C> {
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
                return Err(ResourceLedgerError::Orphan {
                    key: key.clone(),
                    context: Some(snapshot.command_context()),
                });
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
        if let Some(context) = self.duplicate_closes.first() {
            Err(ResourceLedgerError::DuplicateClose {
                key: context.key.clone(),
                context: context.clone(),
            })
        } else {
            Ok(())
        }
    }

    /// Asserts no forbidden resource key was opened.
    pub fn assert_no_forbidden_opened(&self) -> Result<(), ResourceLedgerError> {
        if let Some(context) = self.forbidden_opened.first() {
            Err(ResourceLedgerError::ForbiddenOpen {
                key: context.key.clone(),
                context: Some(context.clone()),
            })
        } else {
            Ok(())
        }
    }

    /// Asserts no explicitly forbidden wildcard resource key was opened.
    pub fn assert_no_wildcard_resource_opened(&self) -> Result<(), ResourceLedgerError> {
        self.assert_no_forbidden_opened()
    }

    /// Asserts a resource is no longer open.
    pub fn assert_resource_not_open(&self, key: &ResourceKey) -> Result<(), ResourceLedgerError> {
        if self.resources.contains_key(key) {
            Err(ResourceLedgerError::StillOpen {
                key: key.clone(),
                context: self.context_for_key(key),
            })
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
            let contexts = resources
                .iter()
                .filter_map(|key| self.context_for_key(key))
                .collect();
            Err(ResourceLedgerError::ClosedScopeOwnsResources {
                scope,
                resources,
                contexts,
            })
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
                context: self.context_for_key(key),
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
                context: self.context_for_key(key),
            })
        }
    }

    /// Asserts a status for a command revision was classified as stale.
    pub fn assert_status_is_stale(
        &self,
        key: &ResourceKey,
        command_revision: Revision,
    ) -> Result<(), ResourceLedgerError> {
        let Some(record) = self.status_records.iter().find(|record| {
            record.status.resource_key == *key && record.status.command_revision == command_revision
        }) else {
            return Err(ResourceLedgerError::MissingStatus {
                key: key.clone(),
                command_revision,
            });
        };
        if record.class == HostStatusClass::Stale {
            Ok(())
        } else {
            Err(ResourceLedgerError::StatusClassMismatch {
                context: status_context(record),
                expected: HostStatusClass::Stale,
            })
        }
    }

    /// Asserts late statuses did not recreate ownership for a closed scope.
    pub fn assert_status_did_not_resurrect_closed_scope(
        &self,
        scope: ScopeId,
    ) -> Result<(), ResourceLedgerError> {
        self.assert_closed_scope_owns_no_resources(scope)?;
        self.assert_no_status_mutated_closed_scope()
    }

    /// Asserts status classification never mutated a closed scope's ownership.
    pub fn assert_no_status_mutated_closed_scope(&self) -> Result<(), ResourceLedgerError> {
        for record in &self.status_records {
            if record.class == HostStatusClass::Late
                && self.scope_owns_resource(record.status.scope, &record.status.resource_key)
            {
                return Err(ResourceLedgerError::StatusMutatedClosedScope {
                    scope: record.status.scope,
                    context: status_context(record),
                });
            }
        }
        Ok(())
    }

    fn assert_count(
        &self,
        key: &ResourceKey,
        field: &'static str,
        expected: usize,
        count: impl FnOnce(&ResourceSnapshot<C>) -> usize,
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
                context: self.context_for_key(key),
            })
        }
    }

    fn scope_owns_resource(&self, scope: ScopeId, key: &ResourceKey) -> bool {
        self.resources
            .get(key)
            .is_some_and(|snapshot| snapshot.owners.contains(&scope))
    }
}

fn status_context(record: &HostStatusRecord) -> ResourceStatusContext {
    ResourceStatusContext {
        status: record.status.clone(),
        class: record.class,
        last_transaction_id: record.last_transaction_id,
        last_command_revision: record.last_command_revision,
    }
}
