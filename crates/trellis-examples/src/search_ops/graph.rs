use trellis_core::{DependencyList, Graph, InputNode, MaterializedOutput, ResourceKey, ScopeId};

use super::selectors::{
    allowed_docs, cache_resources, ranking_resources, reader_resources, search_snapshot,
    shards_for, visible_results,
};
use super::types::{SearchCatalog, SearchCommand, SearchResource, SearchSession, SearchSnapshot};

pub(super) struct SearchGraph {
    pub(super) graph: Graph<SearchCommand>,
    pub(super) session: InputNode<Option<SearchSession>>,
    pub(super) catalog: InputNode<SearchCatalog>,
    pub(super) search_scope: ScopeId,
    pub(super) output: MaterializedOutput<SearchSnapshot>,
}

pub(super) fn build_graph(catalog: SearchCatalog) -> SearchGraph {
    let mut graph = Graph::<SearchCommand>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let search_scope = tx.create_scope("search-workspace").unwrap();
    let session = tx.input::<Option<SearchSession>>("search-session").unwrap();
    let catalog_input = tx.input::<SearchCatalog>("search-catalog").unwrap();
    tx.set_input(session, None).unwrap();
    tx.set_input(catalog_input, catalog).unwrap();

    let allowed_docs = tx
        .set_collection(
            "search-allowed-docs",
            DependencyList::new([session.id(), catalog_input.id()]).unwrap(),
            move |ctx| Ok(allowed_docs(ctx.input(session)?, ctx.input(catalog_input)?)),
        )
        .unwrap();

    let shards = tx
        .set_collection(
            "search-allowed-shards",
            DependencyList::new([catalog_input.id(), allowed_docs.id()]).unwrap(),
            move |ctx| {
                Ok(shards_for(
                    ctx.input(catalog_input)?,
                    ctx.set_collection(allowed_docs)?,
                ))
            },
        )
        .unwrap();

    let visible_results = tx
        .set_collection(
            "search-visible-results",
            DependencyList::new([session.id(), catalog_input.id(), allowed_docs.id()]).unwrap(),
            move |ctx| {
                Ok(visible_results(
                    ctx.input(session)?,
                    ctx.input(catalog_input)?,
                    ctx.set_collection(allowed_docs)?,
                ))
            },
        )
        .unwrap();

    let readers = tx
        .set_collection(
            "search-shard-readers",
            DependencyList::new([shards.id()]).unwrap(),
            move |ctx| Ok(reader_resources(ctx.set_collection(shards)?)),
        )
        .unwrap();

    let ranking = tx
        .set_collection(
            "search-ranking-jobs",
            DependencyList::new([session.id(), shards.id()]).unwrap(),
            move |ctx| {
                Ok(ranking_resources(
                    ctx.input(session)?,
                    ctx.set_collection(shards)?,
                ))
            },
        )
        .unwrap();

    let cache = tx
        .set_collection(
            "search-cache-window",
            DependencyList::new([session.id(), visible_results.id()]).unwrap(),
            move |ctx| {
                Ok(cache_resources(
                    ctx.input(session)?,
                    ctx.set_collection(visible_results)?,
                ))
            },
        )
        .unwrap();

    for collection in [readers, ranking, cache] {
        tx.open_close_planner(collection, search_scope, resource_key, |resource| {
            SearchCommand::Open(resource.clone())
        })
        .unwrap();
    }

    let output = tx
        .materialized_output(
            "search-results-output",
            search_scope,
            DependencyList::new([session.id(), catalog_input.id(), allowed_docs.id()]).unwrap(),
            move |ctx| {
                Ok(search_snapshot(
                    ctx.input(session)?,
                    ctx.input(catalog_input)?,
                    ctx.set_collection(allowed_docs)?,
                ))
            },
        )
        .unwrap();

    tx.commit().unwrap();
    drop(tx);

    SearchGraph {
        graph,
        session,
        catalog: catalog_input,
        search_scope,
        output,
    }
}

pub(super) fn resource_key(resource: &SearchResource) -> ResourceKey {
    match resource {
        SearchResource::ShardReader { corpus, shard } => {
            ResourceKey::from_segments(["search", "reader", corpus.as_str(), shard.as_str()])
        }
        SearchResource::RankingJob {
            corpus,
            shard,
            query_key,
        } => ResourceKey::from_segments([
            "search",
            "ranking",
            corpus.as_str(),
            shard.as_str(),
            query_key.as_str(),
        ]),
        SearchResource::ResultCacheWindow {
            corpus,
            query_key,
            result_fingerprint,
            start,
            len,
        } => {
            let start = start.to_string();
            let len = len.to_string();
            ResourceKey::from_segments([
                "search",
                "cache",
                corpus.as_str(),
                query_key.as_str(),
                result_fingerprint.as_str(),
                start.as_str(),
                len.as_str(),
            ])
        }
    }
}

pub(super) fn resource_from_key(key: &ResourceKey) -> Option<SearchResource> {
    let segments = key.segments().collect::<Vec<_>>();
    match segments.as_slice() {
        ["search", "reader", corpus, shard] => Some(SearchResource::ShardReader {
            corpus: (*corpus).to_owned(),
            shard: (*shard).to_owned(),
        }),
        ["search", "ranking", corpus, shard, query_key] => Some(SearchResource::RankingJob {
            corpus: (*corpus).to_owned(),
            shard: (*shard).to_owned(),
            query_key: (*query_key).to_owned(),
        }),
        [
            "search",
            "cache",
            corpus,
            query_key,
            result_fingerprint,
            start,
            len,
        ] => Some(SearchResource::ResultCacheWindow {
            corpus: (*corpus).to_owned(),
            query_key: (*query_key).to_owned(),
            result_fingerprint: (*result_fingerprint).to_owned(),
            start: start.parse().ok()?,
            len: len.parse().ok()?,
        }),
        _ => None,
    }
}
