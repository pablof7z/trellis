use crate::bugs::stale_status;
use crate::seed::{APP_MAIN, APP_V2, SCHEMA_V2, branch_files};
use crate::types::{Action, AppState, FileRecord, NaiveBugPolicy};

pub(crate) fn mutate_inputs(state: &mut AppState, action: &Action) -> String {
    match action {
        Action::DeleteFile { path } => {
            state.inputs.files.remove(path);
            state.inputs.open_editors.retain(|open| open != path);
            state.analysis_revisions.remove(path);
            format!("Delete {path}")
        }
        Action::SwitchBranch { branch } => {
            state.inputs.active_branch = branch.clone();
            state.inputs.files = branch_files(branch);
            state.inputs.open_editors = vec!["src/app.tl".to_owned()];
            state.inputs.active_editor = Some("src/app.tl".to_owned());
            state.analysis_revisions = state
                .inputs
                .files
                .keys()
                .map(|path| (path.clone(), state.inputs.scenario_revision + 1))
                .collect();
            format!("Switch branch to {branch}")
        }
        Action::RenameSchema => rename_schema(state),
        Action::EditAppWithTypeError => {
            if let Some(app) = state.inputs.files.get_mut("src/app.tl") {
                app.contents = APP_MAIN.replace("./legacy_user.tl\n", "");
            }
            bump_app_job(state);
            "Edit src/app.tl with type error".to_owned()
        }
        Action::FixApp => {
            if let Some(app) = state.inputs.files.get_mut("src/app.tl") {
                app.contents = APP_V2.to_owned();
            }
            bump_app_job(state);
            "Fix src/app.tl".to_owned()
        }
        Action::StartSlowAnalysis => {
            bump_app_job(state);
            "Start slow analysis for src/app.tl".to_owned()
        }
        Action::InjectStaleAnalysisResult => inject_stale_status(state),
        Action::ToggleGenerated => {
            state.inputs.generated_files_enabled = !state.inputs.generated_files_enabled;
            "Toggle generated files".to_owned()
        }
        Action::ChangeConfig { config } => {
            state.inputs.compiler_config = config.clone();
            "Change compiler config".to_owned()
        }
        Action::CloseAppTab => {
            state
                .inputs
                .open_editors
                .retain(|path| path != "src/app.tl");
            state.inputs.active_editor = None;
            state
                .closed_scopes
                .push("EditorTabScope(src/app.tl)".to_owned());
            "Close src/app.tl tab".to_owned()
        }
        Action::OpenFile { path } => {
            if !state.inputs.open_editors.contains(path) {
                state.inputs.open_editors.push(path.clone());
            }
            state.inputs.active_editor = Some(path.clone());
            format!("Open {path}")
        }
        _ => "No-op".to_owned(),
    }
}

pub(crate) fn set_bug(policy: &mut NaiveBugPolicy, key: &str, value: bool) {
    match key {
        "skipClearDiagnosticsForDeletedFile" => {
            policy.skip_clear_diagnostics_for_deleted_file = value
        }
        "skipDocumentLinkRebaseline" => policy.skip_document_link_rebaseline = value,
        "skipWatcherClose" => policy.skip_watcher_close = value,
        "acceptStaleAnalysisResults" => policy.accept_stale_analysis_results = value,
        "skipScopeCloseOutputClear" => policy.skip_scope_close_output_clear = value,
        _ => {}
    }
}

fn rename_schema(state: &mut AppState) -> String {
    state.inputs.files.remove("src/schema.tl");
    state.inputs.files.insert(
        "src/schema_v2.tl".to_owned(),
        FileRecord {
            path: "src/schema_v2.tl".to_owned(),
            contents: SCHEMA_V2.to_owned(),
            generated: false,
        },
    );
    if let Some(app) = state.inputs.files.get_mut("src/app.tl") {
        app.contents = app.contents.replace("./schema.tl", "./schema_v2.tl");
    }
    "Rename src/schema.tl -> src/schema_v2.tl".to_owned()
}

fn inject_stale_status(state: &mut AppState) -> String {
    let current = *state
        .analysis_revisions
        .get("src/app.tl")
        .unwrap_or(&state.inputs.scenario_revision);
    state.inputs.host_statuses.push(stale_status(
        current.saturating_sub(1),
        state.inputs.scenario_revision + 1,
    ));
    "Inject stale AnalysisFinished(src/app.tl)".to_owned()
}

fn bump_app_job(state: &mut AppState) {
    state
        .analysis_revisions
        .insert("src/app.tl".to_owned(), state.inputs.scenario_revision + 1);
}
