use super::shape::resource_key;
use super::types::FeedSnapshot;
use super::*;
use trellis_core::{OutputFrameKind, ResourceCommand};
use trellis_testing::{OutputLedger, ResourceLedger, Scenario};

const ACCOUNT: &str = "acct";
const ROUTE: &str = "home";

fn params() -> ArticleFeedParams {
    ArticleFeedParams::new(ACCOUNT, ROUTE, 100)
}

fn target(source: &str) -> SubscriptionTarget {
    SubscriptionTarget {
        account: ACCOUNT.to_owned(),
        route: ROUTE.to_owned(),
        source: source.to_owned(),
    }
}

fn subscription(source: &str, replay_epoch: u64) -> LiveSubscription {
    LiveSubscription {
        target: target(source),
        limit: 100,
        replay_epoch,
    }
}

fn row(source: &str, id: &str) -> ArticleRow {
    ArticleRow::new(source, id, format!("{source}-{id}"))
}

fn seeded_app(sources: &[&str]) -> ArticleFeedApp {
    let mut app = ArticleFeedApp::new();
    app.set_route_sources(ACCOUNT, ROUTE, sources.iter().copied());
    for source in sources {
        app.replace_source_rows(source, vec![row(source, "1")]);
    }
    assert!(app.drain_subscription_effects().is_empty());
    app
}

fn run_script() -> Scenario {
    let mut app = seeded_app(&["a", "b"]);
    let handle = app.open_article_feed(params());
    let mut scenario = Scenario::new();
    scenario.record("open", app.last_result()).unwrap();
    app.set_route_sources(ACCOUNT, ROUTE, ["a"]);
    scenario.record("shrink", app.last_result()).unwrap();
    app.request_replay(handle);
    scenario.record("replay", app.last_result()).unwrap();
    app.close(handle);
    scenario.record("close", app.last_result()).unwrap();
    scenario
}

#[test]
fn handle_close_tears_down_scoped_resources() {
    let mut app = seeded_app(&["a"]);
    let handle = app.open_article_feed(params());
    assert_eq!(
        app.drain_subscription_effects(),
        vec![SubscriptionEffect::Open(subscription("a", 0))]
    );
    assert_eq!(
        app.poll_output(handle),
        vec![ArticleFeedFrame::Baseline(vec![row("a", "1")])]
    );

    app.close(handle);

    assert_eq!(
        app.drain_subscription_effects(),
        vec![SubscriptionEffect::Close(target("a"))]
    );
    assert_eq!(app.poll_output(handle), vec![ArticleFeedFrame::Cleared]);
    assert!(app.last_result().resource_plan.commands().iter().any(|command| {
        matches!(command, ResourceCommand::Close { key, .. } if key == &resource_key(&target("a")))
    }));
    app.assert_internal_oracle();
}

#[test]
fn source_shrink_withdraws_demand_and_admitted_rows() {
    let mut app = seeded_app(&["a", "b"]);
    let handle = app.open_article_feed(params());
    app.drain_subscription_effects();
    app.poll_output(handle);

    app.set_route_sources(ACCOUNT, ROUTE, ["a"]);

    assert_eq!(
        app.drain_subscription_effects(),
        vec![SubscriptionEffect::Close(target("b"))]
    );
    assert_eq!(
        app.poll_output(handle),
        vec![ArticleFeedFrame::Delta(vec![row("a", "1")])]
    );
    assert!(app.last_result().resource_plan.commands().iter().any(|command| {
        matches!(command, ResourceCommand::Close { key, .. } if key == &resource_key(&target("b")))
    }));
    app.assert_internal_oracle();
}

#[test]
fn empty_source_opens_no_broad_demand() {
    let mut app = ArticleFeedApp::new();
    let handle = app.open_article_feed(params());

    assert!(app.drain_subscription_effects().is_empty());
    assert_eq!(
        app.poll_output(handle),
        vec![ArticleFeedFrame::Baseline(Vec::new())]
    );
    assert!(app.last_result().resource_plan.commands().is_empty());
    app.assert_internal_oracle();
}

