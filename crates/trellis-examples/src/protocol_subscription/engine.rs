use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[cfg(test)]
use trellis_core::ScopeId;
use trellis_core::{
    Graph, InputNode, OutputFrameKind, OutputKey, ResourceCommand, TransactionResult,
};

use super::shape::{open_session, target_from_key};
use super::types::{
    ArticleFeedFrame, ArticleFeedHandle, ArticleFeedParams, ArticleRow, FeedSnapshot,
    InternalSession, LocalRows, ProtocolCommand, SourceCatalog, SubscriptionEffect,
};

/// Article feed wrapper whose public API does not expose Trellis types.
pub struct ArticleFeedApp {
    graph: Graph<ProtocolCommand, FeedSnapshot>,
    source_catalog: InputNode<SourceCatalog>,
    local_rows: InputNode<LocalRows>,
    next_handle: u64,
    sessions: BTreeMap<ArticleFeedHandle, InternalSession>,
    output_handles: BTreeMap<OutputKey, ArticleFeedHandle>,
    output_queues: BTreeMap<ArticleFeedHandle, VecDeque<ArticleFeedFrame>>,
    subscription_effects: VecDeque<SubscriptionEffect>,
    #[cfg(test)]
    results: Vec<TransactionResult<ProtocolCommand, FeedSnapshot>>,
}

impl ArticleFeedApp {
    /// Creates an empty wrapper with global source and local-row inputs.
    pub fn new() -> Self {
        let mut graph = Graph::<ProtocolCommand, FeedSnapshot>::new_with_command_type();
        let mut tx = graph.begin_transaction().unwrap();
        let source_catalog = tx.input::<SourceCatalog>("source-catalog").unwrap();
        let local_rows = tx.input::<LocalRows>("local-rows").unwrap();
        tx.set_input(source_catalog, SourceCatalog::new()).unwrap();
        tx.set_input(local_rows, LocalRows::new()).unwrap();
        let result = tx.commit().unwrap();
        drop(tx);
        debug_assert!(result.resource_plan.commands().is_empty());
        debug_assert!(result.output_frames.is_empty());

        Self {
            graph,
            source_catalog,
            local_rows,
            next_handle: 1,
            sessions: BTreeMap::new(),
            output_handles: BTreeMap::new(),
            output_queues: BTreeMap::new(),
            subscription_effects: VecDeque::new(),
            #[cfg(test)]
            results: Vec::new(),
        }
    }

    /// Replaces the source set for one account route.
    pub fn set_route_sources<I, S>(&mut self, account: &str, route: &str, sources: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut catalog = self.current_source_catalog();
        let key = (account.to_owned(), route.to_owned());
        let sources = sources.into_iter().map(Into::into).collect::<BTreeSet<_>>();
        if sources.is_empty() {
            catalog.remove(&key);
        } else {
            catalog.insert(key, sources);
        }

        let mut tx = self.graph.begin_transaction().unwrap();
        tx.set_input(self.source_catalog, catalog).unwrap();
        let result = tx.commit().unwrap();
        drop(tx);
        self.apply_result(result);
    }

    /// Replaces locally admitted rows for one source.
    pub fn replace_source_rows(&mut self, source: &str, rows: Vec<ArticleRow>) {
        let mut local_rows = self.current_local_rows();
        local_rows.insert(source.to_owned(), rows);

        let mut tx = self.graph.begin_transaction().unwrap();
        tx.set_input(self.local_rows, local_rows).unwrap();
        let result = tx.commit().unwrap();
        drop(tx);
        self.apply_result(result);
    }

    /// Opens an article feed session and returns an opaque public handle.
    pub fn open_article_feed(&mut self, params: ArticleFeedParams) -> ArticleFeedHandle {
        let handle = ArticleFeedHandle(self.next_handle);
        self.next_handle += 1;

        let (session, result) = open_session(
            &mut self.graph,
            self.source_catalog,
            self.local_rows,
            params,
        );
        self.output_handles.insert(session.output.key(), handle);
        self.sessions.insert(handle, session);
        self.apply_result(result);
        handle
    }

