use std::collections::BTreeSet;

use super::types::{
    FleetAlertRule, FleetDashboardParams, FleetDataset, FleetDevice, FleetMetric, FleetPermissions,
    TelemetryTopic,
};

impl FleetDataset {
    /// Returns deterministic sample data for tests and headless scripts.
    pub fn sample() -> Self {
        let pump_1_temp = topic("plant-7", "pump-1", FleetMetric::Temperature);
        let pump_1_vibration = topic("plant-7", "pump-1", FleetMetric::Vibration);
        let pump_2_temp = topic("plant-7", "pump-2", FleetMetric::Temperature);
        let pump_2_vibration = topic("plant-7", "pump-2", FleetMetric::Vibration);
        let boiler_temp = topic("plant-7", "boiler-9", FleetMetric::Temperature);
        let boiler_power = topic("plant-7", "boiler-9", FleetMetric::Power);
        let compressor_vibration = topic("plant-3", "compressor-4", FleetMetric::Vibration);

        let devices = [
            FleetDevice {
                id: "pump-1".to_owned(),
                customer: "acme".to_owned(),
                site: "plant-7".to_owned(),
                group: "pumps".to_owned(),
                label: "North Pump".to_owned(),
                topics: BTreeSet::from([pump_1_temp.clone(), pump_1_vibration.clone()]),
            },
            FleetDevice {
                id: "pump-2".to_owned(),
                customer: "acme".to_owned(),
                site: "plant-7".to_owned(),
                group: "pumps".to_owned(),
                label: "South Pump".to_owned(),
                topics: BTreeSet::from([pump_2_temp.clone(), pump_2_vibration.clone()]),
            },
            FleetDevice {
                id: "boiler-9".to_owned(),
                customer: "acme".to_owned(),
                site: "plant-7".to_owned(),
                group: "boilers".to_owned(),
                label: "Main Boiler".to_owned(),
                topics: BTreeSet::from([boiler_temp.clone(), boiler_power.clone()]),
            },
            FleetDevice {
                id: "compressor-4".to_owned(),
                customer: "globex".to_owned(),
                site: "plant-3".to_owned(),
                group: "compressors".to_owned(),
                label: "East Compressor".to_owned(),
                topics: BTreeSet::from([compressor_vibration.clone()]),
            },
        ]
        .into_iter()
        .map(|device| (device.id.clone(), device))
        .collect();

        let permissions = [(
            "alex".to_owned(),
            FleetPermissions {
                customers: BTreeSet::from(["acme".to_owned()]),
                sites: BTreeSet::from(["plant-7".to_owned()]),
                devices: BTreeSet::from([
                    "pump-1".to_owned(),
                    "pump-2".to_owned(),
                    "boiler-9".to_owned(),
                ]),
            },
        )]
        .into_iter()
        .collect();

        let alert_rules = [
            FleetAlertRule {
                id: "pump-2-overheat".to_owned(),
                label: "South pump overheat".to_owned(),
                topic: pump_2_temp.clone(),
                threshold: 90,
            },
            FleetAlertRule {
                id: "pump-2-vibration".to_owned(),
                label: "South pump vibration".to_owned(),
                topic: pump_2_vibration.clone(),
                threshold: 35,
            },
            FleetAlertRule {
                id: "boiler-power".to_owned(),
                label: "Boiler power draw".to_owned(),
                topic: boiler_power.clone(),
                threshold: 120,
            },
        ]
        .into_iter()
        .map(|rule| (rule.id.clone(), rule))
        .collect();

        let telemetry = [
            (pump_1_temp, 82),
            (pump_1_vibration, 12),
            (pump_2_temp, 95),
            (pump_2_vibration, 41),
            (boiler_temp, 73),
            (boiler_power, 118),
            (compressor_vibration, 18),
        ]
        .into_iter()
        .collect();

        Self {
            permissions,
            devices,
            alert_rules,
            telemetry,
        }
    }
}

pub(super) fn topic(site: &str, device_id: &str, metric: FleetMetric) -> TelemetryTopic {
    TelemetryTopic {
        site: site.to_owned(),
        device_id: device_id.to_owned(),
        metric,
    }
}

pub(super) fn params_for_groups(
    groups: impl IntoIterator<Item = &'static str>,
) -> FleetDashboardParams {
    FleetDashboardParams {
        user: "alex".to_owned(),
        customer: "acme".to_owned(),
        site: "plant-7".to_owned(),
        groups: groups.into_iter().map(str::to_owned).collect(),
        panels: FleetDashboardParams::all_panels(),
    }
}

#[cfg(test)]
pub(super) fn params_for_panels(
    groups: impl IntoIterator<Item = &'static str>,
    panels: impl IntoIterator<Item = super::types::FleetPanel>,
) -> FleetDashboardParams {
    FleetDashboardParams {
        panels: panels.into_iter().collect(),
        ..params_for_groups(groups)
    }
}
