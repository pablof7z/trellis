use std::collections::{BTreeMap, BTreeSet};

use super::types::{
    ControlCondition, ControlResource, ControlResourceStatus, ControlResourceView, ControlSnapshot,
    DesiredAppConfig,
};

pub(super) fn desired_resources(config: &Option<DesiredAppConfig>) -> BTreeSet<ControlResource> {
    let Some(config) = config else {
        return BTreeSet::new();
    };
    let mut resources = BTreeSet::new();
    for ordinal in 0..config.replicas {
        resources.insert(ControlResource::Worker {
            app_id: config.app_id.clone(),
            ordinal,
            image: config.image.clone(),
            version: config.version.clone(),
        });
    }
    resources.insert(ControlResource::Port {
        app_id: config.app_id.clone(),
        port: config.port,
    });
    for name in &config.volumes {
        resources.insert(ControlResource::Volume {
            app_id: config.app_id.clone(),
            name: name.clone(),
        });
    }
    for name in &config.secrets {
        resources.insert(ControlResource::Secret {
            app_id: config.app_id.clone(),
            name: name.clone(),
        });
    }
    resources
}

pub(super) fn retry_resources(
    config: &Option<DesiredAppConfig>,
    desired: &BTreeSet<ControlResource>,
    statuses: &BTreeMap<ControlResource, ControlResourceStatus>,
) -> BTreeSet<ControlResource> {
    let Some(config) = config else {
        return BTreeSet::new();
    };
    desired
        .iter()
        .filter(|resource| {
            matches!(
                statuses.get(*resource),
                Some(ControlResourceStatus::Failed(_))
            )
        })
        .map(|resource| ControlResource::RetryJob {
            app_id: config.app_id.clone(),
            target: resource_id(resource),
        })
        .collect()
}

pub(super) fn managed_resources(
    desired: &BTreeSet<ControlResource>,
    retries: &BTreeSet<ControlResource>,
) -> BTreeSet<ControlResource> {
    desired.union(retries).cloned().collect()
}

pub(super) fn control_snapshot(
    config: &Option<DesiredAppConfig>,
    desired: &BTreeSet<ControlResource>,
    retries: &BTreeSet<ControlResource>,
    statuses: &BTreeMap<ControlResource, ControlResourceStatus>,
) -> ControlSnapshot {
    let resources = desired
        .iter()
        .map(|resource| resource_view(resource, true, statuses.get(resource).cloned()))
        .chain(
            retries
                .iter()
                .map(|resource| resource_view(resource, false, None)),
        )
        .collect();
    ControlSnapshot {
        app_id: config.as_ref().map(|config| config.app_id.clone()),
        desired_resources: desired.len(),
        retry_jobs: retries.len(),
        resources,
        conditions: conditions(desired, retries, statuses),
    }
}

pub(super) fn resource_id(resource: &ControlResource) -> String {
    match resource {
        ControlResource::Worker {
            app_id,
            ordinal,
            image,
            version,
        } => format!("{app_id}/worker/{ordinal}/{image}/{version}"),
        ControlResource::Port { app_id, port } => format!("{app_id}/port/{port}"),
        ControlResource::Volume { app_id, name } => format!("{app_id}/volume/{name}"),
        ControlResource::Secret { app_id, name } => format!("{app_id}/secret/{name}"),
        ControlResource::RetryJob { app_id, target } => format!("{app_id}/retry/{target}"),
    }
}

fn resource_view(
    resource: &ControlResource,
    desired: bool,
    status: Option<ControlResourceStatus>,
) -> ControlResourceView {
    ControlResourceView {
        resource_id: resource_id(resource),
        kind: resource_kind(resource),
        desired,
        status,
    }
}

fn conditions(
    desired: &BTreeSet<ControlResource>,
    retries: &BTreeSet<ControlResource>,
    statuses: &BTreeMap<ControlResource, ControlResourceStatus>,
) -> Vec<ControlCondition> {
    if desired.is_empty() {
        return Vec::new();
    }
    let failures = desired
        .iter()
        .filter(|resource| {
            matches!(
                statuses.get(*resource),
                Some(ControlResourceStatus::Failed(_))
            )
        })
        .count();
    let ready = desired
        .iter()
        .filter(|resource| matches!(statuses.get(*resource), Some(ControlResourceStatus::Ready)))
        .count();
    let mut conditions = Vec::new();
    conditions.push(ControlCondition {
        kind: "Available".to_owned(),
        status: (failures == 0).to_string(),
        message: if failures == 0 {
            format!("{ready}/{} desired resources report ready", desired.len())
        } else {
            format!("{failures} desired resources are failed")
        },
    });
    if failures > 0 {
        conditions.push(ControlCondition {
            kind: "Degraded".to_owned(),
            status: "true".to_owned(),
            message: format!("{} retry jobs requested", retries.len()),
        });
    }
    conditions
}

fn resource_kind(resource: &ControlResource) -> String {
    match resource {
        ControlResource::Worker { .. } => "worker",
        ControlResource::Port { .. } => "port",
        ControlResource::Volume { .. } => "volume",
        ControlResource::Secret { .. } => "secret",
        ControlResource::RetryJob { .. } => "retry",
    }
    .to_owned()
}
