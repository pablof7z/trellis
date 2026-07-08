use std::collections::{BTreeMap, BTreeSet};

use super::types::{MarketDataset, MarketQuote, MarketWorkspace};

/// Builds a sorted symbol set from literal values.
pub fn symbols<const N: usize>(values: [&str; N]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

/// Builds deterministic synthetic symbols for churn validation.
pub fn churn_symbols(count: usize) -> BTreeSet<String> {
    (0..count).map(|index| format!("SYM{index:03}")).collect()
}

/// Sample dataset for the MarketDesk showcase.
pub fn sample_dataset() -> MarketDataset {
    let mut quotes = BTreeMap::new();
    quotes.insert("AAPL".to_owned(), quote(21_234, 90_100_000));
    quotes.insert("AMD".to_owned(), quote(17_420, 62_000_000));
    quotes.insert("NVDA".to_owned(), quote(14_105, 210_000_000));
    quotes.insert("TSLA".to_owned(), quote(32_218, 112_500_000));

    let mut entitled = symbols(["AAPL", "AMD", "NVDA", "TSLA"]);
    for (index, symbol) in churn_symbols(160).into_iter().enumerate() {
        quotes.insert(
            symbol.clone(),
            quote(10_000 + index as i64, 1_000_000 + index as u64),
        );
        entitled.insert(symbol);
    }

    let mut entitlements = BTreeMap::new();
    entitlements.insert("analyst".to_owned(), entitled);
    MarketDataset {
        entitlements,
        quotes,
    }
}

/// Opening workspace used by the headless script.
pub fn opening_workspace() -> MarketWorkspace {
    MarketWorkspace {
        user: "analyst".to_owned(),
        workspace_id: "growth-desk".to_owned(),
        watchlist: symbols(["AAPL", "TSLA", "NVDA"]),
        open_charts: symbols(["TSLA"]),
    }
}

/// Watchlist after rotating out TSLA and adding AMD.
pub fn rotated_watchlist() -> BTreeSet<String> {
    symbols(["AAPL", "NVDA", "AMD"])
}

fn quote(last_price_cents: i64, volume: u64) -> MarketQuote {
    MarketQuote {
        last_price_cents,
        volume,
    }
}
