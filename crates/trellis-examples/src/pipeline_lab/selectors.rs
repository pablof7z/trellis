use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::types::{
    CredentialStore, PipelineGraphNodeView, PipelineGraphSpec, PipelineJobStatus, PipelineNode,
    PipelineNodeKind, PipelineResource, PipelineSession, PipelineSnapshot, PreviewPanel,
    PreviewRow,
};

pub(super) fn selected_node_set(
    session: &Option<PipelineSession>,
    graph: &PipelineGraphSpec,
) -> BTreeSet<String> {
    let Some(session) = session else {
        return BTreeSet::new();
    };
    session
        .selected_nodes
        .iter()
        .filter(|node_id| !session.hidden_nodes.contains(*node_id))
        .filter(|node_id| graph.nodes.contains_key(*node_id))
        .cloned()
        .collect()
}

pub(super) fn affected_downstream_nodes(
    graph: &PipelineGraphSpec,
    selected: &BTreeSet<String>,
) -> BTreeSet<String> {
    downstream_closure(graph, selected)
}

pub(super) fn required_sources(
    graph: &PipelineGraphSpec,
    affected: &BTreeSet<String>,
) -> BTreeSet<String> {
    affected
        .iter()
        .flat_map(|node_id| sources_for_node(graph, node_id))
        .collect()
}

pub(super) fn authorized_sources(
    session: &Option<PipelineSession>,
    credentials: &CredentialStore,
    required: &BTreeSet<String>,
) -> BTreeSet<String> {
    let Some(session) = session else {
        return BTreeSet::new();
    };
    let Some(allowed) = credentials.sources_by_user.get(&session.user) else {
        return BTreeSet::new();
    };
    required
        .iter()
        .filter(|source_id| allowed.contains(*source_id))
        .cloned()
        .collect()
}

pub(super) fn connection_resources(authorized: &BTreeSet<String>) -> BTreeSet<PipelineResource> {
    authorized
        .iter()
        .map(|source_id| PipelineResource::SourceConnection {
            source_id: source_id.clone(),
        })
        .collect()
}

pub(super) fn preview_resources(
    graph: &PipelineGraphSpec,
    selected: &BTreeSet<String>,
    authorized: &BTreeSet<String>,
) -> BTreeSet<PipelineResource> {
    selected
        .iter()
        .filter(|node_id| node_authorized(graph, node_id, authorized))
        .filter_map(|node_id| {
            Some(PipelineResource::PreviewQuery {
                node_id: node_id.clone(),
                lineage_key: lineage_key(graph, node_id)?,
            })
        })
        .collect()
}

pub(super) fn compute_resources(
    graph: &PipelineGraphSpec,
    affected: &BTreeSet<String>,
    authorized: &BTreeSet<String>,
) -> BTreeSet<PipelineResource> {
    affected
        .iter()
        .filter(|node_id| node_authorized(graph, node_id, authorized))
        .filter_map(|node_id| {
            let node = graph.nodes.get(node_id)?;
            matches!(node.kind, PipelineNodeKind::Transform { .. }).then_some(())?;
            Some(PipelineResource::ComputeJob {
                node_id: node_id.clone(),
                lineage_key: lineage_key(graph, node_id)?,
            })
        })
        .collect()
}

pub(super) fn pipeline_snapshot(
    session: &Option<PipelineSession>,
    graph: &PipelineGraphSpec,
    selected: &BTreeSet<String>,
    authorized: &BTreeSet<String>,
    statuses: &BTreeMap<String, PipelineJobStatus>,
) -> PipelineSnapshot {
    let graph_nodes = graph
        .nodes
        .values()
        .map(|node| node_view(session, graph, node, authorized))
        .collect();
    let panels = selected
        .iter()
        .filter(|node_id| node_authorized(graph, node_id, authorized))
        .filter_map(|node_id| preview_panel(graph, node_id, statuses))
        .collect();
    PipelineSnapshot {
        graph_nodes,
        panels,
    }
}

pub(super) fn downstream_closure(
    graph: &PipelineGraphSpec,
    roots: &BTreeSet<String>,
) -> BTreeSet<String> {
    let mut result = roots
        .iter()
        .filter(|node_id| graph.nodes.contains_key(*node_id))
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut queue = result.iter().cloned().collect::<VecDeque<_>>();
    while let Some(current) = queue.pop_front() {
        for node in graph.nodes.values() {
            if node.upstream.contains(&current) && result.insert(node.id.clone()) {
                queue.push_back(node.id.clone());
            }
        }
    }
    result
}

