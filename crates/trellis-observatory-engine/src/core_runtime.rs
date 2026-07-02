use std::collections::{BTreeMap, BTreeSet};

use trellis_core::{
    CollectionNode, DependencyList, Graph, InputNode, MaterializedOutput, ResourceKey,
    ResourcePlan, ScopeId, TransactionResult,
};

use crate::compute::{full_recompute, source_files};
use crate::core_projection::project;
use crate::types::{
    CanonicalInputs, Diagnostic, DocumentLink, FilePath, FullState, OutputFrame, ResourceCommand,
    Revision, SemanticToken,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CoreCommand;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum CoreOutput {
    Diagnostics(FilePath, Vec<Diagnostic>),
    Links(FilePath, Vec<DocumentLink>),
    Tokens(FilePath, Vec<SemanticToken>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum OutputKind {
    Diagnostics,
    Links,
    Tokens,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct OutputIdentity {
    pub(crate) kind: OutputKind,
    pub(crate) path: FilePath,
}

pub struct CoreProjection {
    pub transaction_id: u64,
    pub revision: u64,
    pub resource_commands: Vec<ResourceCommand>,
    pub output_frames: Vec<OutputFrame>,
    pub audit_edges: Vec<String>,
}

pub struct CoreTransition<'a> {
    pub before_inputs: &'a CanonicalInputs,
    pub before_analysis: &'a BTreeMap<FilePath, Revision>,
    pub before_full: &'a FullState,
    pub after_inputs: &'a CanonicalInputs,
    pub after_analysis: &'a BTreeMap<FilePath, Revision>,
    pub after_full: &'a FullState,
    pub app_revision: Revision,
    pub label: &'a str,
}

pub(crate) struct CoreHarness {
    pub(crate) graph: Graph<CoreCommand, CoreOutput>,
    inputs: InputNode<CanonicalInputs>,
    revisions: InputNode<BTreeMap<FilePath, Revision>>,
    source_files: CollectionNode<FilePath, ()>,
    file_scopes: BTreeMap<FilePath, ScopeId>,
    pub(crate) output_labels: BTreeMap<u64, OutputIdentity>,
}

pub fn bootstrap(
    inputs: &CanonicalInputs,
    analysis_revisions: &BTreeMap<FilePath, Revision>,
    full: &FullState,
) -> CoreProjection {
    let (harness, result) = build_harness(inputs, analysis_revisions, full);
    project(
        &harness,
        &result,
        inputs.scenario_revision,
        "Open main branch",
    )
}

pub fn transition(input: CoreTransition<'_>) -> CoreProjection {
    let (mut harness, _) = build_harness(
        input.before_inputs,
        input.before_analysis,
        input.before_full,
    );
    let removed = source_diff(input.before_full, input.after_full).0;
    let added = source_diff(input.before_full, input.after_full).1;
    let graph = &mut harness.graph;
    let mut tx = graph.begin_transaction().unwrap();
    tx.set_input(harness.inputs, input.after_inputs.clone())
        .unwrap();
    tx.set_input(harness.revisions, input.after_analysis.clone())
        .unwrap();
    for path in removed {
        if let Some(scope) = harness.file_scopes.get(&path) {
            tx.close_scope(*scope).unwrap();
        }
    }
    for path in added {
        create_file_scope(
            &mut tx,
            harness.inputs,
            harness.revisions,
            harness.source_files,
            &mut harness.file_scopes,
            &mut harness.output_labels,
            path,
        );
    }
    let result = tx.commit().unwrap();
    drop(tx);
    project(&harness, &result, input.app_revision, input.label)
}

fn build_harness(
    inputs: &CanonicalInputs,
    analysis_revisions: &BTreeMap<FilePath, Revision>,
    full: &FullState,
) -> (CoreHarness, TransactionResult<CoreCommand, CoreOutput>) {
    let mut graph = Graph::<CoreCommand, CoreOutput>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let workspace = tx
        .create_scope(format!("WorkspaceScope({})", inputs.active_branch))
        .unwrap();
    let inputs_node = tx.input::<CanonicalInputs>("canonicalInputs").unwrap();
    let revisions_node = tx
        .input::<BTreeMap<FilePath, Revision>>("analysisRevisions")
        .unwrap();
    tx.set_input(inputs_node, inputs.clone()).unwrap();
    tx.set_input(revisions_node, analysis_revisions.clone())
        .unwrap();
    let source_node = source_collection(&mut tx, inputs_node);
    let resources_node = desired_resources_collection(&mut tx, inputs_node, revisions_node);
    tx.set_resource_planner(resources_node, workspace, move |ctx| {
        let mut plan = ResourcePlan::new();
        for added in &ctx.diff().added {
            plan.open(
                ResourceKey::new(added.value.clone()),
                ctx.scope(),
                CoreCommand,
            );
        }
        for removed in &ctx.diff().removed {
            plan.close(ResourceKey::new(removed.value.clone()), ctx.scope());
        }
        Ok(plan)
    })
    .unwrap();
    let mut file_scopes = BTreeMap::new();
    let mut output_labels = BTreeMap::new();
    for path in &full.source_files {
        create_file_scope(
            &mut tx,
            inputs_node,
            revisions_node,
            source_node,
            &mut file_scopes,
            &mut output_labels,
            path.clone(),
        );
    }
    let result = tx.commit().unwrap();
    drop(tx);
    let harness = CoreHarness {
        graph,
        inputs: inputs_node,
        revisions: revisions_node,
        source_files: source_node,
        file_scopes,
        output_labels,
    };
    (harness, result)
}

fn source_collection(
    tx: &mut trellis_core::Transaction<'_, CoreCommand, CoreOutput>,
    inputs: InputNode<CanonicalInputs>,
) -> CollectionNode<FilePath, ()> {
    tx.set_collection(
        "sourceFiles",
        DependencyList::new([inputs.id()]).unwrap(),
        move |ctx| Ok(source_files(ctx.input(inputs)?).into_iter().collect()),
    )
    .unwrap()
}

fn desired_resources_collection(
    tx: &mut trellis_core::Transaction<'_, CoreCommand, CoreOutput>,
    inputs: InputNode<CanonicalInputs>,
    revisions: InputNode<BTreeMap<FilePath, Revision>>,
) -> CollectionNode<String, ()> {
    tx.set_collection(
        "desiredResources",
        DependencyList::new([inputs.id(), revisions.id()]).unwrap(),
        move |ctx| {
            Ok(full_recompute(ctx.input(inputs)?, ctx.input(revisions)?)
                .desired_resources
                .into_iter()
                .collect())
        },
    )
    .unwrap()
}

fn create_file_scope(
    tx: &mut trellis_core::Transaction<'_, CoreCommand, CoreOutput>,
    inputs: InputNode<CanonicalInputs>,
    revisions: InputNode<BTreeMap<FilePath, Revision>>,
    source_files: CollectionNode<FilePath, ()>,
    file_scopes: &mut BTreeMap<FilePath, ScopeId>,
    output_labels: &mut BTreeMap<u64, OutputIdentity>,
    path: FilePath,
) {
    let scope = tx.create_scope(format!("FileScope({path})")).unwrap();
    file_scopes.insert(path.clone(), scope);
    for kind in [
        OutputKind::Diagnostics,
        OutputKind::Links,
        OutputKind::Tokens,
    ] {
        let output = create_output(
            tx,
            inputs,
            revisions,
            source_files,
            scope,
            &path,
            kind.clone(),
        );
        output_labels.insert(
            output.key().get(),
            OutputIdentity {
                kind,
                path: path.clone(),
            },
        );
    }
}

fn create_output(
    tx: &mut trellis_core::Transaction<'_, CoreCommand, CoreOutput>,
    inputs: InputNode<CanonicalInputs>,
    revisions: InputNode<BTreeMap<FilePath, Revision>>,
    source_files: CollectionNode<FilePath, ()>,
    scope: ScopeId,
    path: &str,
    kind: OutputKind,
) -> MaterializedOutput<CoreOutput> {
    let path = path.to_owned();
    tx.materialized_output(
        output_key(&kind, &path),
        scope,
        DependencyList::new([inputs.id(), revisions.id(), source_files.id()]).unwrap(),
        move |ctx| {
            let full = full_recompute(ctx.input(inputs)?, ctx.input(revisions)?);
            Ok(match kind {
                OutputKind::Diagnostics => CoreOutput::Diagnostics(
                    path.clone(),
                    full.diagnostics_by_file
                        .get(&path)
                        .cloned()
                        .unwrap_or_default(),
                ),
                OutputKind::Links => CoreOutput::Links(
                    path.clone(),
                    full.links_by_file.get(&path).cloned().unwrap_or_default(),
                ),
                OutputKind::Tokens => CoreOutput::Tokens(
                    path.clone(),
                    full.tokens_by_file.get(&path).cloned().unwrap_or_default(),
                ),
            })
        },
    )
    .unwrap()
}

fn source_diff(before: &FullState, after: &FullState) -> (Vec<FilePath>, Vec<FilePath>) {
    let before = before.source_files.iter().cloned().collect::<BTreeSet<_>>();
    let after = after.source_files.iter().cloned().collect::<BTreeSet<_>>();
    (
        before.difference(&after).cloned().collect(),
        after.difference(&before).cloned().collect(),
    )
}

fn output_key(kind: &OutputKind, path: &str) -> String {
    match kind {
        OutputKind::Diagnostics => format!("Diagnostics:{path}"),
        OutputKind::Links => format!("DocumentLinks:{path}"),
        OutputKind::Tokens => format!("SemanticTokens:{path}"),
    }
}
