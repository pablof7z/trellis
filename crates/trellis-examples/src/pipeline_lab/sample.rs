use std::collections::{BTreeMap, BTreeSet};

use super::types::{
    CredentialStore, PipelineGraphSpec, PipelineNode, PipelineNodeKind, PipelineSession,
};

/// Builds a sorted string set from literal values.
pub fn ids<const N: usize>(values: [&str; N]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

/// Opening pipeline session used by the script.
pub fn opening_pipeline() -> PipelineSession {
    PipelineSession {
        user: "analyst".to_owned(),
        selected_nodes: ids(["clean_orders", "daily_revenue"]),
        hidden_nodes: BTreeSet::new(),
    }
}

/// Sample credentials for the PipelineLab showcase.
pub fn sample_credentials() -> CredentialStore {
    CredentialStore {
        sources_by_user: [("analyst".to_owned(), ids(["warehouse", "crm"]))]
            .into_iter()
            .collect(),
    }
}

/// Sample pipeline graph for the PipelineLab showcase.
pub fn sample_pipeline() -> PipelineGraphSpec {
    let mut nodes = BTreeMap::new();
    for node in [
        source("orders", "Orders source", "warehouse"),
        source("accounts", "Accounts source", "crm"),
        transform(
            "clean_orders",
            "Clean orders",
            ["orders"],
            "filter status = paid",
            1,
        ),
        transform(
            "daily_revenue",
            "Daily revenue",
            ["clean_orders"],
            "group by day sum(total)",
            1,
        ),
        transform(
            "joined_revenue",
            "Joined revenue",
            ["daily_revenue", "accounts"],
            "join account tier",
            1,
        ),
    ] {
        nodes.insert(node.id.clone(), node);
    }
    PipelineGraphSpec { nodes }
}

fn source(id: &str, label: &str, source_id: &str) -> PipelineNode {
    PipelineNode {
        id: id.to_owned(),
        label: label.to_owned(),
        kind: PipelineNodeKind::Source {
            source_id: source_id.to_owned(),
        },
        upstream: BTreeSet::new(),
    }
}

fn transform<const N: usize>(
    id: &str,
    label: &str,
    upstream: [&str; N],
    expression: &str,
    revision: u64,
) -> PipelineNode {
    PipelineNode {
        id: id.to_owned(),
        label: label.to_owned(),
        kind: PipelineNodeKind::Transform {
            expression: expression.to_owned(),
            revision,
        },
        upstream: ids(upstream),
    }
}
