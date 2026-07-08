use std::collections::{BTreeMap, BTreeSet};

use super::types::{ResultWindow, SearchCatalog, SearchDocument, SearchFilter, SearchSession};

/// Builds a sorted string set from literal values.
pub fn ids<const N: usize>(values: [&str; N]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

/// Opening search session used by the script.
pub fn opening_search() -> SearchSession {
    SearchSession {
        user: "analyst".to_owned(),
        corpus: "mail".to_owned(),
        query: "rust".to_owned(),
        filter: SearchFilter::All,
        window: ResultWindow { start: 0, len: 2 },
    }
}

/// Sample catalog for the SearchOps showcase.
pub fn sample_catalog() -> SearchCatalog {
    let mut documents = BTreeMap::new();
    for doc in [
        doc(
            "mail-001",
            "mail",
            "mail-a",
            "Rust launch plan",
            "rust rollout budget",
            ["plan"],
        ),
        doc(
            "mail-002",
            "mail",
            "mail-a",
            "Rust budget",
            "budget and compiler staffing",
            ["finance"],
        ),
        doc(
            "mail-003",
            "mail",
            "mail-b",
            "Rust support",
            "customer escalation and rust",
            ["support"],
        ),
        doc(
            "mail-004",
            "mail",
            "mail-b",
            "Search notes",
            "ranking budget",
            ["search"],
        ),
        doc(
            "docs-001",
            "docs",
            "docs-a",
            "Rust guide",
            "rust implementation notes",
            ["guide"],
        ),
        doc(
            "docs-002",
            "docs",
            "docs-a",
            "Search guide",
            "query ranking internals",
            ["search"],
        ),
        doc(
            "docs-003",
            "docs",
            "docs-b",
            "Budget policy",
            "budget planning",
            ["finance"],
        ),
    ] {
        documents.insert(doc.id.clone(), doc);
    }
    SearchCatalog { documents }
}

fn doc<const N: usize>(
    id: &str,
    corpus: &str,
    shard: &str,
    title: &str,
    body: &str,
    tags: [&str; N],
) -> SearchDocument {
    SearchDocument {
        id: id.to_owned(),
        corpus: corpus.to_owned(),
        shard: shard.to_owned(),
        title: title.to_owned(),
        body: body.to_owned(),
        tags: ids(tags),
        allowed_users: ids(["analyst"]),
    }
}
