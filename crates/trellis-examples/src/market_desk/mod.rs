//! MarketDesk live market-data terminal secondary showcase.

mod bug_capsules;
mod engine;
mod graph;
mod sample;
mod scripts;
mod types;

#[cfg(test)]
mod tests;

pub use bug_capsules::{available_bug_capsules, run_all_bug_capsules, run_bug_capsule};
pub use engine::MarketDeskApp;
pub use sample::{churn_symbols, opening_workspace, rotated_watchlist, sample_dataset, symbols};
pub use scripts::market_lifecycle_showcase_trace;
pub use types::{
    ChartPanel, MarketDataset, MarketDeskEvent, MarketDeskUpdate, MarketEffect, MarketFrame,
    MarketQuote, MarketResource, MarketSnapshot, MarketTerminalHandle, MarketWorkspace, QuoteRow,
};
