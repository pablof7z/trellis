use trellis_core::{
    ClearReason, OutputFrameKind, ResourceCommand as CoreResourceCommand, ScopeId,
    TransactionResult,
};

use crate::compute::cause;
use crate::core_runtime::{
    CoreCommand, CoreHarness, CoreOutput, CoreProjection, OutputIdentity, OutputKind,
};
use crate::types::{
    Diagnostic, DocumentLink, OutputFrame, ResourceCommand, Revision, SemanticToken,
};

pub(crate) fn project(
    harness: &CoreHarness,
    result: &TransactionResult<CoreCommand>,
    app_revision: Revision,
    label: &str,
) -> CoreProjection {
    CoreProjection {
        transaction_id: result.transaction_id.get(),
        revision: result.revision.get(),
        resource_commands: resource_commands(harness, result, app_revision, label),
        output_frames: output_frames(harness, result, app_revision, label),
        audit_edges: audit_edges(result),
    }
}

fn resource_commands(
    harness: &CoreHarness,
    result: &TransactionResult<CoreCommand>,
    app_revision: Revision,
    label: &str,
) -> Vec<ResourceCommand> {
    result
        .resource_plan
        .commands()
        .iter()
        .map(|command| {
            let key = command.key().as_str().to_owned();
            let op = match command {
                CoreResourceCommand::Open { .. } => "Open",
                CoreResourceCommand::Close { .. } if key.starts_with("AnalysisJob(") => "Cancel",
                CoreResourceCommand::Close { .. } => "Close",
                CoreResourceCommand::Replace { .. } => "Replace",
                CoreResourceCommand::Refresh { .. } => "Replace",
            };
            ResourceCommand {
                op: op.to_owned(),
                key: key.clone(),
                old_key: None,
                new_key: None,
                scope: resource_scope_label(harness, &key, command.scope()),
                command_revision: app_revision,
                policy: None,
                cause: cause(
                    "trellis-core transaction",
                    label,
                    "resourcePlan",
                    "trellis-core ResourcePlan reconciled desired resource demand",
                ),
            }
        })
        .collect()
}

fn output_frames(
    harness: &CoreHarness,
    result: &TransactionResult<CoreCommand>,
    app_revision: Revision,
    label: &str,
) -> Vec<OutputFrame> {
    result
        .output_frames
        .iter()
        .filter_map(|frame| match &frame.kind {
            OutputFrameKind::Baseline(output)
            | OutputFrameKind::Delta(output)
            | OutputFrameKind::Rebaseline(output, _) => Some(payload_frame(
                output.get::<CoreOutput>()?,
                app_revision,
                label,
            )),
            OutputFrameKind::Clear(ClearReason::ScopeClosed) => harness
                .output_labels
                .get(&frame.output_key.get())
                .map(|identity| clear_frame(identity, app_revision, label)),
        })
        .collect()
}

fn payload_frame(output: &CoreOutput, revision: Revision, label: &str) -> OutputFrame {
    let (kind, path, diagnostics, links, tokens) = match output {
        CoreOutput::Diagnostics(path, diagnostics) => (
            "BaselineDiagnostics",
            path,
            diagnostics.clone(),
            Vec::new(),
            Vec::new(),
        ),
        CoreOutput::Links(path, links) => (
            "BaselineDocumentLinks",
            path,
            Vec::new(),
            links.clone(),
            Vec::new(),
        ),
        CoreOutput::Tokens(path, tokens) => (
            "BaselineSemanticTokens",
            path,
            Vec::new(),
            Vec::new(),
            tokens.clone(),
        ),
    };
    frame(kind, path, revision, diagnostics, links, tokens, label)
}

fn clear_frame(identity: &OutputIdentity, revision: Revision, label: &str) -> OutputFrame {
    frame(
        match identity.kind {
            OutputKind::Diagnostics => "ClearDiagnostics",
            OutputKind::Links => "ClearDocumentLinks",
            OutputKind::Tokens => "ClearSemanticTokens",
        },
        &identity.path,
        revision,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        label,
    )
}

fn frame(
    kind: &str,
    path: &str,
    revision: Revision,
    diagnostics: Vec<Diagnostic>,
    links: Vec<DocumentLink>,
    tokens: Vec<SemanticToken>,
    label: &str,
) -> OutputFrame {
    OutputFrame {
        kind: kind.to_owned(),
        output_key: format!("{kind}:{path}"),
        scope: format!("FileScope({path})"),
        revision,
        file_path: Some(path.to_owned()),
        diagnostics,
        links,
        tokens,
        status: None,
        cause: cause(
            "trellis-core transaction",
            label,
            kind,
            "trellis-core OutputFrame materialized committed output state",
        ),
    }
}

fn audit_edges<C>(result: &TransactionResult<C>) -> Vec<String> {
    result
        .phase_trace
        .iter()
        .map(|phase| format!("trellis-core::{phase:?}"))
        .collect()
}

fn resource_path(key: &str) -> Option<&str> {
    if key.starts_with("WorkspaceIndex(") {
        return None;
    }
    key.split_once('(')
        .and_then(|(_, rest)| rest.split_once(')').map(|(inner, _)| inner))
        .map(|inner| inner.split_once("@rev").map_or(inner, |(path, _)| path))
}

fn scope_label(harness: &CoreHarness, scope: ScopeId) -> String {
    harness
        .graph
        .scope_meta(scope)
        .map(|meta| meta.debug_name().to_owned())
        .unwrap_or_else(|| format!("Scope({})", scope.get()))
}

fn resource_scope_label(harness: &CoreHarness, key: &str, core_scope: ScopeId) -> String {
    resource_path(key)
        .map(|path| format!("FileScope({path})"))
        .unwrap_or_else(|| scope_label(harness, core_scope))
}
