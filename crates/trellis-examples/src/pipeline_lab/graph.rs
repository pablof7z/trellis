use trellis_core::{DependencyList, Graph, InputNode, MaterializedOutput, ResourceKey, ScopeId};

use super::selectors::{
    affected_downstream_nodes, authorized_sources, compute_resources, connection_resources,
    pipeline_snapshot, preview_resources, required_sources, selected_node_set,
};
use super::types::{
    CredentialStore, PipelineCommand, PipelineGraphSpec, PipelineJobStatus, PipelineResource,
    PipelineSession, PipelineSnapshot,
};
use std::collections::BTreeMap;

pub(super) struct PipelineGraph {
    pub(super) graph: Graph<PipelineCommand>,
    pub(super) session: InputNode<Option<PipelineSession>>,
    pub(super) pipeline: InputNode<PipelineGraphSpec>,
    pub(super) credentials: InputNode<CredentialStore>,
    pub(super) statuses: InputNode<BTreeMap<String, PipelineJobStatus>>,
    pub(super) workspace_scope: ScopeId,
    pub(super) output: MaterializedOutput<PipelineSnapshot>,
}

pub(super) fn build_graph(
    pipeline: PipelineGraphSpec,
    credentials: CredentialStore,
) -> PipelineGraph {
    let mut graph = Graph::<PipelineCommand>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let workspace_scope = tx.create_scope("pipeline-workspace").unwrap();
    let session = tx
        .input::<Option<PipelineSession>>("pipeline-session")
        .unwrap();
    let pipeline_input = tx.input::<PipelineGraphSpec>("pipeline-graph").unwrap();
    let credentials_input = tx.input::<CredentialStore>("pipeline-credentials").unwrap();
    let statuses = tx
        .input::<BTreeMap<String, PipelineJobStatus>>("pipeline-job-statuses")
        .unwrap();
    tx.set_input(session, None).unwrap();
    tx.set_input(pipeline_input, pipeline).unwrap();
    tx.set_input(credentials_input, credentials).unwrap();
    tx.set_input(statuses, BTreeMap::new()).unwrap();

    let selected = tx
        .set_collection(
            "pipeline-selected-nodes",
            DependencyList::new([session.id(), pipeline_input.id()]).unwrap(),
            move |ctx| {
                Ok(selected_node_set(
                    ctx.input(session)?,
                    ctx.input(pipeline_input)?,
                ))
            },
        )
        .unwrap();

    let affected = tx
        .set_collection(
            "pipeline-affected-downstream-nodes",
            DependencyList::new([pipeline_input.id(), selected.id()]).unwrap(),
            move |ctx| {
                Ok(affected_downstream_nodes(
                    ctx.input(pipeline_input)?,
                    ctx.set_collection(selected)?,
                ))
            },
        )
        .unwrap();

    let required = tx
        .set_collection(
            "pipeline-required-sources",
            DependencyList::new([pipeline_input.id(), affected.id()]).unwrap(),
            move |ctx| {
                Ok(required_sources(
                    ctx.input(pipeline_input)?,
                    ctx.set_collection(affected)?,
                ))
            },
        )
        .unwrap();

    let authorized = tx
        .set_collection(
            "pipeline-authorized-sources",
            DependencyList::new([session.id(), credentials_input.id(), required.id()]).unwrap(),
            move |ctx| {
                Ok(authorized_sources(
                    ctx.input(session)?,
                    ctx.input(credentials_input)?,
                    ctx.set_collection(required)?,
                ))
            },
        )
        .unwrap();

    let connections = tx
        .set_collection(
            "pipeline-source-connections",
            DependencyList::new([authorized.id()]).unwrap(),
            move |ctx| Ok(connection_resources(ctx.set_collection(authorized)?)),
        )
        .unwrap();

    let previews = tx
        .set_collection(
            "pipeline-preview-queries",
            DependencyList::new([pipeline_input.id(), selected.id(), authorized.id()]).unwrap(),
            move |ctx| {
                Ok(preview_resources(
                    ctx.input(pipeline_input)?,
                    ctx.set_collection(selected)?,
                    ctx.set_collection(authorized)?,
                ))
            },
        )
        .unwrap();

    let compute = tx
        .set_collection(
            "pipeline-compute-jobs",
            DependencyList::new([pipeline_input.id(), affected.id(), authorized.id()]).unwrap(),
            move |ctx| {
                Ok(compute_resources(
                    ctx.input(pipeline_input)?,
                    ctx.set_collection(affected)?,
                    ctx.set_collection(authorized)?,
                ))
            },
        )
        .unwrap();

    for collection in [connections, previews, compute] {
        tx.open_close_planner(collection, workspace_scope, resource_key, |resource| {
            PipelineCommand::Open(resource.clone())
        })
        .unwrap();
    }

    let output = tx
        .materialized_output(
            "pipeline-preview-output",
            workspace_scope,
            DependencyList::new([
                session.id(),
                pipeline_input.id(),
                selected.id(),
                authorized.id(),
                statuses.id(),
            ])
            .unwrap(),
            move |ctx| {
                Ok(pipeline_snapshot(
                    ctx.input(session)?,
                    ctx.input(pipeline_input)?,
                    ctx.set_collection(selected)?,
                    ctx.set_collection(authorized)?,
                    ctx.input(statuses)?,
                ))
            },
        )
        .unwrap();

    tx.commit().unwrap();
    drop(tx);

    PipelineGraph {
        graph,
        session,
        pipeline: pipeline_input,
        credentials: credentials_input,
        statuses,
        workspace_scope,
        output,
    }
}

pub(super) fn resource_key(resource: &PipelineResource) -> ResourceKey {
    match resource {
        PipelineResource::SourceConnection { source_id } => {
            ResourceKey::from_segments(["pipeline", "source", source_id.as_str()])
        }
        PipelineResource::PreviewQuery {
            node_id,
            lineage_key,
        } => ResourceKey::from_segments([
            "pipeline",
            "preview",
            node_id.as_str(),
            lineage_key.as_str(),
        ]),
        PipelineResource::ComputeJob {
            node_id,
            lineage_key,
        } => ResourceKey::from_segments([
            "pipeline",
            "compute",
            node_id.as_str(),
            lineage_key.as_str(),
        ]),
    }
}

pub(super) fn resource_from_key(key: &ResourceKey) -> Option<PipelineResource> {
    let segments = key.segments().collect::<Vec<_>>();
    match segments.as_slice() {
        ["pipeline", "source", source_id] => Some(PipelineResource::SourceConnection {
            source_id: (*source_id).to_owned(),
        }),
        ["pipeline", "preview", node_id, lineage_key] => Some(PipelineResource::PreviewQuery {
            node_id: (*node_id).to_owned(),
            lineage_key: (*lineage_key).to_owned(),
        }),
        ["pipeline", "compute", node_id, lineage_key] => Some(PipelineResource::ComputeJob {
            node_id: (*node_id).to_owned(),
            lineage_key: (*lineage_key).to_owned(),
        }),
        _ => None,
    }
}
