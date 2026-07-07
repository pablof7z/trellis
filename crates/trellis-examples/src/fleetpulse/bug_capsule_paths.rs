use crate::seeded_bugs::{self, SeededBugFailure, SeededBugRun};

use super::engine::default_params;
use super::sample::{params_for_groups, topic};
use super::status_runtime::host_status_for;
use super::{
    FleetEffect, FleetFilterChange, FleetFrame, FleetHostOutcome, FleetMetric, FleetPanel,
    FleetPermissionChange, FleetPulseApp, FleetSnapshot, FleetStatusClass, FleetTarget,
};

pub(super) fn run_filter_shrink(invariant: &'static str) -> (SeededBugRun, SeededBugRun) {
    let (mut app, handle) = open_default_dashboard();
    app.apply_filter_change(
        handle,
        FleetFilterChange {
            customer: "acme".to_owned(),
            site: "plant-7".to_owned(),
            groups: ["boilers"].into_iter().map(str::to_owned).collect(),
        },
    );
    let effects = app.drain_effects();
    let traces = app.drain_diagnostic_traces();
    let success_failures = topic_close_failures(invariant, &effects);
    let bug_effects = effects
        .iter()
        .filter(|effect| !matches!(effect, FleetEffect::Close(target) if *target == pump_2_temp()))
        .cloned()
        .collect::<Vec<_>>();
    let bug_failures = topic_close_failures(invariant, &bug_effects);

    (
        seeded_bugs::run(
            "trellis",
            "fleet-filter-shrink",
            traces.len(),
            success_failures,
        ),
        seeded_bugs::run("naive", "fleet-filter-shrink", traces.len(), bug_failures),
    )
}

pub(super) fn run_late_status(invariant: &'static str) -> (SeededBugRun, SeededBugRun) {
    let (mut app, handle) = open_default_dashboard();
    let target = pump_2_temp();
    let open_revision = app
        .command_revision_for(&target)
        .expect("pump-2 topic opens during setup");
    app.apply_permission_change(
        handle,
        FleetPermissionChange::RevokeDevice {
            device_id: "pump-2".to_owned(),
        },
    );
    app.drain_effects();
    app.drain_output(handle);
    app.drain_diagnostic_traces();

    app.apply_host_status(
        handle,
        host_status_for(
            target.clone(),
            FleetPanel::Overview,
            open_revision,
            100,
            FleetHostOutcome::Open,
        ),
    );
    let effects = app.drain_effects();
    let frames = app.drain_output(handle);
    let traces = app.drain_diagnostic_traces();
    let success_failures = late_status_failures(invariant, &effects, frame_snapshot(&frames));
    let bug_failures = vec![failure(
        "fleet-late-status-audit-only",
        "late status remained audit-only",
        "HostStatusAudit",
        invariant,
        "Naive callback accepted a late pump-2 topic status and reopened output demand.",
    )];

    (
        seeded_bugs::run(
            "trellis",
            "fleet-late-status",
            traces.len(),
            success_failures,
        ),
        seeded_bugs::run("naive", "fleet-late-status", traces.len(), bug_failures),
    )
}

pub(super) fn run_shared_topic(invariant: &'static str) -> (SeededBugRun, SeededBugRun) {
    let (mut app, handle) = open_default_dashboard();
    app.close_panel(handle, FleetPanel::Overview);
    let effects = app.drain_effects();
    let traces = app.drain_diagnostic_traces();
    let success_failures = shared_topic_failures(invariant, &effects);
    let bug_failures = vec![failure(
        "fleet-shared-topic-keeps-last-owner",
        "shared topic retained alert-panel owner",
        "ResourceLedger",
        invariant,
        "Naive owner table closed pump-2 temperature while the alerts panel still owned it.",
    )];

    (
        seeded_bugs::run(
            "trellis",
            "fleet-shared-topic",
            traces.len(),
            success_failures,
        ),
        seeded_bugs::run("naive", "fleet-shared-topic", traces.len(), bug_failures),
    )
}

