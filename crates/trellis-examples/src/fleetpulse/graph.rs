use trellis_core::{DependencyList, Graph, InputNode, MaterializedOutput, ResourceKey, ScopeId};

use super::selectors::{
    alert_targets_for, fleet_snapshot, overview_targets_for, visible_device_ids,
};
use super::status::FleetStatusFrame;
use super::types::{
    FleetCommand, FleetDashboardParams, FleetDataset, FleetMetric, FleetSnapshot, FleetTarget,
    TelemetryTopic,
};

pub(super) struct FleetGraph {
    pub(super) graph: Graph<FleetCommand>,
    pub(super) params: InputNode<Option<FleetDashboardParams>>,
    pub(super) dataset: InputNode<FleetDataset>,
    pub(super) host_status: InputNode<Option<FleetStatusFrame>>,
    pub(super) dashboard_scope: ScopeId,
    pub(super) overview_scope: ScopeId,
    pub(super) alerts_scope: ScopeId,
    pub(super) output: MaterializedOutput<FleetSnapshot>,
}

pub(super) fn build_graph(dataset: FleetDataset) -> FleetGraph {
    let mut graph = Graph::<FleetCommand>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let dashboard_scope = tx.create_scope("fleet-dashboard").unwrap();
    let overview_scope = tx
        .create_scope_with_parent("overview-panel", Some(dashboard_scope))
        .unwrap();
    let alerts_scope = tx
        .create_scope_with_parent("alerts-panel", Some(dashboard_scope))
        .unwrap();
    let params = tx
        .input::<Option<FleetDashboardParams>>("fleet-params")
        .unwrap();
    let dataset_input = tx.input::<FleetDataset>("fleet-dataset").unwrap();
    let host_status = tx
        .input::<Option<FleetStatusFrame>>("fleet-host-status")
        .unwrap();
    tx.set_input(params, None).unwrap();
    tx.set_input(dataset_input, dataset).unwrap();
    tx.set_input(host_status, None).unwrap();

    let visible_devices = tx
        .set_collection(
            "visible-devices",
            DependencyList::new([params.id(), dataset_input.id()]).unwrap(),
            move |ctx| {
                Ok(visible_device_ids(
                    ctx.input(params)?,
                    ctx.input(dataset_input)?,
                ))
            },
        )
        .unwrap();

    let overview_targets = tx
        .set_collection(
            "overview-targets",
            DependencyList::new([params.id(), dataset_input.id(), visible_devices.id()]).unwrap(),
            move |ctx| {
                Ok(overview_targets_for(
                    ctx.input(params)?,
                    ctx.input(dataset_input)?,
                    ctx.set_collection(visible_devices)?,
                ))
            },
        )
        .unwrap();

    let alert_targets = tx
        .set_collection(
            "alert-targets",
            DependencyList::new([params.id(), dataset_input.id(), visible_devices.id()]).unwrap(),
            move |ctx| {
                Ok(alert_targets_for(
                    ctx.input(params)?,
                    ctx.input(dataset_input)?,
                    ctx.set_collection(visible_devices)?,
                ))
            },
        )
        .unwrap();

    tx.open_close_planner(overview_targets, overview_scope, target_key, |target| {
        FleetCommand::Open(target.clone())
    })
    .unwrap();
    tx.open_close_planner(alert_targets, alerts_scope, target_key, |target| {
        FleetCommand::Open(target.clone())
    })
    .unwrap();

    let output = tx
        .materialized_output(
            "fleet-dashboard-output",
            dashboard_scope,
            DependencyList::new([
                params.id(),
                dataset_input.id(),
                visible_devices.id(),
                host_status.id(),
            ])
            .unwrap(),
            move |ctx| {
                Ok(fleet_snapshot(
                    ctx.input(params)?,
                    ctx.input(dataset_input)?,
                    ctx.set_collection(visible_devices)?,
                    ctx.input(host_status)?,
                ))
            },
        )
        .unwrap();
    tx.commit().unwrap();
    drop(tx);

    FleetGraph {
        graph,
        params,
        dataset: dataset_input,
        host_status,
        dashboard_scope,
        overview_scope,
        alerts_scope,
        output,
    }
}

pub(super) fn target_key(target: &FleetTarget) -> ResourceKey {
    match target {
        FleetTarget::Topic(topic) => ResourceKey::from_segments([
            "fleet",
            "topic",
            &topic.site,
            &topic.device_id,
            metric_segment(&topic.metric),
        ]),
        FleetTarget::AlertStream { rule_id } => {
            ResourceKey::from_segments(["fleet", "alert", rule_id])
        }
    }
}

pub(super) fn target_from_key(key: &ResourceKey) -> Option<FleetTarget> {
    let segments = key.segments().collect::<Vec<_>>();
    match segments.as_slice() {
        ["fleet", "topic", site, device_id, metric] => Some(FleetTarget::Topic(TelemetryTopic {
            site: (*site).to_owned(),
            device_id: (*device_id).to_owned(),
            metric: metric_from_segment(metric)?,
        })),
        ["fleet", "alert", rule_id] => Some(FleetTarget::AlertStream {
            rule_id: (*rule_id).to_owned(),
        }),
        _ => None,
    }
}

fn metric_segment(metric: &FleetMetric) -> &'static str {
    match metric {
        FleetMetric::Temperature => "temperature",
        FleetMetric::Vibration => "vibration",
        FleetMetric::Power => "power",
    }
}

fn metric_from_segment(segment: &str) -> Option<FleetMetric> {
    match segment {
        "temperature" => Some(FleetMetric::Temperature),
        "vibration" => Some(FleetMetric::Vibration),
        "power" => Some(FleetMetric::Power),
        _ => None,
    }
}
