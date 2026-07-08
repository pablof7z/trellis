use std::collections::BTreeSet;

use trellis_core::{DependencyList, Graph, InputNode, ResourceKey, ScopeId};

use super::types::{
    ChartPanel, MarketCommand, MarketDataset, MarketQuote, MarketResource, MarketSnapshot,
    MarketWorkspace, QuoteRow,
};

pub(super) struct MarketGraph {
    pub(super) graph: Graph<MarketCommand>,
    pub(super) workspace: InputNode<Option<MarketWorkspace>>,
    pub(super) dataset: InputNode<MarketDataset>,
    pub(super) workspace_scope: ScopeId,
}

pub(super) fn build_graph(dataset: MarketDataset) -> MarketGraph {
    let mut graph = Graph::<MarketCommand>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let workspace_scope = tx.create_scope("market-workspace").unwrap();
    let grid_scope = tx
        .create_scope_with_parent("quote-grid", Some(workspace_scope))
        .unwrap();
    let chart_scope = tx
        .create_scope_with_parent("chart-panels", Some(workspace_scope))
        .unwrap();
    let workspace = tx
        .input::<Option<MarketWorkspace>>("market-workspace-input")
        .unwrap();
    let dataset_input = tx.input::<MarketDataset>("market-dataset").unwrap();
    tx.set_input(workspace, None).unwrap();
    tx.set_input(dataset_input, dataset).unwrap();

    let visible_symbols = tx
        .set_collection(
            "market-visible-symbols",
            DependencyList::new([workspace.id(), dataset_input.id()]).unwrap(),
            move |ctx| {
                Ok(visible_symbols(
                    ctx.input(workspace)?,
                    ctx.input(dataset_input)?,
                ))
            },
        )
        .unwrap();

    let grid_resources = tx
        .set_collection(
            "market-grid-resources",
            DependencyList::new([visible_symbols.id()]).unwrap(),
            move |ctx| Ok(grid_resources(ctx.set_collection(visible_symbols)?)),
        )
        .unwrap();

    let chart_symbols = tx
        .set_collection(
            "market-chart-symbols",
            DependencyList::new([workspace.id(), visible_symbols.id()]).unwrap(),
            move |ctx| {
                Ok(chart_symbols(
                    ctx.input(workspace)?,
                    ctx.set_collection(visible_symbols)?,
                ))
            },
        )
        .unwrap();

    let chart_resources = tx
        .set_collection(
            "market-chart-resources",
            DependencyList::new([chart_symbols.id()]).unwrap(),
            move |ctx| Ok(chart_resources(ctx.set_collection(chart_symbols)?)),
        )
        .unwrap();

    tx.open_close_planner(grid_resources, grid_scope, resource_key, |resource| {
        MarketCommand::Open(resource.clone())
    })
    .unwrap();
    tx.open_close_planner(chart_resources, chart_scope, resource_key, |resource| {
        MarketCommand::Open(resource.clone())
    })
    .unwrap();

    tx.materialized_output(
        "market-terminal-output",
        workspace_scope,
        DependencyList::new([
            workspace.id(),
            dataset_input.id(),
            visible_symbols.id(),
            chart_symbols.id(),
        ])
        .unwrap(),
        move |ctx| {
            Ok(terminal_snapshot(
                ctx.input(workspace)?,
                ctx.input(dataset_input)?,
                ctx.set_collection(visible_symbols)?,
                ctx.set_collection(chart_symbols)?,
            ))
        },
    )
    .unwrap();

    tx.commit().unwrap();
    drop(tx);

    MarketGraph {
        graph,
        workspace,
        dataset: dataset_input,
        workspace_scope,
    }
}

