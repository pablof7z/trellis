use std::collections::{BTreeMap, BTreeSet};

use super::status::FleetStatusFrame;

/// Telemetry metric carried by a device topic.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum FleetMetric {
    /// Device temperature.
    Temperature,
    /// Device vibration.
    Vibration,
    /// Device power draw.
    Power,
}

/// Stable telemetry topic identity.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct TelemetryTopic {
    /// Site id that owns the topic.
    pub site: String,
    /// Device id that publishes the topic.
    pub device_id: String,
    /// Metric published on this topic.
    pub metric: FleetMetric,
}

/// Device metadata known to the local dashboard cache.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetDevice {
    /// Stable device id.
    pub id: String,
    /// Customer id.
    pub customer: String,
    /// Site id.
    pub site: String,
    /// Device group.
    pub group: String,
    /// Display label.
    pub label: String,
    /// Telemetry topics published by the device.
    pub topics: BTreeSet<TelemetryTopic>,
}

/// User permission facts owned by the host application.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FleetPermissions {
    /// Customers visible to the user.
    pub customers: BTreeSet<String>,
    /// Sites visible to the user.
    pub sites: BTreeSet<String>,
    /// Devices visible to the user.
    pub devices: BTreeSet<String>,
}

/// Alert rule derived from telemetry topics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetAlertRule {
    /// Stable alert rule id.
    pub id: String,
    /// Display label.
    pub label: String,
    /// Topic evaluated by the rule.
    pub topic: TelemetryTopic,
    /// Alert threshold in integer units.
    pub threshold: i64,
}

/// Host-owned dataset for the FleetPulse showcase.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FleetDataset {
    /// Permissions by user id.
    pub permissions: BTreeMap<String, FleetPermissions>,
    /// Device metadata by device id.
    pub devices: BTreeMap<String, FleetDevice>,
    /// Alert rules by rule id.
    pub alert_rules: BTreeMap<String, FleetAlertRule>,
    /// Latest telemetry values by topic.
    pub telemetry: BTreeMap<TelemetryTopic, i64>,
}

/// Dashboard panel with independent resource ownership.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum FleetPanel {
    /// Main live-card panel.
    Overview,
    /// Alert panel.
    Alerts,
}

/// Parameters for opening or re-filtering a fleet dashboard.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetDashboardParams {
    /// Active user id.
    pub user: String,
    /// Selected customer id.
    pub customer: String,
    /// Selected site id.
    pub site: String,
    /// Visible device groups.
    pub groups: BTreeSet<String>,
    /// Visible panels.
    pub panels: BTreeSet<FleetPanel>,
}

impl FleetDashboardParams {
    /// Returns the default visible panels.
    pub fn all_panels() -> BTreeSet<FleetPanel> {
        [FleetPanel::Overview, FleetPanel::Alerts]
            .into_iter()
            .collect()
    }
}

/// Domain filter change applied by the host.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetFilterChange {
    /// Next selected customer id.
    pub customer: String,
    /// Next selected site id.
    pub site: String,
    /// Next visible device groups.
    pub groups: BTreeSet<String>,
}

/// Domain permission change applied by the host.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FleetPermissionChange {
    /// Removes one visible device from the active user's permission set.
    RevokeDevice {
        /// Device id to revoke.
        device_id: String,
    },
    /// Replaces the entire permission set for the active user.
    ReplacePermissions(FleetPermissions),
}

/// Opaque handle for a FleetPulse dashboard.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct FleetDashboardHandle(pub(super) u64);

/// Host-visible resource target.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum FleetTarget {
    /// Telemetry topic subscription.
    Topic(TelemetryTopic),
    /// Alert stream for one rule.
    AlertStream {
        /// Alert rule id.
        rule_id: String,
    },
}

/// Host effect emitted by the dashboard wrapper.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FleetEffect {
    /// Open a resource target.
    Open(FleetTarget),
    /// Replace a resource target.
    Replace(FleetTarget),
    /// Close a resource target.
    Close(FleetTarget),
}

/// One live telemetry card.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetCard {
    /// Device id.
    pub device_id: String,
    /// Display label.
    pub label: String,
    /// Device group.
    pub group: String,
    /// Latest readings by metric.
    pub readings: BTreeMap<FleetMetric, i64>,
}

/// One alert row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetAlert {
    /// Alert rule id.
    pub rule_id: String,
    /// Device id that triggered the alert.
    pub device_id: String,
    /// Display label.
    pub label: String,
    /// Current reading.
    pub reading: i64,
    /// Rule threshold.
    pub threshold: i64,
}

/// Materialized dashboard state.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FleetSnapshot {
    /// Live telemetry cards.
    pub cards: Vec<FleetCard>,
    /// Active alert rows.
    pub alerts: Vec<FleetAlert>,
    /// Most recent host status classification.
    pub last_status: Option<FleetStatusFrame>,
}

impl FleetSnapshot {
    /// Returns visible device ids in deterministic order.
    pub fn device_ids(&self) -> BTreeSet<String> {
        self.cards
            .iter()
            .map(|card| card.device_id.clone())
            .collect()
    }

    /// Returns true when no cards or alerts are visible.
    pub fn is_empty(&self) -> bool {
        self.cards.is_empty() && self.alerts.is_empty()
    }
}

/// Public output frame emitted by the dashboard wrapper.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FleetFrame {
    /// Complete current dashboard state.
    Baseline(FleetSnapshot),
    /// Replacement dashboard state after ordinary changes.
    Delta(FleetSnapshot),
    /// Complete dashboard state after explicit rebaseline.
    Rebaseline(FleetSnapshot),
    /// Terminal clear frame after dashboard close.
    Cleared,
}

/// Summary returned by FleetPulse API methods.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct FleetUpdate {
    /// Number of effects queued by the method.
    pub emitted_effects: usize,
    /// Number of frames queued by the method.
    pub emitted_frames: usize,
}

/// Private Trellis resource command payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum FleetCommand {
    /// Open a target.
    Open(FleetTarget),
}
