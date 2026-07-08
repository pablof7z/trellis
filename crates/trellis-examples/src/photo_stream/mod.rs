//! PhotoStream smart-album hydrator secondary showcase.

mod bug_capsules;
mod engine;
mod graph;
mod sample;
mod scripts;
mod types;

#[cfg(test)]
mod tests;

pub use bug_capsules::{available_bug_capsules, run_all_bug_capsules, run_bug_capsule};
pub use engine::PhotoStreamApp;
pub use sample::{ids, opening_album, sample_catalog};
pub use scripts::smart_album_lifecycle_showcase_trace;
pub use types::{
    AssetViewport, MediaKind, PhotoAlbumHandle, PhotoAsset, PhotoCatalog, PhotoEffect, PhotoFrame,
    PhotoGridSnapshot, PhotoResource, PhotoStreamEvent, PhotoStreamUpdate, PhotoTile,
    SmartAlbumRule, SmartAlbumSession, StoragePolicy,
};
