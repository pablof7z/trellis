use std::collections::{BTreeMap, BTreeSet};

use crate::showcase_trace::ShowcaseHostStatus;
use trellis_core::{
    HostResourceCommandState, HostResourceOutcome, HostResourceStatus, ResourceKey, Revision,
    ScopeId, classify_host_resource_status,
};

use super::status::{FleetHostOutcome, FleetHostStatus, FleetStatusFrame};
use super::types::{FleetPanel, FleetTarget};

#[derive(Copy, Clone)]
struct CommandState {
    scope: ScopeId,
    command_revision: Revision,
    resource_is_live: bool,
}

pub(super) struct FleetStatusRuntime {
    command_states: BTreeMap<ResourceKey, CommandState>,
    accepted_statuses: BTreeSet<(ResourceKey, ScopeId, Revision, Revision)>,
}

impl FleetStatusRuntime {
    pub(super) fn new() -> Self {
        Self {
            command_states: BTreeMap::new(),
            accepted_statuses: BTreeSet::new(),
        }
    }

    pub(super) fn record_live(&mut self, key: ResourceKey, scope: ScopeId, revision: Revision) {
        self.command_states.insert(
            key,
            CommandState {
                scope,
                command_revision: revision,
                resource_is_live: true,
            },
        );
    }

    pub(super) fn record_closed(&mut self, key: ResourceKey, scope: ScopeId, revision: Revision) {
        self.command_states.insert(
            key,
            CommandState {
                scope,
                command_revision: revision,
                resource_is_live: false,
            },
        );
    }

    pub(super) fn command_revision_for(&self, key: &ResourceKey) -> Option<u64> {
        self.command_states
            .get(key)
            .map(|state| state.command_revision.get())
    }

    pub(super) fn classify_status(
        &mut self,
        status: FleetHostStatus,
        key: ResourceKey,
        scope: ScopeId,
        scope_owns_resource: bool,
    ) -> FleetStatusFrame {
        let command_revision = Revision::new(status.command_revision);
        let status_revision = Revision::new(status.status_revision);
        let status_input = HostResourceStatus::new(
            key.clone(),
            scope,
            command_revision,
            status_revision,
            host_outcome(&status.outcome),
        );
        let duplicate_identity = (key.clone(), scope, command_revision, status_revision);
        let duplicate = self.accepted_statuses.contains(&duplicate_identity);
        let class = classify_host_resource_status(
            &status_input,
            self.status_state(&key, scope_owns_resource),
            duplicate,
        );
        if class == trellis_core::HostStatusClass::Current {
            self.accepted_statuses.insert(duplicate_identity);
        }

        FleetStatusFrame {
            target: status.target,
            panel: status.panel,
            class: class.into(),
            outcome: status.outcome,
            command_revision: status.command_revision,
            status_revision: status.status_revision,
        }
    }

    fn status_state(
        &self,
        key: &ResourceKey,
        scope_owns_resource: bool,
    ) -> Option<HostResourceCommandState> {
        let state = self.command_states.get(key)?;
        Some(HostResourceCommandState {
            scope: state.scope,
            command_revision: state.command_revision,
            resource_is_live: state.resource_is_live,
            scope_owns_resource,
        })
    }
}

pub(super) fn showcase_status(frame: &FleetStatusFrame) -> ShowcaseHostStatus {
    ShowcaseHostStatus {
        target: showcase_target(&frame.target),
        status: format!("{:?}:{:?}", frame.class, frame.outcome),
        command_revision: Some(frame.command_revision),
    }
}

pub(super) fn host_status_for(
    target: FleetTarget,
    panel: FleetPanel,
    command_revision: u64,
    status_revision: u64,
    outcome: FleetHostOutcome,
) -> FleetHostStatus {
    FleetHostStatus {
        target,
        panel,
        command_revision,
        status_revision,
        outcome,
    }
}

fn host_outcome(outcome: &FleetHostOutcome) -> HostResourceOutcome {
    match outcome {
        FleetHostOutcome::Open => HostResourceOutcome::Open,
        FleetHostOutcome::Closed => HostResourceOutcome::Closed,
        FleetHostOutcome::Failed(reason) => HostResourceOutcome::Failed(reason.clone()),
        FleetHostOutcome::Unsupported(reason) => HostResourceOutcome::Unsupported(reason.clone()),
    }
}

fn showcase_target(target: &FleetTarget) -> String {
    match target {
        FleetTarget::Topic(topic) => format!(
            "topic/{}/{}/{:?}",
            topic.site, topic.device_id, topic.metric
        ),
        FleetTarget::AlertStream { rule_id } => {
            format!("alert/{rule_id}")
        }
    }
}
