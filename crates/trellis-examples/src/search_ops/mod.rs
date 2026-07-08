//! SearchOps live search/index dashboard secondary showcase.

mod bug_capsules;
mod engine;
mod graph;
mod sample;
mod scripts;
mod selectors;
mod types;

#[cfg(test)]
mod tests;

pub use bug_capsules::{available_bug_capsules, run_all_bug_capsules, run_bug_capsule};
pub use engine::SearchOpsApp;
pub use sample::{ids, opening_search, sample_catalog};
pub use scripts::search_lifecycle_showcase_trace;
pub use types::{
    ResultWindow, SearchCatalog, SearchDocument, SearchEffect, SearchFilter, SearchFrame,
    SearchHandle, SearchOpsEvent, SearchOpsUpdate, SearchResource, SearchResultRow, SearchSession,
    SearchSnapshot,
};
