use std::collections::{BTreeMap, BTreeSet};

use crate::language;
use crate::types::{
    CanonicalInputs, CollectionDiff, FilePath, FullState, InputChange, OutputFrame, Revision,
};

pub fn full_recompute(
    inputs: &CanonicalInputs,
    analysis_revisions: &BTreeMap<FilePath, Revision>,
) -> FullState {
    let files = source_files(inputs);
    let has_verified_field = inputs
        .files
        .values()
        .any(|file| file.contents.contains("email_verified: bool"));
    let mut diagnostics_by_file = BTreeMap::new();
    let mut links_by_file = BTreeMap::new();
    let mut tokens_by_file = BTreeMap::new();
    let mut import_edges = Vec::new();
    let mut module_graph = Vec::new();

    for path in &files {
        let record = &inputs.files[path];
        let links = language::imports(path, &record.contents, &files);
        for link in &links {
            import_edges.push(format!("{} -> {}", link.file_path, link.target_path));
        }
        let diagnostics = language::diagnostics(
            path,
            &record.contents,
            &links,
            &inputs.compiler_config,
            has_verified_field,
        );
        if !diagnostics.is_empty() {
            diagnostics_by_file.insert(path.clone(), diagnostics);
        }
        links_by_file.insert(path.clone(), links);
        tokens_by_file.insert(
            path.clone(),
            language::semantic_tokens(path, &record.contents),
        );
        module_graph.push(format!("node:{path}"));
    }

    let mut desired_resources = vec![format!("WorkspaceIndex({})", inputs.active_branch)];
    for path in &files {
        if path.starts_with("generated/") {
            desired_resources.push(format!("GeneratedFileWatch({path})"));
        } else {
            desired_resources.push(format!("WatchFile({path})"));
        }
        let rev = analysis_revisions
            .get(path)
            .copied()
            .unwrap_or(inputs.scenario_revision);
        desired_resources.push(format!("AnalysisJob({path}@rev{rev})"));
    }
    desired_resources.sort();
    import_edges.sort();
    module_graph.sort();

    FullState {
        source_files: files,
        import_edges,
        diagnostics_by_file,
        links_by_file,
        tokens_by_file,
        desired_resources,
        module_graph,
    }
}

pub fn source_files(inputs: &CanonicalInputs) -> Vec<FilePath> {
    let mut files = inputs
        .files
        .values()
        .filter(|file| !file.generated || inputs.generated_files_enabled)
        .map(|file| file.path.clone())
        .collect::<Vec<_>>();
    files.sort();
    files
}

pub fn diff_vec(collection: &str, before: &[String], after: &[String]) -> CollectionDiff {
    let before = before.iter().cloned().collect::<BTreeSet<_>>();
    let after = after.iter().cloned().collect::<BTreeSet<_>>();
    CollectionDiff {
        collection: collection.to_owned(),
        added: after.difference(&before).cloned().collect(),
        removed: before.difference(&after).cloned().collect(),
        updated: Vec::new(),
    }
}

pub fn input_change(key: &str, before: impl ToString, after: impl ToString) -> InputChange {
    InputChange {
        key: key.to_owned(),
        before: before.to_string(),
        after: after.to_string(),
    }
}

pub fn frame(kind: &str, path: &str, revision: Revision, label: &str) -> OutputFrame {
    OutputFrame {
        kind: kind.to_owned(),
        output_key: format!("{kind}:{path}"),
        scope: format!("FileScope({path})"),
        revision,
        file_path: Some(path.to_owned()),
        diagnostics: Vec::new(),
        links: Vec::new(),
        tokens: Vec::new(),
        status: None,
        cause: cause(
            "files",
            label,
            kind,
            "output lifecycle follows current source graph",
        ),
    }
}

pub fn cause(
    input_key: &str,
    after: &str,
    changed: &str,
    reason: &str,
) -> crate::types::AuditCause {
    crate::types::AuditCause {
        input_key: input_key.to_owned(),
        before: "previous committed state".to_owned(),
        after: after.to_owned(),
        changed_node: changed.to_owned(),
        collection: "sourceFiles/moduleGraph/outputModel".to_owned(),
        added: Vec::new(),
        removed: Vec::new(),
        updated: Vec::new(),
        reason: reason.to_owned(),
        path: vec![
            "canonical inputs".to_owned(),
            "sourceFiles".to_owned(),
            "moduleGraph".to_owned(),
            "resourcePlan/outputFrames".to_owned(),
        ],
    }
}
