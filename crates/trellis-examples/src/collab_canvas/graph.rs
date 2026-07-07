use std::collections::BTreeSet;

use trellis_core::{DependencyList, Graph, InputNode, MaterializedOutput, ResourceKey, ScopeId};

use super::types::{CanvasCommand, CanvasResource, DocumentSession, EditorSnapshot};

pub(super) struct CollabCanvasGraph {
    pub(super) graph: Graph<CanvasCommand>,
    pub(super) primary: DocumentGraph,
    pub(super) secondary: DocumentGraph,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(super) enum DocumentSlot {
    Primary,
    Secondary,
}

pub(super) struct DocumentGraph {
    pub(super) session: InputNode<Option<DocumentSession>>,
    pub(super) scope: ScopeId,
    pub(super) output: MaterializedOutput<EditorSnapshot>,
}

pub(super) fn build_graph() -> CollabCanvasGraph {
    let mut graph = Graph::<CanvasCommand>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let primary = add_document_graph(&mut tx, "primary-document");
    let secondary = add_document_graph(&mut tx, "secondary-document");
    tx.commit().unwrap();
    drop(tx);

    CollabCanvasGraph {
        graph,
        primary,
        secondary,
    }
}

pub(super) fn resource_key(resource: &CanvasResource) -> ResourceKey {
    match resource {
        CanvasResource::SubdocumentRoom(id) => ResourceKey::from_segments(["collab", "subdoc", id]),
        CanvasResource::CommentThread(id) => ResourceKey::from_segments(["collab", "comment", id]),
        CanvasResource::PresenceRoom(id) => ResourceKey::from_segments(["collab", "presence", id]),
        CanvasResource::AttachmentHydration(id) => {
            ResourceKey::from_segments(["collab", "attachment", id])
        }
    }
}

pub(super) fn resource_from_key(key: &ResourceKey) -> Option<CanvasResource> {
    let segments = key.segments().collect::<Vec<_>>();
    match segments.as_slice() {
        ["collab", "subdoc", id] => Some(CanvasResource::SubdocumentRoom((*id).to_owned())),
        ["collab", "comment", id] => Some(CanvasResource::CommentThread((*id).to_owned())),
        ["collab", "presence", id] => Some(CanvasResource::PresenceRoom((*id).to_owned())),
        ["collab", "attachment", id] => Some(CanvasResource::AttachmentHydration((*id).to_owned())),
        _ => None,
    }
}

impl CollabCanvasGraph {
    pub(super) fn document(&self, slot: DocumentSlot) -> &DocumentGraph {
        match slot {
            DocumentSlot::Primary => &self.primary,
            DocumentSlot::Secondary => &self.secondary,
        }
    }
}

fn add_document_graph(
    tx: &mut trellis_core::Transaction<'_, CanvasCommand>,
    name: &str,
) -> DocumentGraph {
    let scope = tx.create_scope(name).unwrap();
    let session = tx
        .input::<Option<DocumentSession>>(format!("{name}-session"))
        .unwrap();
    tx.set_input(session, None).unwrap();

    let resources = tx
        .set_collection(
            format!("{name}-resource-demand"),
            DependencyList::new([session.id()]).unwrap(),
            move |ctx| Ok(resource_demand(ctx.input(session)?)),
        )
        .unwrap();

    tx.open_close_planner(resources, scope, resource_key, |resource| {
        CanvasCommand::Open(resource.clone())
    })
    .unwrap();

    let output = tx
        .materialized_output(
            format!("{name}-editor-output"),
            scope,
            DependencyList::new([session.id(), resources.id()]).unwrap(),
            move |ctx| {
                Ok(editor_snapshot(
                    ctx.input(session)?,
                    ctx.set_collection(resources)?,
                ))
            },
        )
        .unwrap();

    DocumentGraph {
        session,
        scope,
        output,
    }
}

fn resource_demand(session: &Option<DocumentSession>) -> BTreeSet<CanvasResource> {
    let Some(session) = session else {
        return BTreeSet::new();
    };
    let mut demand = BTreeSet::new();
    demand.extend(
        session
            .manifest
            .subdocuments
            .iter()
            .cloned()
            .map(CanvasResource::SubdocumentRoom),
    );
    demand.extend(
        session
            .manifest
            .comment_threads
            .iter()
            .cloned()
            .map(CanvasResource::CommentThread),
    );
    demand.extend(
        session
            .manifest
            .presence_rooms
            .iter()
            .cloned()
            .map(CanvasResource::PresenceRoom),
    );
    demand.extend(
        session
            .manifest
            .attachments
            .intersection(&session.visible_attachments)
            .cloned()
            .map(CanvasResource::AttachmentHydration),
    );
    demand
}

fn editor_snapshot(
    session: &Option<DocumentSession>,
    resources: &BTreeSet<CanvasResource>,
) -> EditorSnapshot {
    let mut snapshot = EditorSnapshot {
        document_id: session.as_ref().map(|session| session.document_id.clone()),
        ..EditorSnapshot::default()
    };
    for resource in resources {
        match resource {
            CanvasResource::SubdocumentRoom(id) => {
                snapshot.subdocuments.insert(id.clone());
            }
            CanvasResource::CommentThread(id) => {
                snapshot.comment_threads.insert(id.clone());
            }
            CanvasResource::PresenceRoom(id) => {
                snapshot.presence_rooms.insert(id.clone());
            }
            CanvasResource::AttachmentHydration(id) => {
                snapshot.hydrated_attachments.insert(id.clone());
            }
        }
    }
    snapshot
}
