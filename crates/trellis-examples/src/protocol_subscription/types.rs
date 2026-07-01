use std::collections::{BTreeMap, BTreeSet};

use trellis_core::{InputNode, MaterializedOutput, ScopeId};

pub(super) type SourceCatalog = BTreeMap<(String, String), BTreeSet<String>>;
pub(super) type LocalRows = BTreeMap<String, Vec<ArticleRow>>;

/// Opaque application handle for one article feed session.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ArticleFeedHandle(pub(super) u64);

/// Parameters used to open one article feed session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArticleFeedParams {
    /// Account whose route should be observed.
    pub account: String,
    /// Application route whose source set should be observed.
    pub route: String,
    /// Maximum number of admitted local rows returned by the feed.
    pub limit: usize,
}

impl ArticleFeedParams {
    /// Creates feed parameters from application-owned identifiers.
    pub fn new(account: impl Into<String>, route: impl Into<String>, limit: usize) -> Self {
        Self {
            account: account.into(),
            route: route.into(),
            limit,
        }
    }
}

/// Protocol target for one live subscription.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SubscriptionTarget {
    /// Account owning the subscription target.
    pub account: String,
    /// Route owning the subscription target.
    pub route: String,
    /// Concrete source selected by the application source catalog.
    pub source: String,
}

/// Application-owned live subscription shape.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiveSubscription {
    /// Target to subscribe to.
    pub target: SubscriptionTarget,
    /// Maximum replay/live row count requested for this session.
    pub limit: usize,
    /// Monotonic replay generation selected by the wrapper.
    pub replay_epoch: u64,
}

/// Typed host effect emitted by the wrapper after graph propagation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SubscriptionEffect {
    /// Open a live subscription.
    Open(LiveSubscription),
    /// Replace an already-open subscription shape.
    Replace(LiveSubscription),
    /// Close a live subscription target.
    Close(SubscriptionTarget),
}

/// One admitted article row in the public feed output.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ArticleRow {
    /// Source that produced this row.
    pub source: String,
    /// Stable row identity within the source.
    pub id: String,
    /// Display body for the example row.
    pub body: String,
}

impl ArticleRow {
    /// Creates an article row from application-owned values.
    pub fn new(source: impl Into<String>, id: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            id: id.into(),
            body: body.into(),
        }
    }
}

/// Typed output frame returned by `ArticleFeedApp::poll_output`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArticleFeedFrame {
    /// Complete current state for a newly opened session.
    Baseline(Vec<ArticleRow>),
    /// Replacement state after ordinary input changes.
    Delta(Vec<ArticleRow>),
    /// Complete current state after an explicit replay request.
    Replay(Vec<ArticleRow>),
    /// Terminal frame after the session handle is closed.
    Cleared,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct FeedSnapshot {
    pub(super) rows: Vec<ArticleRow>,
    pub(super) replay_epoch: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ReplaySelector {
    pub(super) limit: usize,
    pub(super) replay_epoch: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum ProtocolCommand {
    Subscribe(LiveSubscription),
}

#[derive(Clone)]
pub(super) struct InternalSession {
    pub(super) scope: ScopeId,
    pub(super) replay_epoch: InputNode<u64>,
    pub(super) output: MaterializedOutput<FeedSnapshot>,
}
