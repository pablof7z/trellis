use trellis_core::{
    HostResourceOutcome, ResourceCommand, ResourceKey, Revision, ScopeId, TransactionResult,
};

use crate::{HostStatusClass, HostStatusEvent, ResourceLedger};

/// Status event produced by the fake host boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FakeHostEvent {
    /// Explicit event that application tests feed back as canonical input.
    pub status: HostStatusEvent,
    /// Ledger classification for the event at the time it was produced.
    pub class: HostStatusClass,
}

/// Deterministic fake host boundary for resource status simulations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FakeHost {
    next_status_revision: u64,
}

impl FakeHost {
    /// Creates a fake host with status revisions starting at one.
    pub const fn new() -> Self {
        Self {
            next_status_revision: 1,
        }
    }

    /// Applies a transaction result to the ledger and returns explicit statuses.
    pub fn apply_result<C, O>(
        &mut self,
        ledger: &mut ResourceLedger,
        result: &TransactionResult<C, O>,
    ) -> Vec<FakeHostEvent> {
        ledger.apply_result(result);
        result
            .resource_plan
            .commands()
            .iter()
            .filter_map(|command| self.status_for_command(ledger, command, result.revision))
            .collect()
    }

    /// Produces a custom host status event and classifies it through the ledger.
    pub fn observe(
        &mut self,
        ledger: &mut ResourceLedger,
        resource_key: ResourceKey,
        scope: ScopeId,
        command_revision: Revision,
    ) -> FakeHostEvent {
        let status = HostStatusEvent {
            resource_key,
            scope,
            command_revision,
            status_revision: self.next_revision(),
            status: HostResourceOutcome::Open,
        };
        let class = ledger.classify_status(status.clone());
        FakeHostEvent { status, class }
    }

    fn status_for_command<C>(
        &mut self,
        ledger: &mut ResourceLedger,
        command: &ResourceCommand<C>,
        revision: Revision,
    ) -> Option<FakeHostEvent> {
        match command {
            ResourceCommand::Open { key, scope, .. }
            | ResourceCommand::Replace { key, scope, .. }
            | ResourceCommand::Refresh { key, scope, .. } => {
                Some(self.observe(ledger, key.clone(), *scope, revision))
            }
            ResourceCommand::Close { .. } => None,
        }
    }

    fn next_revision(&mut self) -> Revision {
        let revision = Revision::new(self.next_status_revision);
        self.next_status_revision += 1;
        revision
    }
}

impl Default for FakeHost {
    fn default() -> Self {
        Self::new()
    }
}
