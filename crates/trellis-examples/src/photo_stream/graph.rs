use std::collections::BTreeSet;

use trellis_core::{DependencyList, Graph, InputNode, ResourceKey, ScopeId};

use super::types::{
    PhotoAsset, PhotoCatalog, PhotoCommand, PhotoGridSnapshot, PhotoResource, PhotoTile,
    SmartAlbumRule, SmartAlbumSession, StoragePolicy,
};

pub(super) struct PhotoStreamGraph {
    pub(super) graph: Graph<PhotoCommand>,
    pub(super) session: InputNode<Option<SmartAlbumSession>>,
    pub(super) catalog: InputNode<PhotoCatalog>,
    pub(super) album_scope: ScopeId,
}

pub(super) fn build_graph(catalog: PhotoCatalog) -> PhotoStreamGraph {
    let mut graph = Graph::<PhotoCommand>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let album_scope = tx.create_scope("photo-smart-album").unwrap();
    let session = tx
        .input::<Option<SmartAlbumSession>>("photo-album-session")
        .unwrap();
    let catalog_input = tx.input::<PhotoCatalog>("photo-catalog").unwrap();
    tx.set_input(session, None).unwrap();
    tx.set_input(catalog_input, catalog).unwrap();

    let matches = tx
        .set_collection(
            "photo-matching-assets",
            DependencyList::new([session.id(), catalog_input.id()]).unwrap(),
            move |ctx| {
                Ok(matching_assets(
                    ctx.input(session)?,
                    ctx.input(catalog_input)?,
                ))
            },
        )
        .unwrap();

    let visible = tx
        .set_collection(
            "photo-visible-assets",
            DependencyList::new([session.id(), matches.id()]).unwrap(),
            move |ctx| {
                Ok(visible_assets(
                    ctx.input(session)?,
                    ctx.set_collection(matches)?,
                ))
            },
        )
        .unwrap();

    let resources = tx
        .set_collection(
            "photo-visible-resources",
            DependencyList::new([session.id(), catalog_input.id(), visible.id()]).unwrap(),
            move |ctx| {
                Ok(resource_demand(
                    ctx.input(session)?,
                    ctx.input(catalog_input)?,
                    ctx.set_collection(visible)?,
                ))
            },
        )
        .unwrap();

    tx.open_close_planner(resources, album_scope, resource_key, |resource| {
        PhotoCommand::Open(resource.clone())
    })
    .unwrap();

    tx.materialized_output(
        "photo-grid-output",
        album_scope,
        DependencyList::new([session.id(), catalog_input.id(), matches.id(), visible.id()])
            .unwrap(),
        move |ctx| {
            Ok(grid_snapshot(
                ctx.input(session)?,
                ctx.input(catalog_input)?,
                ctx.set_collection(matches)?,
                ctx.set_collection(visible)?,
            ))
        },
    )
    .unwrap();

    tx.commit().unwrap();
    drop(tx);

    PhotoStreamGraph {
        graph,
        session,
        catalog: catalog_input,
        album_scope,
    }
}

pub(super) fn resource_key(resource: &PhotoResource) -> ResourceKey {
    match resource {
        PhotoResource::ThumbnailJob(asset_id) => {
            ResourceKey::from_segments(["photo", "thumb", asset_id.as_str()])
        }
        PhotoResource::MetadataHydration(asset_id) => {
            ResourceKey::from_segments(["photo", "metadata", asset_id.as_str()])
        }
        PhotoResource::CloudDownload(asset_id) => {
            ResourceKey::from_segments(["photo", "cloud", asset_id.as_str()])
        }
        PhotoResource::HighResPreview(asset_id) => {
            ResourceKey::from_segments(["photo", "highres", asset_id.as_str()])
        }
    }
}

pub(super) fn resource_from_key(key: &ResourceKey) -> Option<PhotoResource> {
    let segments = key.segments().collect::<Vec<_>>();
    match segments.as_slice() {
        ["photo", "thumb", asset_id] => Some(PhotoResource::ThumbnailJob((*asset_id).to_owned())),
        ["photo", "metadata", asset_id] => {
            Some(PhotoResource::MetadataHydration((*asset_id).to_owned()))
        }
        ["photo", "cloud", asset_id] => Some(PhotoResource::CloudDownload((*asset_id).to_owned())),
        ["photo", "highres", asset_id] => {
            Some(PhotoResource::HighResPreview((*asset_id).to_owned()))
        }
        _ => None,
    }
}

fn matching_assets(
    session: &Option<SmartAlbumSession>,
    catalog: &PhotoCatalog,
) -> BTreeSet<String> {
    let Some(session) = session else {
        return BTreeSet::new();
    };
    catalog
        .assets
        .values()
        .filter(|asset| rule_matches(&session.rule, asset))
        .map(|asset| asset.id.clone())
        .collect()
}

fn visible_assets(
    session: &Option<SmartAlbumSession>,
    matches: &BTreeSet<String>,
) -> BTreeSet<String> {
    let Some(session) = session else {
        return BTreeSet::new();
    };
    matches
        .iter()
        .skip(session.viewport.start)
        .take(session.viewport.len)
        .cloned()
        .collect()
}

fn resource_demand(
    session: &Option<SmartAlbumSession>,
    catalog: &PhotoCatalog,
    visible: &BTreeSet<String>,
) -> BTreeSet<PhotoResource> {
    let Some(session) = session else {
        return BTreeSet::new();
    };
    let mut demand = BTreeSet::new();
    for asset_id in visible {
        demand.insert(PhotoResource::ThumbnailJob(asset_id.clone()));
        demand.insert(PhotoResource::MetadataHydration(asset_id.clone()));
        if session.storage_policy == StoragePolicy::Normal {
            demand.insert(PhotoResource::HighResPreview(asset_id.clone()));
            if catalog
                .assets
                .get(asset_id)
                .is_some_and(|asset| asset.cloud_available && !asset.local_original)
            {
                demand.insert(PhotoResource::CloudDownload(asset_id.clone()));
            }
        }
    }
    demand
}

fn grid_snapshot(
    session: &Option<SmartAlbumSession>,
    catalog: &PhotoCatalog,
    matches: &BTreeSet<String>,
    visible: &BTreeSet<String>,
) -> PhotoGridSnapshot {
    let tiles = visible
        .iter()
        .filter_map(|asset_id| catalog.assets.get(asset_id))
        .map(|asset| PhotoTile {
            asset_id: asset.id.clone(),
            media_kind: asset.media_kind,
            local_original: asset.local_original,
        })
        .collect();
    PhotoGridSnapshot {
        rule: session.as_ref().map(|session| session.rule.clone()),
        total_matches: matches.len(),
        tiles,
    }
}

fn rule_matches(rule: &SmartAlbumRule, asset: &PhotoAsset) -> bool {
    match rule {
        SmartAlbumRule::Favorites => asset.favorite,
        SmartAlbumRule::Person(person) => asset.people.contains(person),
        SmartAlbumRule::MediaKind(kind) => asset.media_kind == *kind,
        SmartAlbumRule::AllAssets => true,
    }
}
