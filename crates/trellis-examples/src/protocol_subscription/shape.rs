use std::collections::{BTreeMap, BTreeSet};

use trellis_core::{
    DependencyList, Graph, InputNode, ResourceKey, ResourcePlan, TransactionResult,
};

use super::types::{
    ArticleFeedParams, ArticleRow, FeedSnapshot, InternalSession, LiveSubscription, LocalRows,
    ProtocolCommand, ReplaySelector, SourceCatalog, SubscriptionTarget,
};

pub(super) fn open_session(
    graph: &mut Graph<ProtocolCommand, FeedSnapshot>,
    source_catalog: InputNode<SourceCatalog>,
    local_rows: InputNode<LocalRows>,
    params: ArticleFeedParams,
) -> (
    InternalSession,
    TransactionResult<ProtocolCommand, FeedSnapshot>,
) {
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("article-feed-session").unwrap();
    let params_input = tx.input::<ArticleFeedParams>("session-params").unwrap();
    let replay_epoch = tx.input::<u64>("replay-epoch").unwrap();
    tx.set_input(params_input, params).unwrap();
    tx.set_input(replay_epoch, 0).unwrap();

    let source_set = tx
        .derived(
            "source-set",
            DependencyList::new([params_input.id(), source_catalog.id()]).unwrap(),
            move |ctx| {
                let params = ctx.input(params_input)?;
                Ok(ctx
                    .input(source_catalog)?
                    .get(&(params.account.clone(), params.route.clone()))
                    .cloned()
                    .unwrap_or_default())
            },
        )
        .unwrap();
    let interest_set = tx
        .derived(
            "desired-interest-set",
            DependencyList::new([params_input.id(), source_set.id()]).unwrap(),
            move |ctx| {
                let params = ctx.input(params_input)?;
                Ok(ctx
                    .derived(source_set)?
                    .iter()
                    .map(|source| SubscriptionTarget {
                        account: params.account.clone(),
                        route: params.route.clone(),
                        source: source.clone(),
                    })
                    .collect::<BTreeSet<_>>())
            },
        )
        .unwrap();
    let replay_selector = tx
        .derived(
            "replay-selector",
            DependencyList::new([params_input.id(), replay_epoch.id()]).unwrap(),
            move |ctx| {
                let params = ctx.input(params_input)?;
                Ok(ReplaySelector {
                    limit: params.limit,
                    replay_epoch: *ctx.input(replay_epoch)?,
                })
            },
        )
        .unwrap();
    let live_shape = tx
        .map_collection(
            "live-subscription-shape",
            DependencyList::new([interest_set.id(), replay_selector.id()]).unwrap(),
            move |ctx| {
                let selector = ctx.derived(replay_selector)?;
                Ok(ctx
                    .derived(interest_set)?
                    .iter()
                    .map(|target| {
                        (
                            target.clone(),
                            LiveSubscription {
                                target: target.clone(),
                                limit: selector.limit,
                                replay_epoch: selector.replay_epoch,
                            },
                        )
                    })
                    .collect::<BTreeMap<_, _>>())
            },
        )
        .unwrap();
    tx.map_resource_planner(live_shape, scope, move |ctx| {
        let mut plan = ResourcePlan::new();
        for added in &ctx.diff().added {
            let (target, shape) = &added.value;
            plan.open(
                resource_key(target),
                ctx.scope(),
                ProtocolCommand::Subscribe(shape.clone()),
            );
        }
        for updated in &ctx.diff().updated {
            plan.replace(
                resource_key(&updated.key),
                ctx.scope(),
                ProtocolCommand::Subscribe(updated.current.clone()),
            );
        }
        for removed in &ctx.diff().removed {
            let (target, _) = &removed.value;
            plan.close(resource_key(target), ctx.scope());
        }
        Ok(plan)
    })
    .unwrap();

    let admitted_rows = tx
        .derived(
            "admitted-local-rows",
            DependencyList::new([local_rows.id(), source_set.id(), params_input.id()]).unwrap(),
            move |ctx| {
                let params = ctx.input(params_input)?;
                Ok(admit_rows(
                    ctx.input(local_rows)?,
                    ctx.derived(source_set)?,
                    params.limit,
                ))
            },
        )
        .unwrap();
    let output = tx
        .materialized_output(
            "article-feed-output",
            scope,
            DependencyList::new([admitted_rows.id(), replay_selector.id()]).unwrap(),
            move |ctx| {
                Ok(FeedSnapshot {
                    rows: ctx.derived(admitted_rows)?.clone(),
                    replay_epoch: ctx.derived(replay_selector)?.replay_epoch,
                })
            },
        )
        .unwrap();
    let result = tx.commit().unwrap();
    drop(tx);

    (
        InternalSession {
            scope,
            replay_epoch,
            output,
        },
        result,
    )
}

pub(super) fn resource_key(target: &SubscriptionTarget) -> ResourceKey {
    ResourceKey::new(format!(
        "article-feed/{}/{}/{}",
        target.account, target.route, target.source
    ))
}

pub(super) fn target_from_key(key: &ResourceKey) -> SubscriptionTarget {
    let mut parts = key.as_str().splitn(4, '/');
    let _prefix = parts.next();
    let account = parts.next().unwrap_or_default().to_owned();
    let route = parts.next().unwrap_or_default().to_owned();
    let source = parts.next().unwrap_or_default().to_owned();
    SubscriptionTarget {
        account,
        route,
        source,
    }
}

fn admit_rows(local_rows: &LocalRows, sources: &BTreeSet<String>, limit: usize) -> Vec<ArticleRow> {
    let mut rows = sources
        .iter()
        .filter_map(|source| local_rows.get(source))
        .flatten()
        .cloned()
        .collect::<Vec<_>>();
    rows.sort();
    rows.truncate(limit);
    rows
}
