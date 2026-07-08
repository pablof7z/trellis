use super::*;

fn open_market() -> (MarketDeskApp, MarketTerminalHandle) {
    let mut app = MarketDeskApp::new(sample_dataset());
    let handle = app.open_terminal(opening_workspace());
    app.drain_effects();
    app.drain_output(handle);
    app.drain_diagnostic_traces();
    (app, handle)
}

#[test]
fn symbol_rotation_closes_removed_feeds_and_opens_added_symbol() {
    let (mut app, handle) = open_market();

    app.apply_event(
        handle,
        MarketDeskEvent::ReplaceWatchlist(rotated_watchlist()),
    );
    let effects = app.drain_effects();
    assert!(
        effects.contains(&MarketEffect::Close(MarketResource::QuoteFeed {
            symbol: "TSLA".to_owned(),
        }))
    );
    assert!(
        effects.contains(&MarketEffect::Close(MarketResource::TradeFeed {
            symbol: "TSLA".to_owned(),
        }))
    );
    assert!(
        effects.contains(&MarketEffect::Close(MarketResource::OrderBookDepth {
            symbol: "TSLA".to_owned(),
        }))
    );
    assert!(
        effects.contains(&MarketEffect::Open(MarketResource::QuoteFeed {
            symbol: "AMD".to_owned(),
        }))
    );
    assert!(matches!(
        app.drain_output(handle).first(),
        Some(MarketFrame::Delta(snapshot))
            if !snapshot.row_symbols().contains("TSLA")
                && snapshot.row_symbols().contains("AMD")
    ));
}

#[test]
fn chart_open_starts_depth_and_chart_close_preserves_grid_quote() {
    let (mut app, handle) = open_market();

    app.apply_event(handle, MarketDeskEvent::OpenChart("AAPL".to_owned()));
    let open_effects = app.drain_effects();
    assert!(
        open_effects.contains(&MarketEffect::Open(MarketResource::OrderBookDepth {
            symbol: "AAPL".to_owned(),
        }))
    );
    assert!(
        open_effects.contains(&MarketEffect::Open(MarketResource::CandleStream {
            symbol: "AAPL".to_owned(),
        }))
    );

    app.apply_event(handle, MarketDeskEvent::CloseChart("AAPL".to_owned()));
    let close_effects = app.drain_effects();
    assert!(
        close_effects.contains(&MarketEffect::Close(MarketResource::OrderBookDepth {
            symbol: "AAPL".to_owned(),
        }))
    );
    assert!(
        !close_effects.contains(&MarketEffect::Close(MarketResource::QuoteFeed {
            symbol: "AAPL".to_owned(),
        }))
    );
}

#[test]
fn entitlement_revoke_closes_forbidden_feeds_and_clears_rows() {
    let (mut app, handle) = open_market();

    app.apply_event(
        handle,
        MarketDeskEvent::RevokeEntitlement {
            symbol: "NVDA".to_owned(),
        },
    );
    let effects = app.drain_effects();
    assert!(
        effects.contains(&MarketEffect::Close(MarketResource::QuoteFeed {
            symbol: "NVDA".to_owned(),
        }))
    );
    assert!(
        effects.contains(&MarketEffect::Close(MarketResource::TradeFeed {
            symbol: "NVDA".to_owned(),
        }))
    );
    assert!(matches!(
        app.drain_output(handle).first(),
        Some(MarketFrame::Delta(snapshot)) if !snapshot.row_symbols().contains("NVDA")
    ));
}

#[test]
fn workspace_close_closes_streams_and_clears_output() {
    let (mut app, handle) = open_market();

    app.close(handle);
    let effects = app.drain_effects();
    assert!(
        effects.contains(&MarketEffect::Close(MarketResource::QuoteFeed {
            symbol: "AAPL".to_owned(),
        }))
    );
    assert!(
        effects.contains(&MarketEffect::Close(MarketResource::TradeFeed {
            symbol: "AAPL".to_owned(),
        }))
    );
    assert!(
        effects.contains(&MarketEffect::Close(MarketResource::OrderBookDepth {
            symbol: "TSLA".to_owned(),
        }))
    );
    assert!(app.drain_output(handle).contains(&MarketFrame::Cleared));
}

#[test]
fn high_frequency_churn_emits_large_diff_and_keeps_oracle_green() {
    let (mut app, handle) = open_market();

    let update = app.apply_event(
        handle,
        MarketDeskEvent::ReplaceWatchlist(churn_symbols(128)),
    );
    assert!(update.emitted_effects >= 256);
    let trace = app.drain_diagnostic_traces().pop().unwrap();
    assert!(trace.resource_commands.len() >= 256);
    assert!(
        trace
            .invariant_results
            .iter()
            .any(|result| result.name == "incremental_equals_full_recompute" && result.passed)
    );
}

#[test]
fn market_lifecycle_trace_uses_showcase_contract() {
    let trace = market_lifecycle_showcase_trace();

    assert_eq!(trace.showcase, "market-desk");
    assert_eq!(trace.script, "market-lifecycle");
    assert_eq!(trace.replay.status, "passed");
    assert_eq!(
        trace
            .steps
            .iter()
            .map(|step| step.name.as_str())
            .collect::<Vec<_>>(),
        vec![
            "rotate-watchlist",
            "open-depth-chart",
            "revoke-entitlement",
            "high-frequency-churn",
            "close-workspace",
        ]
    );
    assert!(trace.steps.iter().all(|step| {
        step.trace
            .invariant_results
            .iter()
            .any(|result| result.name == "incremental_equals_full_recompute" && result.passed)
    }));
}

#[test]
fn seeded_bug_capsule_detects_stale_revoked_symbol() {
    let report = run_bug_capsule("market-entitlement-revoke-closes-feeds").unwrap();

    assert_eq!(report.status, "pass");
    assert!(report.expected_failures_detected);
    assert_eq!(available_bug_capsules().len(), 1);
}
