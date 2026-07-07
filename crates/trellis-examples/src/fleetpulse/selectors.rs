use std::collections::{BTreeMap, BTreeSet};

use super::status::FleetStatusFrame;
use super::types::{
    FleetAlert, FleetCard, FleetDashboardParams, FleetDataset, FleetPanel, FleetSnapshot,
    FleetTarget,
};

pub(super) fn visible_device_ids(
    params: &Option<FleetDashboardParams>,
    dataset: &FleetDataset,
) -> BTreeSet<String> {
    let Some(params) = params else {
        return BTreeSet::new();
    };
    let Some(permission) = dataset.permissions.get(&params.user) else {
        return BTreeSet::new();
    };
    if !permission.customers.contains(&params.customer)
        || !permission.sites.contains(&params.site)
        || params.groups.is_empty()
    {
        return BTreeSet::new();
    }

    dataset
        .devices
        .values()
        .filter(|device| device.customer == params.customer)
        .filter(|device| device.site == params.site)
        .filter(|device| params.groups.contains(&device.group))
        .filter(|device| permission.devices.contains(&device.id))
        .map(|device| device.id.clone())
        .collect()
}

pub(super) fn overview_targets_for(
    params: &Option<FleetDashboardParams>,
    dataset: &FleetDataset,
    devices: &BTreeSet<String>,
) -> BTreeSet<FleetTarget> {
    if !panel_visible(params, FleetPanel::Overview) {
        return BTreeSet::new();
    }
    topic_targets(dataset, devices)
}

pub(super) fn alert_targets_for(
    params: &Option<FleetDashboardParams>,
    dataset: &FleetDataset,
    devices: &BTreeSet<String>,
) -> BTreeSet<FleetTarget> {
    if !panel_visible(params, FleetPanel::Alerts) {
        return BTreeSet::new();
    }

    let visible_topics = topic_targets(dataset, devices);
    let mut targets = BTreeSet::new();
    for rule in dataset.alert_rules.values() {
        let topic_target = FleetTarget::Topic(rule.topic.clone());
        if visible_topics.contains(&topic_target) {
            targets.insert(topic_target);
            targets.insert(FleetTarget::AlertStream {
                rule_id: rule.id.clone(),
            });
        }
    }
    targets
}

pub(super) fn fleet_snapshot(
    params: &Option<FleetDashboardParams>,
    dataset: &FleetDataset,
    devices: &BTreeSet<String>,
    last_status: &Option<FleetStatusFrame>,
) -> FleetSnapshot {
    let cards = if panel_visible(params, FleetPanel::Overview) {
        cards_for(dataset, devices)
    } else {
        Vec::new()
    };
    let alerts = if panel_visible(params, FleetPanel::Alerts) {
        alerts_for(dataset, devices)
    } else {
        Vec::new()
    };
    FleetSnapshot {
        cards,
        alerts,
        last_status: last_status.clone(),
    }
}

fn topic_targets(dataset: &FleetDataset, devices: &BTreeSet<String>) -> BTreeSet<FleetTarget> {
    devices
        .iter()
        .filter_map(|device_id| dataset.devices.get(device_id))
        .flat_map(|device| device.topics.iter().cloned().map(FleetTarget::Topic))
        .collect()
}

fn cards_for(dataset: &FleetDataset, devices: &BTreeSet<String>) -> Vec<FleetCard> {
    devices
        .iter()
        .filter_map(|device_id| dataset.devices.get(device_id))
        .map(|device| {
            let readings = device
                .topics
                .iter()
                .filter_map(|topic| {
                    dataset
                        .telemetry
                        .get(topic)
                        .map(|value| (topic.metric.clone(), *value))
                })
                .collect::<BTreeMap<_, _>>();
            FleetCard {
                device_id: device.id.clone(),
                label: device.label.clone(),
                group: device.group.clone(),
                readings,
            }
        })
        .collect()
}

fn alerts_for(dataset: &FleetDataset, devices: &BTreeSet<String>) -> Vec<FleetAlert> {
    dataset
        .alert_rules
        .values()
        .filter(|rule| devices.contains(&rule.topic.device_id))
        .filter_map(|rule| {
            let reading = *dataset.telemetry.get(&rule.topic)?;
            (reading >= rule.threshold).then(|| FleetAlert {
                rule_id: rule.id.clone(),
                device_id: rule.topic.device_id.clone(),
                label: rule.label.clone(),
                reading,
                threshold: rule.threshold,
            })
        })
        .collect()
}

fn panel_visible(params: &Option<FleetDashboardParams>, panel: FleetPanel) -> bool {
    params
        .as_ref()
        .is_some_and(|params| params.panels.contains(&panel))
}
