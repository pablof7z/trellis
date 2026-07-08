use std::collections::{BTreeMap, BTreeSet};

/// Opaque handle for an open smart album.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct PhotoAlbumHandle(pub u64);

/// Media kind for an asset in the photo library.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum MediaKind {
    /// Still image asset.
    Photo,
    /// Video asset.
    Video,
}

/// Smart album rule owned by the photo app.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SmartAlbumRule {
    /// Favorite assets only.
    Favorites,
    /// Assets containing one person label.
    Person(String),
    /// Assets of one media kind.
    MediaKind(MediaKind),
    /// All assets in the catalog.
    AllAssets,
}

/// Visible grid window.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct AssetViewport {
    /// First visible item index.
    pub start: usize,
    /// Maximum visible item count.
    pub len: usize,
}

/// Storage policy that gates optional work.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum StoragePolicy {
    /// Normal mode opens optional high-res and cloud work.
    Normal,
    /// Pressure mode drops optional high-res and cloud work.
    Constrained,
}

/// Open smart album state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SmartAlbumSession {
    /// Current smart album rule.
    pub rule: SmartAlbumRule,
    /// Current visible viewport.
    pub viewport: AssetViewport,
    /// Current storage pressure policy.
    pub storage_policy: StoragePolicy,
}

/// Host-owned photo asset metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhotoAsset {
    /// Stable asset id.
    pub id: String,
    /// Asset media kind.
    pub media_kind: MediaKind,
    /// Whether the user marked this asset as favorite.
    pub favorite: bool,
    /// Person labels detected for this asset.
    pub people: BTreeSet<String>,
    /// Whether an original is available from cloud storage.
    pub cloud_available: bool,
    /// Whether the original is already local.
    pub local_original: bool,
}

/// Host-owned photo library catalog.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PhotoCatalog {
    /// Assets by id.
    pub assets: BTreeMap<String, PhotoAsset>,
}

/// Domain event applied to an open smart album.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PhotoStreamEvent {
    /// Replace the smart album rule.
    ReplaceRule(SmartAlbumRule),
    /// Scroll to a new visible window.
    ScrollViewport(AssetViewport),
    /// Change the storage policy.
    SetStoragePolicy(StoragePolicy),
    /// Replace the host-owned catalog.
    ReplaceCatalog(PhotoCatalog),
}

/// Host resource controlled by PhotoStream.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum PhotoResource {
    /// CPU thumbnail decode job.
    ThumbnailJob(String),
    /// Disk/cache metadata hydration job.
    MetadataHydration(String),
    /// Cloud original download job.
    CloudDownload(String),
    /// Memory-backed high-resolution preview job.
    HighResPreview(String),
}

/// Host command payload used by Trellis resource planning.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum PhotoCommand {
    /// Open the given photo resource.
    Open(PhotoResource),
}

/// Typed effect emitted to the photo host executor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PhotoEffect {
    /// Open the given resource.
    Open(PhotoResource),
    /// Close the given resource.
    Close(PhotoResource),
}

/// One bounded grid tile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhotoTile {
    /// Stable asset id.
    pub asset_id: String,
    /// Asset media kind.
    pub media_kind: MediaKind,
    /// Whether the tile can use a local original.
    pub local_original: bool,
}

/// Materialized bounded smart-album grid.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PhotoGridSnapshot {
    /// Current rule, if the album is open.
    pub rule: Option<SmartAlbumRule>,
    /// Total matching assets before viewport bounding.
    pub total_matches: usize,
    /// Tiles visible in the current viewport.
    pub tiles: Vec<PhotoTile>,
}

/// Public output frame emitted by the PhotoStream wrapper.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PhotoFrame {
    /// Initial baseline frame.
    Baseline(PhotoGridSnapshot),
    /// Incremental delta frame.
    Delta(PhotoGridSnapshot),
    /// Explicit rebaseline frame.
    Rebaseline(PhotoGridSnapshot),
    /// Clear frame emitted when the album scope closes.
    Cleared,
}

/// Count of wrapper effects and output frames emitted by an action.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PhotoStreamUpdate {
    /// Number of photo lifecycle effects queued.
    pub emitted_effects: usize,
    /// Number of grid frames queued.
    pub emitted_frames: usize,
}
