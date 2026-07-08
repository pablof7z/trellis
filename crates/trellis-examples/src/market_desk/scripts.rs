use crate::showcase_trace::{ShowcaseStep, ShowcaseTrace, build_showcase_trace};

use super::MarketDeskApp;
use super::sample::{churn_symbols, opening_workspace, rotated_watchlist, sample_dataset};
use super::types::MarketDeskEvent;

/// Runs the headless `market-lifecycle` showcase script.
pub fn market_lifecycle_showcase_trace() -> ShowcaseTrace {
    build_showcase_trace(
        "market-desk",
        "market-lifecycle",
        &[
            "cargo",
            "run",
            "-p",
            "trellis-examples",
            "--example",
            "market_desk",
            "--",
            "--script",
            "market-lifecycle",
        ],
        || {
            let mut app = MarketDeskApp::new(sample_dataset());
            let terminal = app.open_terminal(opening_workspace());
            app.drain_effects();
            app.drain_output(terminal);
            app.drain_diagnostic_traces();

            app.apply_event(
                terminal,
                MarketDeskEvent::ReplaceWatchlist(rotated_watchlist()),
            );
            let rotate_watchlist = pop_trace(&mut app, "rotate-watchlist");

            app.apply_event(terminal, MarketDeskEvent::OpenChart("AMD".to_owned()));
            let open_depth_chart = pop_trace(&mut app, "open-depth-chart");

            app.apply_event(
                terminal,
                MarketDeskEvent::RevokeEntitlement {
                    symbol: "NVDA".to_owned(),
                },
            );
            let revoke_entitlement = pop_trace(&mut app, "revoke-entitlement");

            app.apply_event(
                terminal,
                MarketDeskEvent::ReplaceWatchlist(churn_symbols(128)),
            );
            let high_frequency_churn = pop_trace(&mut app, "high-frequency-churn");

            app.close(terminal);
            let close_workspace = pop_trace(&mut app, "close-workspace");

            vec![
                rotate_watchlist,
                open_depth_chart,
                revoke_entitlement,
                high_frequency_churn,
                close_workspace,
            ]
        },
    )
}

fn pop_trace(app: &mut MarketDeskApp, name: &str) -> ShowcaseStep {
    let trace = app
        .drain_diagnostic_traces()
        .pop()
        .expect("script step emits one trace");
    ShowcaseStep {
        name: name.to_owned(),
        host_statuses: Vec::new(),
        trace,
    }
}
