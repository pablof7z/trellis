use std::collections::BTreeSet;

use super::types::{
    SearchCatalog, SearchDocument, SearchFilter, SearchResource, SearchResultRow, SearchSession,
    SearchSnapshot,
};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(super) struct ShardKey {
    corpus: String,
    shard: String,
}

pub(super) fn allowed_docs(
    session: &Option<SearchSession>,
    catalog: &SearchCatalog,
) -> BTreeSet<String> {
    let Some(session) = session else {
        return BTreeSet::new();
    };
    catalog
        .documents
        .values()
        .filter(|doc| doc.corpus == session.corpus && doc.allowed_users.contains(&session.user))
        .map(|doc| doc.id.clone())
        .collect()
}

pub(super) fn shards_for(catalog: &SearchCatalog, docs: &BTreeSet<String>) -> BTreeSet<ShardKey> {
    docs.iter()
        .filter_map(|id| catalog.documents.get(id))
        .map(|doc| ShardKey {
            corpus: doc.corpus.clone(),
            shard: doc.shard.clone(),
        })
        .collect()
}

pub(super) fn visible_results(
    session: &Option<SearchSession>,
    catalog: &SearchCatalog,
    allowed: &BTreeSet<String>,
) -> BTreeSet<String> {
    let Some(session) = session else {
        return BTreeSet::new();
    };
    result_ids(session, catalog, allowed)
        .into_iter()
        .skip(session.window.start)
        .take(session.window.len)
        .collect()
}

pub(super) fn reader_resources(shards: &BTreeSet<ShardKey>) -> BTreeSet<SearchResource> {
    shards
        .iter()
        .map(|shard| SearchResource::ShardReader {
            corpus: shard.corpus.clone(),
            shard: shard.shard.clone(),
        })
        .collect()
}

pub(super) fn ranking_resources(
    session: &Option<SearchSession>,
    shards: &BTreeSet<ShardKey>,
) -> BTreeSet<SearchResource> {
    let Some(session) = session else {
        return BTreeSet::new();
    };
    let query_key = query_key(session);
    shards
        .iter()
        .map(|shard| SearchResource::RankingJob {
            corpus: shard.corpus.clone(),
            shard: shard.shard.clone(),
            query_key: query_key.clone(),
        })
        .collect()
}

pub(super) fn cache_resources(
    session: &Option<SearchSession>,
    visible: &BTreeSet<String>,
) -> BTreeSet<SearchResource> {
    let Some(session) = session else {
        return BTreeSet::new();
    };
    (!visible.is_empty())
        .then(|| SearchResource::ResultCacheWindow {
            corpus: session.corpus.clone(),
            query_key: query_key(session),
            result_fingerprint: visible.iter().cloned().collect::<Vec<_>>().join("|"),
            start: session.window.start,
            len: session.window.len,
        })
        .into_iter()
        .collect()
}

pub(super) fn search_snapshot(
    session: &Option<SearchSession>,
    catalog: &SearchCatalog,
    allowed: &BTreeSet<String>,
) -> SearchSnapshot {
    let Some(session) = session else {
        return SearchSnapshot::default();
    };
    let results = result_ids(session, catalog, allowed);
    let rows = results
        .iter()
        .skip(session.window.start)
        .take(session.window.len)
        .filter_map(|id| catalog.documents.get(id))
        .map(row)
        .collect();
    SearchSnapshot {
        corpus: Some(session.corpus.clone()),
        query_key: Some(query_key(session)),
        total_results: results.len(),
        rows,
    }
}

fn query_key(session: &SearchSession) -> String {
    let filter = match &session.filter {
        SearchFilter::All => "all".to_owned(),
        SearchFilter::Tag(tag) => format!("tag-{tag}"),
    };
    format!(
        "{}:{filter}",
        session.query.to_lowercase().replace(' ', "_")
    )
}

fn result_ids(
    session: &SearchSession,
    catalog: &SearchCatalog,
    allowed: &BTreeSet<String>,
) -> Vec<String> {
    allowed
        .iter()
        .filter_map(|id| catalog.documents.get(id))
        .filter(|doc| query_matches(session, doc))
        .map(|doc| doc.id.clone())
        .collect()
}

fn query_matches(session: &SearchSession, doc: &SearchDocument) -> bool {
    let query = session.query.to_lowercase();
    let text = format!("{} {}", doc.title, doc.body).to_lowercase();
    let filter_matches = match &session.filter {
        SearchFilter::All => true,
        SearchFilter::Tag(tag) => doc.tags.contains(tag),
    };
    filter_matches && text.contains(&query)
}

fn row(doc: &SearchDocument) -> SearchResultRow {
    SearchResultRow {
        doc_id: doc.id.clone(),
        title: doc.title.clone(),
        corpus: doc.corpus.clone(),
        shard: doc.shard.clone(),
    }
}