    /// Requests an explicit replay/rebaseline for an open feed handle.
    pub fn request_replay(&mut self, handle: ArticleFeedHandle) {
        let Some(session) = self.sessions.get(&handle).cloned() else {
            return;
        };
        let current = *self
            .graph
            .input_value(session.replay_epoch)
            .unwrap()
            .unwrap_or(&0);

        let mut tx = self.graph.begin_transaction().unwrap();
        tx.set_input(session.replay_epoch, current + 1).unwrap();
        tx.rebaseline_output(session.output).unwrap();
        let result = tx.commit().unwrap();
        drop(tx);
        self.apply_result(result);
    }

    /// Closes a feed session and tears down its scoped resources.
    pub fn close(&mut self, handle: ArticleFeedHandle) {
        let Some(session) = self.sessions.get(&handle).cloned() else {
            return;
        };

        let mut tx = self.graph.begin_transaction().unwrap();
        tx.close_scope(session.scope).unwrap();
        let result = tx.commit().unwrap();
        drop(tx);
        self.apply_result(result);

        self.sessions.remove(&handle);
        self.output_handles.remove(&session.output.key());
    }

    /// Drains queued public output frames for one handle.
    pub fn poll_output(&mut self, handle: ArticleFeedHandle) -> Vec<ArticleFeedFrame> {
        self.output_queues
            .remove(&handle)
            .unwrap_or_default()
            .into_iter()
            .collect()
    }

    /// Drains queued typed subscription effects.
    pub fn drain_subscription_effects(&mut self) -> Vec<SubscriptionEffect> {
        self.subscription_effects.drain(..).collect()
    }

    fn current_source_catalog(&self) -> SourceCatalog {
        self.graph
            .input_value(self.source_catalog)
            .unwrap()
            .cloned()
            .unwrap_or_default()
    }

    fn current_local_rows(&self) -> LocalRows {
        self.graph
            .input_value(self.local_rows)
            .unwrap()
            .cloned()
            .unwrap_or_default()
    }

    fn apply_result(&mut self, result: TransactionResult<ProtocolCommand, FeedSnapshot>) {
        for command in result.resource_plan.commands() {
            match command {
                ResourceCommand::Open {
                    command: ProtocolCommand::Subscribe(shape),
                    ..
                } => self
                    .subscription_effects
                    .push_back(SubscriptionEffect::Open(shape.clone())),
                ResourceCommand::Replace {
                    command: ProtocolCommand::Subscribe(shape),
                    ..
                } => self
                    .subscription_effects
                    .push_back(SubscriptionEffect::Replace(shape.clone())),
                ResourceCommand::Close { key, .. } => self
                    .subscription_effects
                    .push_back(SubscriptionEffect::Close(target_from_key(key))),
                ResourceCommand::Refresh { .. } => {}
            }
        }

        for frame in &result.output_frames {
            let Some(handle) = self.output_handles.get(&frame.output_key).copied() else {
                continue;
            };
            let frame = match &frame.kind {
                OutputFrameKind::Baseline(snapshot) => {
                    ArticleFeedFrame::Baseline(snapshot.rows.clone())
                }
                OutputFrameKind::Delta(snapshot) => ArticleFeedFrame::Delta(snapshot.rows.clone()),
                OutputFrameKind::Rebaseline(snapshot, _) => {
                    ArticleFeedFrame::Replay(snapshot.rows.clone())
                }
                OutputFrameKind::Clear(_) => ArticleFeedFrame::Cleared,
            };
            self.output_queues
                .entry(handle)
                .or_default()
                .push_back(frame);
        }

        #[cfg(test)]
        self.results.push(result);
    }

    #[cfg(test)]
    pub(super) fn last_result(&self) -> &TransactionResult<ProtocolCommand, FeedSnapshot> {
        self.results
            .last()
            .expect("a transaction result was recorded")
    }

    #[cfg(test)]
    pub(super) fn session_scope(&self, handle: ArticleFeedHandle) -> ScopeId {
        self.sessions.get(&handle).unwrap().scope
    }

    #[cfg(test)]
    pub(super) fn session_output_key(&self, handle: ArticleFeedHandle) -> OutputKey {
        self.sessions.get(&handle).unwrap().output.key()
    }

    #[cfg(test)]
    pub(super) fn assert_internal_oracle(&self) {
        self.graph.assert_incremental_equals_full().unwrap();
    }
}

impl Default for ArticleFeedApp {
    fn default() -> Self {
        Self::new()
    }
}
