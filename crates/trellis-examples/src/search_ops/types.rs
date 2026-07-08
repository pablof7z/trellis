use std::collections::{BTreeMap, BTreeSet};

/// Opaque handle for an open search workspace.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SearchHandle(pub u64);

/// Search filter owned by the app.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SearchFilter {
    /// Search all documents.
    All,
    /// Require one tag.
    Tag(String),
}

/// Visible result window.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ResultWindow {
    /// First visible result index.
    pub start: usize,
    /// Maximum visible result count.
    pub len: usize,
}

/// Current search session inputs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchSession {
    /// Active user id.
    pub user: String,
    /// Selected corpus id.
    pub corpus: String,
    /// Query text.
    pub query: String,
    /// Query filter.
    pub filter: SearchFilter,
    /// Visible result window.
    pub window: ResultWindow,
}

/// Host-owned searchable document metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchDocument {
    /// Stable document id.
    pub id: String,
    /// Corpus id.
    pub corpus: String,
    /// Index shard id.
    pub shard: String,
    /// Display title.
    pub title: String,
    /// Searchable body text.
    pub body: String,
    /// Search tags.
    pub tags: BTreeSet<String>,
    /// Users allowed to see this document.
    pub allowed_users: BTreeSet<String>,
}

/// Host-owned search catalog.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SearchCatalog {
    /// Documents by id.
    pub documents: BTreeMap<String, SearchDocument>,
}

/// Domain event applied to an open search workspace.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SearchOpsEvent {
    /// Select a different corpus.
    SelectCorpus(String),
    /// Change the query text.
    ChangeQuery(String),
    /// Replace the query filter.
    ReplaceFilter(SearchFilter),
    /// Change the visible result window.
    SetWindow(ResultWindow),
    /// Revoke current-user access to one document.
    RevokeDocumentPermission {
        /// Revoked document id.
        doc_id: String,
    },
    /// Replace the host-owned search catalog.
    ReplaceCatalog(SearchCatalog),
}

/// Host resource controlled by SearchOps.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SearchResource {
    /// Index shard reader.
    ShardReader {
        /// Corpus id.
        corpus: String,
        /// Index shard id.
        shard: String,
    },
    /// Ranking job for one query/shard pair.
    RankingJob {
        /// Corpus id.
        corpus: String,
        /// Index shard id.
        shard: String,
        /// Normalized query and filter key.
        query_key: String,
    },
    /// Cache window for the visible result page.
    ResultCacheWindow {
        /// Corpus id.
        corpus: String,
        /// Normalized query and filter key.
        query_key: String,
        /// Stable fingerprint of visible result ids.
        result_fingerprint: String,
        /// First visible result index.
        start: usize,
        /// Maximum visible result count.
        len: usize,
    },
}

/// Host command payload used by Trellis resource planning.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum SearchCommand {
    /// Open the given search resource.
    Open(SearchResource),
}

/// Typed effect emitted to the search host executor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SearchEffect {
    /// Open the given resource.
    Open(SearchResource),
    /// Close the given resource.
    Close(SearchResource),
}

/// One materialized search result row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchResultRow {
    /// Document id.
    pub doc_id: String,
    /// Display title.
    pub title: String,
    /// Corpus id.
    pub corpus: String,
    /// Index shard id.
    pub shard: String,
}

/// Materialized bounded search result output.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SearchSnapshot {
    /// Current corpus, if search is open.
    pub corpus: Option<String>,
    /// Current query key.
    pub query_key: Option<String>,
    /// Total matching results before page bounding.
    pub total_results: usize,
    /// Visible result rows.
    pub rows: Vec<SearchResultRow>,
}

impl SearchSnapshot {
    /// Returns visible document ids in deterministic order.
    pub fn row_doc_ids(&self) -> BTreeSet<String> {
        self.rows.iter().map(|row| row.doc_id.clone()).collect()
    }
}

/// Public output frame emitted by the SearchOps wrapper.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SearchFrame {
    /// Initial baseline frame.
    Baseline(SearchSnapshot),
    /// Incremental delta frame.
    Delta(SearchSnapshot),
    /// Explicit rebaseline frame.
    Rebaseline(SearchSnapshot),
    /// Clear frame emitted when the search scope closes.
    Cleared,
}

/// Count of wrapper effects and output frames emitted by an action.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SearchOpsUpdate {
    /// Number of search lifecycle effects queued.
    pub emitted_effects: usize,
    /// Number of result frames queued.
    pub emitted_frames: usize,
}
