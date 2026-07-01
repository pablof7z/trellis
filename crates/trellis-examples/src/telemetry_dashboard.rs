use std::collections::{BTreeMap, BTreeSet};

use trellis_core::{DependencyList, Graph, ResourceKey, ResourcePlan};

/// Host command payload for topic subscriptions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TopicCommand {
    /// Subscribe to a telemetry topic.
    Subscribe(String),
}

fn key(topic: &str) -> ResourceKey {
    ResourceKey::new(format!("topic:{topic}"))
}

#[cfg(test)]
fn customer_devices(
    entries: &[(&str, &[(&str, &str)])],
) -> BTreeMap<String, BTreeMap<String, String>> {
    entries
        .iter()
        .map(|(customer, devices)| {
            (
                (*customer).to_owned(),
                devices
                    .iter()
                    .map(|(device, topic)| ((*device).to_owned(), (*topic).to_owned()))
                    .collect(),
            )
        })
        .collect()
}

fn cards(topics: &BTreeSet<String>) -> Vec<String> {
    topics.iter().map(|topic| format!("card:{topic}")).collect()
}

/// Built telemetry-dashboard example graph, inputs, and panel scopes.
pub struct TelemetryDashboardExample {
    /// Example graph.
    pub graph: Graph<TopicCommand, Vec<String>>,
    /// Selected customer canonical input.
    pub selected_customer: trellis_core::InputNode<Option<String>>,
    /// Device index canonical input.
    pub device_index: trellis_core::InputNode<BTreeMap<String, BTreeMap<String, String>>>,
    /// Left panel scope.
    pub left_panel: trellis_core::ScopeId,
    /// Right panel scope.
    pub right_panel: trellis_core::ScopeId,
}

/// Builds the telemetry dashboard proof graph.
pub fn build_graph(
    selected: Option<&str>,
    devices: BTreeMap<String, BTreeMap<String, String>>,
) -> TelemetryDashboardExample {
    let mut graph = Graph::<TopicCommand, Vec<String>>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let left_panel = tx.create_scope("left-panel").unwrap();
    let right_panel = tx.create_scope("right-panel").unwrap();
    let selected_customer = tx.input::<Option<String>>("selected-customer").unwrap();
    let device_index = tx
        .input::<BTreeMap<String, BTreeMap<String, String>>>("device-index")
        .unwrap();
    tx.set_input(selected_customer, selected.map(str::to_owned))
        .unwrap();
    tx.set_input(device_index, devices).unwrap();
    let visible_devices = tx
        .derived(
            "visible-devices",
            DependencyList::new([selected_customer.id(), device_index.id()]).unwrap(),
            move |ctx| {
                let selected = ctx.input(selected_customer)?;
                let index = ctx.input(device_index)?;
                Ok(selected
                    .as_ref()
                    .and_then(|customer| index.get(customer))
                    .cloned()
                    .unwrap_or_default())
            },
        )
        .unwrap();
    let topics = tx
        .set_collection(
            "topic-set",
            DependencyList::new([visible_devices.id()]).unwrap(),
            move |ctx| Ok(ctx.derived(visible_devices)?.values().cloned().collect()),
        )
        .unwrap();
    for scope in [left_panel, right_panel] {
        tx.set_resource_planner(topics, scope, move |ctx| {
            let mut plan = ResourcePlan::new();
            for added in &ctx.diff().added {
                plan.open(
                    key(&added.value),
                    ctx.scope(),
                    TopicCommand::Subscribe(added.value.clone()),
                );
            }
            for removed in &ctx.diff().removed {
                plan.close(key(&removed.value), ctx.scope());
            }
            Ok(plan)
        })
        .unwrap();
    }
    tx.materialized_output(
        "cards",
        left_panel,
        DependencyList::new([topics.id()]).unwrap(),
        move |ctx| Ok(cards(ctx.set_collection(topics)?)),
    )
    .unwrap();
    tx.commit().unwrap();
    drop(tx);
    TelemetryDashboardExample {
        graph,
        selected_customer,
        device_index,
        left_panel,
        right_panel,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use trellis_core::{OutputFrameKind, ResourceCommand};

    #[test]
    fn filter_shrink_unsubscribes_removed_topics() {
        let mut example = build_graph(
            Some("acme"),
            customer_devices(&[("acme", &[("d1", "a"), ("d2", "b")])]),
        );

        let mut tx = example.graph.begin_transaction().unwrap();
        tx.set_input(
            example.device_index,
            customer_devices(&[("acme", &[("d1", "a")])]),
        )
        .unwrap();
        let result = tx.commit().unwrap();
        drop(tx);

        assert!(result.resource_plan.commands().iter().any(|command| {
            matches!(command, ResourceCommand::Close { key: resource_key, .. } if resource_key == &key("b"))
        }));
        assert!(matches!(
            &result.output_frames[0].kind,
            OutputFrameKind::Delta(cards) if cards == &vec!["card:a".to_owned()]
        ));

        let mut tx = example.graph.begin_transaction().unwrap();
        tx.set_input(example.selected_customer, None).unwrap();
        let result = tx.commit().unwrap();
        drop(tx);
        assert!(result.resource_plan.commands().iter().any(|command| {
            matches!(command, ResourceCommand::Close { key: resource_key, .. } if resource_key == &key("a"))
        }));
        example.graph.assert_incremental_equals_full().unwrap();
    }

    #[test]
    fn shared_topic_closes_after_last_panel() {
        let mut example = build_graph(
            Some("acme"),
            customer_devices(&[("acme", &[("d1", "shared")])]),
        );

        let mut tx = example.graph.begin_transaction().unwrap();
        tx.close_scope(example.left_panel).unwrap();
        let result = tx.commit().unwrap();
        drop(tx);
        assert!(result.resource_plan.commands().is_empty());
        assert!(example.graph.resource_owners(&key("shared")).is_some());

        let mut tx = example.graph.begin_transaction().unwrap();
        tx.close_scope(example.right_panel).unwrap();
        let result = tx.commit().unwrap();
        drop(tx);

        assert_eq!(result.resource_plan.commands().len(), 1);
        assert!(example.graph.resource_owners(&key("shared")).is_none());
        example.graph.assert_incremental_equals_full().unwrap();
    }
}
