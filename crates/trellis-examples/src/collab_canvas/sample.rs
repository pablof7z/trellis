use std::collections::BTreeSet;

use super::types::DocumentManifest;

/// Builds a set of ids from string literals.
pub fn ids(values: impl IntoIterator<Item = &'static str>) -> BTreeSet<String> {
    values.into_iter().map(str::to_owned).collect()
}

/// Primary design document before optional embeds or visible attachments.
pub fn design_doc_base() -> DocumentManifest {
    DocumentManifest {
        subdocuments: ids(["theme"]),
        comment_threads: ids(["intro-thread"]),
        presence_rooms: ids(["design-room"]),
        attachments: ids(["hero.png", "notes.pdf"]),
    }
}

/// Primary design document with an added embedded specification.
pub fn design_doc_with_spec() -> DocumentManifest {
    let mut manifest = design_doc_base();
    manifest.subdocuments.insert("spec".to_owned());
    manifest
}

/// Secondary document that shares the theme subdocument.
pub fn style_doc() -> DocumentManifest {
    DocumentManifest {
        subdocuments: ids(["theme"]),
        comment_threads: ids(["style-thread"]),
        presence_rooms: ids(["style-room"]),
        attachments: ids(["palette.png"]),
    }
}