#[test]
fn resource_keys_preserve_subscription_target_segments_with_slashes() {
    let account = "acct/with/slash";
    let route = "home/main";
    let source = "relay/wss://example";
    let expected_target = SubscriptionTarget {
        account: account.to_owned(),
        route: route.to_owned(),
        source: source.to_owned(),
    };
    let key = resource_key(&expected_target);
    assert_eq!(
        key.segments().collect::<Vec<_>>(),
        vec!["article-feed", account, route, source]
    );

    let mut app = ArticleFeedApp::new();
    app.set_route_sources(account, route, [source]);
    app.replace_source_rows(source, vec![row(source, "1")]);
    let handle = app.open_article_feed(ArticleFeedParams::new(account, route, 100));
    assert_eq!(
        app.drain_subscription_effects(),
        vec![SubscriptionEffect::Open(LiveSubscription {
            target: expected_target.clone(),
            limit: 100,
            replay_epoch: 0,
        })]
    );

    app.close(handle);
    assert_eq!(
        app.drain_subscription_effects(),
        vec![SubscriptionEffect::Close(expected_target)]
    );
}

#[test]
fn replay_and_baseline_frames_are_coherent() {
    let mut app = seeded_app(&["a"]);
    let handle = app.open_article_feed(params());
    assert_eq!(
        app.poll_output(handle),
        vec![ArticleFeedFrame::Baseline(vec![row("a", "1")])]
    );
    app.drain_subscription_effects();

    app.request_replay(handle);

    assert_eq!(
        app.drain_subscription_effects(),
        vec![SubscriptionEffect::Replace(subscription("a", 1))]
    );
    assert_eq!(
        app.poll_output(handle),
        vec![ArticleFeedFrame::Replay(vec![row("a", "1")])]
    );
    assert!(matches!(
        &app.last_result().output_frames[0].kind,
        OutputFrameKind::Rebaseline(snapshot, _)
            if snapshot
                .get::<FeedSnapshot>()
                .is_some_and(|snapshot| snapshot.replay_epoch == 1
                    && snapshot.rows == vec![row("a", "1")])
    ));
    app.assert_internal_oracle();
}

#[test]
fn trellis_test_asserts_lifecycle_output_and_replay_invariants() {
    let mut app = seeded_app(&["a", "b"]);
    let handle = app.open_article_feed(params());
    let scope = app.session_scope(handle);
    let output = app.session_output_key(handle);
    let mut resources = ResourceLedger::new();
    let mut outputs = OutputLedger::new();

    let open = app.last_result().clone();
    resources.apply_result(&open);
    resources.assert_all_resources_have_owner().unwrap();
    outputs.apply_result(&open);
    outputs
        .assert_current_equals(
            output,
            &FeedSnapshot {
                rows: vec![row("a", "1"), row("b", "1")],
                replay_epoch: 0,
            },
        )
        .unwrap();

    app.set_route_sources(ACCOUNT, ROUTE, ["a"]);
    let shrink = app.last_result().clone();
    resources.apply_result(&shrink);
    resources
        .assert_resource_not_open(&resource_key(&target("b")))
        .unwrap();
    outputs.apply_result(&shrink);
    outputs
        .assert_current_equals(
            output,
            &FeedSnapshot {
                rows: vec![row("a", "1")],
                replay_epoch: 0,
            },
        )
        .unwrap();

    app.request_replay(handle);
    let replay = app.last_result().clone();
    resources.apply_result(&replay);
    outputs.apply_result(&replay);
    outputs
        .assert_current_equals(
            output,
            &FeedSnapshot {
                rows: vec![row("a", "1")],
                replay_epoch: 1,
            },
        )
        .unwrap();

    app.close(handle);
    let close = app.last_result().clone();
    resources.apply_result(&close);
    resources
        .assert_resource_not_open(&resource_key(&target("a")))
        .unwrap();
    resources.assert_no_duplicate_close().unwrap();
    outputs.close_scope(scope);
    outputs.apply_result(&close);
    outputs.assert_cleared(output).unwrap();
    outputs.assert_revision_monotonic().unwrap();

    run_script().assert_replay_matches(&run_script()).unwrap();
}