pub(super) fn resource_key(resource: &MarketResource) -> ResourceKey {
    match resource {
        MarketResource::QuoteFeed { symbol } => {
            ResourceKey::from_segments(["market-desk", "quote", symbol.as_str()])
        }
        MarketResource::TradeFeed { symbol } => {
            ResourceKey::from_segments(["market-desk", "trade", symbol.as_str()])
        }
        MarketResource::OrderBookDepth { symbol } => {
            ResourceKey::from_segments(["market-desk", "depth", symbol.as_str()])
        }
        MarketResource::CandleStream { symbol } => {
            ResourceKey::from_segments(["market-desk", "candle", symbol.as_str()])
        }
    }
}

pub(super) fn resource_from_key(key: &ResourceKey) -> Option<MarketResource> {
    let segments = key.segments().collect::<Vec<_>>();
    match segments.as_slice() {
        ["market-desk", "quote", symbol] => Some(MarketResource::QuoteFeed {
            symbol: (*symbol).to_owned(),
        }),
        ["market-desk", "trade", symbol] => Some(MarketResource::TradeFeed {
            symbol: (*symbol).to_owned(),
        }),
        ["market-desk", "depth", symbol] => Some(MarketResource::OrderBookDepth {
            symbol: (*symbol).to_owned(),
        }),
        ["market-desk", "candle", symbol] => Some(MarketResource::CandleStream {
            symbol: (*symbol).to_owned(),
        }),
        _ => None,
    }
}

fn visible_symbols(
    workspace: &Option<MarketWorkspace>,
    dataset: &MarketDataset,
) -> BTreeSet<String> {
    let Some(workspace) = workspace else {
        return BTreeSet::new();
    };
    let Some(entitled) = dataset.entitlements.get(&workspace.user) else {
        return BTreeSet::new();
    };
    workspace
        .watchlist
        .intersection(entitled)
        .cloned()
        .collect()
}

fn chart_symbols(
    workspace: &Option<MarketWorkspace>,
    visible_symbols: &BTreeSet<String>,
) -> BTreeSet<String> {
    let Some(workspace) = workspace else {
        return BTreeSet::new();
    };
    workspace
        .open_charts
        .intersection(visible_symbols)
        .cloned()
        .collect()
}

fn grid_resources(symbols: &BTreeSet<String>) -> BTreeSet<MarketResource> {
    let mut resources = BTreeSet::new();
    for symbol in symbols {
        resources.insert(MarketResource::QuoteFeed {
            symbol: symbol.clone(),
        });
        resources.insert(MarketResource::TradeFeed {
            symbol: symbol.clone(),
        });
    }
    resources
}

fn chart_resources(symbols: &BTreeSet<String>) -> BTreeSet<MarketResource> {
    let mut resources = BTreeSet::new();
    for symbol in symbols {
        resources.insert(MarketResource::QuoteFeed {
            symbol: symbol.clone(),
        });
        resources.insert(MarketResource::OrderBookDepth {
            symbol: symbol.clone(),
        });
        resources.insert(MarketResource::CandleStream {
            symbol: symbol.clone(),
        });
    }
    resources
}

fn terminal_snapshot(
    workspace: &Option<MarketWorkspace>,
    dataset: &MarketDataset,
    visible_symbols: &BTreeSet<String>,
    chart_symbols: &BTreeSet<String>,
) -> MarketSnapshot {
    let mut rows = Vec::new();
    for symbol in visible_symbols {
        let quote = dataset.quotes.get(symbol).copied().unwrap_or_default();
        rows.push(row(symbol, quote));
    }
    let charts = chart_symbols
        .iter()
        .cloned()
        .map(|symbol| ChartPanel {
            symbol,
            has_depth: true,
            has_candles: true,
        })
        .collect();
    MarketSnapshot {
        workspace_id: workspace
            .as_ref()
            .map(|workspace| workspace.workspace_id.clone()),
        rows,
        charts,
    }
}

fn row(symbol: &str, quote: MarketQuote) -> QuoteRow {
    QuoteRow {
        symbol: symbol.to_owned(),
        last_price_cents: quote.last_price_cents,
        volume: quote.volume,
    }
}