pub(super) fn run_empty_device_set(invariant: &'static str) -> (SeededBugRun, SeededBugRun) {
    let mut app = FleetPulseApp::default();
    let handle = app.open_fleet_dashboard(params_for_groups(["missing"]));
    let effects = app.drain_effects();
    let frames = app.drain_output(handle);
    let traces = app.drain_diagnostic_traces();
    let success_failures = empty_device_failures(invariant, &effects, frame_snapshot(&frames));
    let bug_failures = vec![failure(
        "fleet-empty-device-set-opens-no-wildcard",
        "empty device set opened wildcard subscription",
        "ResourceLedger",
        invariant,
        "Naive fallback opened fleet/topic/* even though visible devices was empty.",
    )];

    (
        seeded_bugs::run(
            "trellis",
            "fleet-empty-device",
            traces.len(),
            success_failures,
        ),
        seeded_bugs::run("naive", "fleet-empty-device", traces.len(), bug_failures),
    )
}

fn open_default_dashboard() -> (FleetPulseApp, super::FleetDashboardHandle) {
    let mut app = FleetPulseApp::default();
    let handle = app.open_fleet_dashboard(default_params());
    app.drain_effects();
    app.drain_output(handle);
    app.drain_diagnostic_traces();
    (app, handle)
}

fn topic_close_failures(invariant: &'static str, effects: &[FleetEffect]) -> Vec<SeededBugFailure> {
    if effects.contains(&FleetEffect::Close(pump_2_temp())) {
        Vec::new()
    } else {
        vec![failure(
            "fleet-filter-shrink-unsubscribes-topic",
            "filter shrink unsubscribed removed topic",
            "ResourceLedger",
            invariant,
            "pump-2 temperature subscription remained live after filtering to boilers.",
        )]
    }
}

fn late_status_failures(
    invariant: &'static str,
    effects: &[FleetEffect],
    snapshot: Option<&FleetSnapshot>,
) -> Vec<SeededBugFailure> {
    let late = snapshot
        .and_then(|snapshot| snapshot.last_status.as_ref())
        .is_some_and(|status| {
            status.class == FleetStatusClass::Late && status.target == pump_2_temp()
        });
    if effects.is_empty() && late {
        Vec::new()
    } else {
        vec![failure(
            "fleet-late-status-audit-only",
            "late status remained audit-only",
            "HostStatusAudit",
            invariant,
            "late pump-2 status produced effects or was not classified as Late.",
        )]
    }
}

fn shared_topic_failures(
    invariant: &'static str,
    effects: &[FleetEffect],
) -> Vec<SeededBugFailure> {
    if effects.contains(&FleetEffect::Close(pump_2_temp())) {
        vec![failure(
            "fleet-shared-topic-keeps-last-owner",
            "shared topic retained alert-panel owner",
            "ResourceLedger",
            invariant,
            "pump-2 temperature closed while alert panel still owned the shared topic.",
        )]
    } else {
        Vec::new()
    }
}

fn empty_device_failures(
    invariant: &'static str,
    effects: &[FleetEffect],
    snapshot: Option<&FleetSnapshot>,
) -> Vec<SeededBugFailure> {
    if effects.is_empty() && snapshot.is_none_or(FleetSnapshot::is_empty) {
        Vec::new()
    } else {
        vec![failure(
            "fleet-empty-device-set-opens-no-wildcard",
            "empty device set opened wildcard subscription",
            "ResourceLedger",
            invariant,
            "empty device set emitted fleet effects or visible cards.",
        )]
    }
}

fn frame_snapshot(frames: &[FleetFrame]) -> Option<&FleetSnapshot> {
    frames.first().and_then(|frame| match frame {
        FleetFrame::Baseline(snapshot)
        | FleetFrame::Delta(snapshot)
        | FleetFrame::Rebaseline(snapshot) => Some(snapshot),
        FleetFrame::Cleared => None,
    })
}

fn pump_2_temp() -> FleetTarget {
    FleetTarget::Topic(topic("plant-7", "pump-2", FleetMetric::Temperature))
}

fn failure(
    id: &str,
    label: &str,
    source: &str,
    invariant: &'static str,
    details: &str,
) -> SeededBugFailure {
    seeded_bugs::failure(id, label, source, invariant, details)
}
