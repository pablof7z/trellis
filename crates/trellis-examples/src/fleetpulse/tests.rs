use super::engine::default_params;
use super::sample::{params_for_groups, params_for_panels, topic};
use super::status_runtime::host_status_for;
use super::*;

#[test]
fn filter_shrink_unsubscribes_removed_topics_and_rebaselines() {
    let mut app = FleetPulseApp::default();
    let handle = app.open_fleet_dashboard(default_params());
    app.drain_effects();
    app.drain_output(handle);
    app.drain_diagnostic_traces();

    let update = app.apply_filter_change(
        handle,
        FleetFilterChange {
            customer: "acme".to_owned(),
            site: "plant-7".to_owned(),
            groups: ["boilers"].into_iter().map(str::to_owned).collect(),
        },
    );
    assert!(update.emitted_effects > 0);
    assert_eq!(update.emitted_frames, 1);

    let effects = app.drain_effects();
    assert!(effects.contains(&FleetEffect::Close(pump_2_temp())));
    assert!(
        effects.contains(&FleetEffect::Close(FleetTarget::AlertStream {
            rule_id: "pump-2-overheat".to_owned()
        }))
    );
    assert!(
        effects.contains(&FleetEffect::Open(FleetTarget::Topic(topic(
            "plant-7",
            "boiler-9",
            FleetMetric::Power
        ))))
    );

    let frames = app.drain_output(handle);
    assert!(matches!(
        &frames[0],
        FleetFrame::Rebaseline(snapshot)
            if snapshot.device_ids() == ["boiler-9".to_owned()].into_iter().collect()
                && snapshot.alerts.is_empty()
    ));
    assert_oracle_trace(&mut app);
}

#[test]
fn permission_revoke_closes_unauthorized_topics_and_clears_cards() {
    let mut app = FleetPulseApp::default();
    let handle = app.open_fleet_dashboard(default_params());
    app.drain_effects();
    app.drain_output(handle);
    app.drain_diagnostic_traces();

    let update = app.apply_permission_change(
        handle,
        FleetPermissionChange::RevokeDevice {
            device_id: "pump-2".to_owned(),
        },
    );
    assert!(update.emitted_effects > 0);

    let effects = app.drain_effects();
    assert!(effects.contains(&FleetEffect::Close(pump_2_temp())));
    assert!(
        effects.contains(&FleetEffect::Close(FleetTarget::AlertStream {
            rule_id: "pump-2-vibration".to_owned()
        }))
    );

    let frames = app.drain_output(handle);
    assert!(matches!(
        &frames[0],
        FleetFrame::Delta(snapshot)
            if snapshot.device_ids() == ["pump-1".to_owned()].into_iter().collect()
                && snapshot.alerts.is_empty()
    ));
    assert_oracle_trace(&mut app);
}

#[test]
fn empty_device_filter_opens_no_windows_or_wildcards() {
    let mut app = FleetPulseApp::default();
    let handle = app.open_fleet_dashboard(params_for_groups(["missing"]));
    assert!(app.drain_effects().is_empty());
    let frames = app.drain_output(handle);
    assert!(
        frames
            .first()
            .is_none_or(|frame| snapshot(frame).is_empty())
    );
    assert_oracle_trace(&mut app);
}

#[test]
fn shared_topic_closes_after_last_panel_owner() {
    let mut app = FleetPulseApp::default();
    let handle = app.open_fleet_dashboard(default_params());
    app.drain_effects();
    app.drain_output(handle);
    app.drain_diagnostic_traces();

    let update = app.close_panel_for_test(handle, FleetPanel::Overview);
    assert!(update.emitted_effects > 0);
    let effects = app.drain_effects();
    assert!(!effects.contains(&FleetEffect::Close(pump_2_temp())));
    app.drain_diagnostic_traces();

    let update = app.close(handle);
    assert!(update.emitted_effects > 0);
    assert!(
        app.drain_effects()
            .contains(&FleetEffect::Close(pump_2_temp()))
    );
}

#[test]
fn late_status_for_closed_topic_is_classified_and_ignored() {
    let mut app = FleetPulseApp::default();
    let handle = app.open_fleet_dashboard(default_params());
    let open_revision = app.command_revision_for(&pump_2_temp()).unwrap();
    app.drain_effects();
    app.drain_output(handle);
    app.drain_diagnostic_traces();

    app.apply_permission_change(
        handle,
        FleetPermissionChange::RevokeDevice {
            device_id: "pump-2".to_owned(),
        },
    );
    app.drain_effects();
    app.drain_output(handle);
    app.drain_diagnostic_traces();

    let update = app.apply_host_status(
        handle,
        host_status_for(
            pump_2_temp(),
            FleetPanel::Overview,
            open_revision,
            100,
            FleetHostOutcome::Open,
        ),
    );
    assert_eq!(update.emitted_effects, 0);
    let frames = app.drain_output(handle);
    assert!(matches!(
        &snapshot(&frames[0]).last_status,
        Some(status)
            if status.class == FleetStatusClass::Late
                && status.target == pump_2_temp()
    ));
    assert!(app.drain_effects().is_empty());
    assert_oracle_trace(&mut app);
}

#[test]
fn close_tears_down_scopes_resources_and_output() {
    let mut app = FleetPulseApp::default();
    let handle = app.open_fleet_dashboard(params_for_panels(
        ["pumps"],
        [FleetPanel::Overview, FleetPanel::Alerts],
    ));
    app.drain_effects();
    app.drain_output(handle);
    app.drain_diagnostic_traces();

    let update = app.close(handle);
    assert!(update.emitted_effects > 0);
    assert_eq!(update.emitted_frames, 1);
    assert_eq!(app.drain_output(handle), vec![FleetFrame::Cleared]);
    let traces = app.drain_diagnostic_traces();
    assert!(
        traces[0]
            .scope_events
            .iter()
            .any(|event| event.kind == trellis_core::ScopeLifecycleKind::Closed)
    );
}

fn assert_oracle_trace(app: &mut FleetPulseApp) {
    let traces = app.drain_diagnostic_traces();
    assert!(traces.iter().any(|trace| {
        trace.invariant_results.iter().any(|invariant| {
            invariant.name == "incremental_equals_full_recompute" && invariant.passed
        })
    }));
}

fn snapshot(frame: &FleetFrame) -> &FleetSnapshot {
    match frame {
        FleetFrame::Baseline(snapshot)
        | FleetFrame::Delta(snapshot)
        | FleetFrame::Rebaseline(snapshot) => snapshot,
        FleetFrame::Cleared => panic!("expected a fleet snapshot frame"),
    }
}

fn pump_2_temp() -> FleetTarget {
    FleetTarget::Topic(topic("plant-7", "pump-2", FleetMetric::Temperature))
}
