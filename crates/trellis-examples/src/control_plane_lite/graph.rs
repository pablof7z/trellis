use std::collections::BTreeMap;

use trellis_core::{DependencyList, Graph, InputNode, MaterializedOutput, ResourceKey, ScopeId};

use super::selectors::{control_snapshot, desired_resources, managed_resources, retry_resources};
use super::types::{
    ControlCommand, ControlResource, ControlResourceStatus, ControlSnapshot, DesiredAppConfig,
};

pub(super) struct ControlGraph {
    pub(super) graph: Graph<ControlCommand>,
    pub(super) config: InputNode<Option<DesiredAppConfig>>,
    pub(super) statuses: InputNode<BTreeMap<ControlResource, ControlResourceStatus>>,
    pub(super) controller_scope: ScopeId,
    pub(super) output: MaterializedOutput<ControlSnapshot>,
}

pub(super) fn build_graph() -> ControlGraph {
    let mut graph = Graph::<ControlCommand>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let controller_scope = tx.create_scope("control-plane-controller").unwrap();
    let config = tx
        .input::<Option<DesiredAppConfig>>("desired-app-config")
        .unwrap();
    let statuses = tx
        .input::<BTreeMap<ControlResource, ControlResourceStatus>>("actual-resource-statuses")
        .unwrap();
    tx.set_input(config, None).unwrap();
    tx.set_input(statuses, BTreeMap::new()).unwrap();

    let desired = tx
        .set_collection(
            "control-desired-resources",
            DependencyList::new([config.id()]).unwrap(),
            move |ctx| Ok(desired_resources(ctx.input(config)?)),
        )
        .unwrap();

    let retries = tx
        .set_collection(
            "control-retry-resources",
            DependencyList::new([config.id(), desired.id(), statuses.id()]).unwrap(),
            move |ctx| {
                Ok(retry_resources(
                    ctx.input(config)?,
                    ctx.set_collection(desired)?,
                    ctx.input(statuses)?,
                ))
            },
        )
        .unwrap();

    let managed = tx
        .set_collection(
            "control-managed-resources",
            DependencyList::new([desired.id(), retries.id()]).unwrap(),
            move |ctx| {
                Ok(managed_resources(
                    ctx.set_collection(desired)?,
                    ctx.set_collection(retries)?,
                ))
            },
        )
        .unwrap();

    tx.open_close_planner(managed, controller_scope, resource_key, |resource| {
        ControlCommand::Open(resource.clone())
    })
    .unwrap();

    let output = tx
        .materialized_output(
            "control-status-output",
            controller_scope,
            DependencyList::new([config.id(), desired.id(), retries.id(), statuses.id()]).unwrap(),
            move |ctx| {
                Ok(control_snapshot(
                    ctx.input(config)?,
                    ctx.set_collection(desired)?,
                    ctx.set_collection(retries)?,
                    ctx.input(statuses)?,
                ))
            },
        )
        .unwrap();

    tx.commit().unwrap();
    drop(tx);

    ControlGraph {
        graph,
        config,
        statuses,
        controller_scope,
        output,
    }
}

pub(super) fn resource_key(resource: &ControlResource) -> ResourceKey {
    match resource {
        ControlResource::Worker {
            app_id,
            ordinal,
            image,
            version,
        } => {
            let ordinal = ordinal.to_string();
            ResourceKey::from_segments([
                "control",
                "worker",
                app_id.as_str(),
                ordinal.as_str(),
                image.as_str(),
                version.as_str(),
            ])
        }
        ControlResource::Port { app_id, port } => {
            let port = port.to_string();
            ResourceKey::from_segments(["control", "port", app_id.as_str(), port.as_str()])
        }
        ControlResource::Volume { app_id, name } => {
            ResourceKey::from_segments(["control", "volume", app_id.as_str(), name.as_str()])
        }
        ControlResource::Secret { app_id, name } => {
            ResourceKey::from_segments(["control", "secret", app_id.as_str(), name.as_str()])
        }
        ControlResource::RetryJob { app_id, target } => {
            ResourceKey::from_segments(["control", "retry", app_id.as_str(), target.as_str()])
        }
    }
}

pub(super) fn resource_from_key(key: &ResourceKey) -> Option<ControlResource> {
    let segments = key.segments().collect::<Vec<_>>();
    match segments.as_slice() {
        ["control", "worker", app_id, ordinal, image, version] => Some(ControlResource::Worker {
            app_id: (*app_id).to_owned(),
            ordinal: ordinal.parse().ok()?,
            image: (*image).to_owned(),
            version: (*version).to_owned(),
        }),
        ["control", "port", app_id, port] => Some(ControlResource::Port {
            app_id: (*app_id).to_owned(),
            port: port.parse().ok()?,
        }),
        ["control", "volume", app_id, name] => Some(ControlResource::Volume {
            app_id: (*app_id).to_owned(),
            name: (*name).to_owned(),
        }),
        ["control", "secret", app_id, name] => Some(ControlResource::Secret {
            app_id: (*app_id).to_owned(),
            name: (*name).to_owned(),
        }),
        ["control", "retry", app_id, target] => Some(ControlResource::RetryJob {
            app_id: (*app_id).to_owned(),
            target: (*target).to_owned(),
        }),
        _ => None,
    }
}
