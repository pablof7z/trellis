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

impl FakeHostEvent {
    /// Consumes the event into the host status application tests feed as input.
    pub fn into_status(self) -> HostStatusEvent {
        self.status
    }
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
    pub fn apply_result<C: Clone, O>(
        &mut self,
        ledger: &mut ResourceLedger<C>,
        result: &TransactionResult<C, O>,
    ) -> Vec<FakeHostEvent> {
        ledger.apply_result(result);
        result
            .resource_plan
            .commands()
            .iter()
            .map(|command| self.status_for_command(ledger, command, result.revision))
            .collect()
    }

    /// Produces a custom successful-open status event.
    pub fn observe<C: Clone>(
        &mut self,
        ledger: &mut ResourceLedger<C>,
        resource_key: ResourceKey,
        scope: ScopeId,
        command_revision: Revision,
    ) -> FakeHostEvent {
        self.observe_outcome(
            ledger,
            resource_key,
            scope,
            command_revision,
            HostResourceOutcome::Open,
        )
    }

    /// Produces a custom host outcome and classifies it through the ledger.
    pub fn observe_outcome<C: Clone>(
        &mut self,
        ledger: &mut ResourceLedger<C>,
        resource_key: ResourceKey,
        scope: ScopeId,
        command_revision: Revision,
        status: HostResourceOutcome,
    ) -> FakeHostEvent {
        let status = HostStatusEvent {
            resource_key,
            scope,
            command_revision,
            status_revision: self.next_revision(),
            status,
        };
        let class = ledger.classify_status(status.clone());
        FakeHostEvent { status, class }
    }

    /// Reports that an open command succeeded.
    pub fn open_succeeded<C: Clone>(
        &mut self,
        ledger: &mut ResourceLedger<C>,
        resource_key: ResourceKey,
        scope: ScopeId,
        command_revision: Revision,
    ) -> FakeHostEvent {
        self.observe(ledger, resource_key, scope, command_revision)
    }

    /// Reports that an earlier open command succeeded after later graph work.
    pub fn open_succeeds_later<C: Clone>(
        &mut self,
        ledger: &mut ResourceLedger<C>,
        resource_key: ResourceKey,
        scope: ScopeId,
        command_revision: Revision,
    ) -> FakeHostEvent {
        self.open_succeeded(ledger, resource_key, scope, command_revision)
    }

    /// Reports that an open command failed.
    pub fn open_failed<C: Clone>(
        &mut self,
        ledger: &mut ResourceLedger<C>,
        resource_key: ResourceKey,
        scope: ScopeId,
        command_revision: Revision,
        reason: impl Into<String>,
    ) -> FakeHostEvent {
        self.observe_outcome(
            ledger,
            resource_key,
            scope,
            command_revision,
            HostResourceOutcome::Failed(reason.into()),
        )
    }

    /// Reports that a close command succeeded.
    pub fn close_succeeded<C: Clone>(
        &mut self,
        ledger: &mut ResourceLedger<C>,
        resource_key: ResourceKey,
        scope: ScopeId,
        command_revision: Revision,
    ) -> FakeHostEvent {
        self.observe_outcome(
            ledger,
            resource_key,
            scope,
            command_revision,
            HostResourceOutcome::Closed,
        )
    }

    /// Reports that a close command failed at the host boundary.
    pub fn close_failed<C: Clone>(
        &mut self,
        ledger: &mut ResourceLedger<C>,
        resource_key: ResourceKey,
        scope: ScopeId,
        command_revision: Revision,
        reason: impl Into<String>,
    ) -> FakeHostEvent {
        self.observe_outcome(
            ledger,
            resource_key,
            scope,
            command_revision,
            HostResourceOutcome::Failed(reason.into()),
        )
    }

    /// Reports that a resource was externally lost outside graph propagation.
    pub fn resource_lost<C: Clone>(
        &mut self,
        ledger: &mut ResourceLedger<C>,
        resource_key: ResourceKey,
        scope: ScopeId,
        command_revision: Revision,
        reason: impl Into<String>,
    ) -> FakeHostEvent {
        self.observe_outcome(
            ledger,
            resource_key,
            scope,
            command_revision,
            HostResourceOutcome::Failed(reason.into()),
        )
    }

    /// Re-delivers a previous host status without assigning a new host revision.
    pub fn duplicate_status<C: Clone>(
        &mut self,
        ledger: &mut ResourceLedger<C>,
        event: &FakeHostEvent,
    ) -> FakeHostEvent {
        let status = event.status.clone();
        let class = ledger.classify_status(status.clone());
        FakeHostEvent { status, class }
    }

    fn status_for_command<C: Clone>(
        &mut self,
        ledger: &mut ResourceLedger<C>,
        command: &ResourceCommand<C>,
        revision: Revision,
    ) -> FakeHostEvent {
        match command {
            ResourceCommand::Open { key, scope, .. }
            | ResourceCommand::Replace { key, scope, .. }
            | ResourceCommand::Refresh { key, scope, .. } => {
                self.open_succeeded(ledger, key.clone(), *scope, revision)
            }
            ResourceCommand::Close { key, scope } => {
                self.close_succeeded(ledger, key.clone(), *scope, revision)
            }
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
