use std::collections::{BTreeMap, BTreeSet};

use super::types::{
    AssetViewport, MediaKind, PhotoAsset, PhotoCatalog, SmartAlbumRule, SmartAlbumSession,
    StoragePolicy,
};

/// Builds a sorted string set from literal values.
pub fn ids<const N: usize>(values: [&str; N]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

/// Opening smart album state used by the script.
pub fn opening_album() -> SmartAlbumSession {
    SmartAlbumSession {
        rule: SmartAlbumRule::Favorites,
        viewport: AssetViewport { start: 0, len: 4 },
        storage_policy: StoragePolicy::Normal,
    }
}

/// Sample catalog for the PhotoStream showcase.
pub fn sample_catalog() -> PhotoCatalog {
    let mut assets = BTreeMap::new();
    insert(&mut assets, asset("asset-001", true, ["Ava"], true, true));
    insert(&mut assets, asset("asset-002", true, ["Noah"], true, false));
    insert(&mut assets, asset("asset-003", false, ["Ava"], false, true));
    insert(&mut assets, asset("asset-004", true, ["Mia"], false, true));
    insert(
        &mut assets,
        asset("asset-005", false, ["Noah"], true, false),
    );
    insert(&mut assets, asset("asset-006", true, ["Ava"], true, false));
    insert(&mut assets, asset("asset-007", false, ["Mia"], false, true));
    insert(&mut assets, asset("asset-008", true, ["Noah"], true, true));
    insert(&mut assets, asset("asset-009", false, ["Ava"], true, false));
    insert(&mut assets, video("asset-010", false, ["Ava"], true, false));

    for index in 11..171 {
        let id = format!("asset-{index:03}");
        let favorite = index % 3 == 0;
        let cloud = index % 4 == 0;
        let local = index % 5 != 0;
        let people = if index % 2 == 0 { ["Ava"] } else { ["Noah"] };
        insert(&mut assets, asset(&id, favorite, people, cloud, local));
    }

    PhotoCatalog { assets }
}

fn insert(assets: &mut BTreeMap<String, PhotoAsset>, asset: PhotoAsset) {
    assets.insert(asset.id.clone(), asset);
}

fn asset<const N: usize>(
    id: &str,
    favorite: bool,
    people: [&str; N],
    cloud_available: bool,
    local_original: bool,
) -> PhotoAsset {
    PhotoAsset {
        id: id.to_owned(),
        media_kind: MediaKind::Photo,
        favorite,
        people: ids(people),
        cloud_available,
        local_original,
    }
}

fn video<const N: usize>(
    id: &str,
    favorite: bool,
    people: [&str; N],
    cloud_available: bool,
    local_original: bool,
) -> PhotoAsset {
    PhotoAsset {
        media_kind: MediaKind::Video,
        ..asset(id, favorite, people, cloud_available, local_original)
    }
}
