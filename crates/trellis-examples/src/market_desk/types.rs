use std::collections::{BTreeMap, BTreeSet};

/// Opaque handle for an open market terminal workspace.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct MarketTerminalHandle(pub u64);

/// Current terminal workspace inputs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketWorkspace {
    /// Active user id for entitlement lookup.
    pub user: String,
    /// Stable workspace id.
    pub workspace_id: String,
    /// Symbols visible in the watchlist grid.
    pub watchlist: BTreeSet<String>,
    /// Symbols with an open chart panel.
    pub open_charts: BTreeSet<String>,
}

/// Latest quote metadata known to the host application.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct MarketQuote {
    /// Last trade price in cents.
    pub last_price_cents: i64,
    /// Last known session volume.
    pub volume: u64,
}

/// Host-owned market data and entitlement snapshot.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MarketDataset {
    /// Entitled symbols by user id.
    pub entitlements: BTreeMap<String, BTreeSet<String>>,
    /// Latest quote metadata by symbol.
    pub quotes: BTreeMap<String, MarketQuote>,
}

/// Domain event applied to an open market terminal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MarketDeskEvent {
    /// Replace the watchlist symbols.
    ReplaceWatchlist(BTreeSet<String>),
    /// Open a chart panel for one symbol.
    OpenChart(String),
    /// Close a chart panel for one symbol.
    CloseChart(String),
    /// Revoke one symbol entitlement for the active user.
    RevokeEntitlement {
        /// Revoked symbol.
        symbol: String,
    },
    /// Replace all symbol entitlements for the active user.
    ReplaceEntitlements(BTreeSet<String>),
}

/// Host subscription or job controlled by the market terminal.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum MarketResource {
    /// Top-of-book quote feed.
    QuoteFeed {
        /// Market symbol.
        symbol: String,
    },
    /// Trade tape feed.
    TradeFeed {
        /// Market symbol.
        symbol: String,
    },
    /// Order-book depth feed used by chart panels.
    OrderBookDepth {
        /// Market symbol.
        symbol: String,
    },
    /// Candle stream used by chart panels.
    CandleStream {
        /// Market symbol.
        symbol: String,
    },
}

/// Host command payload used by Trellis resource planning.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum MarketCommand {
    /// Open the given market resource.
    Open(MarketResource),
}

/// Typed effect emitted to the market-data host executor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MarketEffect {
    /// Open the given resource.
    Open(MarketResource),
    /// Close the given resource.
    Close(MarketResource),
}

/// Materialized quote grid row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuoteRow {
    /// Market symbol.
    pub symbol: String,
    /// Last trade price in cents.
    pub last_price_cents: i64,
    /// Last known session volume.
    pub volume: u64,
}

/// Materialized chart panel state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChartPanel {
    /// Market symbol.
    pub symbol: String,
    /// Whether an order-book depth feed is active.
    pub has_depth: bool,
    /// Whether a candle stream is active.
    pub has_candles: bool,
}

/// Materialized market terminal output.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MarketSnapshot {
    /// Open workspace id, if any.
    pub workspace_id: Option<String>,
    /// Visible quote grid rows.
    pub rows: Vec<QuoteRow>,
    /// Visible chart panels.
    pub charts: Vec<ChartPanel>,
}

impl MarketSnapshot {
    /// Returns visible row symbols in deterministic order.
    pub fn row_symbols(&self) -> BTreeSet<String> {
        self.rows.iter().map(|row| row.symbol.clone()).collect()
    }
}

/// Public output frame emitted by the market terminal wrapper.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MarketFrame {
    /// Initial baseline frame.
    Baseline(MarketSnapshot),
    /// Incremental delta frame.
    Delta(MarketSnapshot),
    /// Explicit rebaseline frame.
    Rebaseline(MarketSnapshot),
    /// Clear frame emitted when the workspace scope closes.
    Cleared,
}

/// Count of wrapper effects and output frames emitted by an action.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MarketDeskUpdate {
    /// Number of market lifecycle effects queued.
    pub emitted_effects: usize,
    /// Number of terminal frames queued.
    pub emitted_frames: usize,
}