fn node_view(
    session: &Option<PipelineSession>,
    graph: &PipelineGraphSpec,
    node: &PipelineNode,
    authorized: &BTreeSet<String>,
) -> PipelineGraphNodeView {
    let selected = session
        .as_ref()
        .is_some_and(|session| session.selected_nodes.contains(&node.id));
    let hidden = session
        .as_ref()
        .is_some_and(|session| session.hidden_nodes.contains(&node.id));
    PipelineGraphNodeView {
        node_id: node.id.clone(),
        label: node.label.clone(),
        kind: match &node.kind {
            PipelineNodeKind::Source { .. } => "source".to_owned(),
            PipelineNodeKind::Transform { .. } => "transform".to_owned(),
        },
        selected,
        hidden,
        authorized: node_authorized(graph, &node.id, authorized),
    }
}

fn preview_panel(
    graph: &PipelineGraphSpec,
    node_id: &str,
    statuses: &BTreeMap<String, PipelineJobStatus>,
) -> Option<PreviewPanel> {
    let node = graph.nodes.get(node_id)?;
    let status = statuses.get(node_id).cloned().unwrap_or_default();
    Some(PreviewPanel {
        node_id: node.id.clone(),
        label: node.label.clone(),
        lineage_key: lineage_key(graph, node_id)?,
        rows: preview_rows(node, &status),
        status,
    })
}

fn preview_rows(node: &PipelineNode, status: &PipelineJobStatus) -> Vec<PreviewRow> {
    if matches!(status, PipelineJobStatus::Failed(_)) {
        return Vec::new();
    }
    let mut cells = BTreeMap::new();
    cells.insert("node".to_owned(), node.id.clone());
    cells.insert("label".to_owned(), node.label.clone());
    match &node.kind {
        PipelineNodeKind::Source { source_id } => {
            cells.insert("source".to_owned(), source_id.clone());
        }
        PipelineNodeKind::Transform {
            expression,
            revision,
        } => {
            cells.insert("expression".to_owned(), expression.clone());
            cells.insert("revision".to_owned(), revision.to_string());
        }
    }
    vec![PreviewRow { cells }]
}

fn node_authorized(
    graph: &PipelineGraphSpec,
    node_id: &str,
    authorized: &BTreeSet<String>,
) -> bool {
    let sources = sources_for_node(graph, node_id);
    !sources.is_empty()
        && sources
            .iter()
            .all(|source_id| authorized.contains(source_id))
}

fn sources_for_node(graph: &PipelineGraphSpec, node_id: &str) -> BTreeSet<String> {
    let mut sources = BTreeSet::new();
    collect_sources(graph, node_id, &mut BTreeSet::new(), &mut sources);
    sources
}

fn collect_sources(
    graph: &PipelineGraphSpec,
    node_id: &str,
    visiting: &mut BTreeSet<String>,
    sources: &mut BTreeSet<String>,
) {
    if !visiting.insert(node_id.to_owned()) {
        return;
    }
    let Some(node) = graph.nodes.get(node_id) else {
        return;
    };
    match &node.kind {
        PipelineNodeKind::Source { source_id } => {
            sources.insert(source_id.clone());
        }
        PipelineNodeKind::Transform { .. } => {
            for upstream in &node.upstream {
                collect_sources(graph, upstream, visiting, sources);
            }
        }
    }
    visiting.remove(node_id);
}

fn lineage_key(graph: &PipelineGraphSpec, node_id: &str) -> Option<String> {
    lineage_key_inner(graph, node_id, &mut BTreeSet::new())
}

fn lineage_key_inner(
    graph: &PipelineGraphSpec,
    node_id: &str,
    visiting: &mut BTreeSet<String>,
) -> Option<String> {
    if !visiting.insert(node_id.to_owned()) {
        return None;
    }
    let node = graph.nodes.get(node_id)?;
    let key = match &node.kind {
        PipelineNodeKind::Source { source_id } => format!("source:{source_id}"),
        PipelineNodeKind::Transform { revision, .. } => {
            let upstream = node
                .upstream
                .iter()
                .filter_map(|upstream| lineage_key_inner(graph, upstream, visiting))
                .collect::<Vec<_>>()
                .join("+");
            format!("transform:{}:r{revision}:{upstream}", node.id)
        }
    };
    visiting.remove(node_id);
    Some(key)
}
