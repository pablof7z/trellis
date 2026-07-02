use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub type FilePath = String;
pub type BranchId = String;
pub type Revision = u32;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalInputs {
    pub active_branch: BranchId,
    pub files: BTreeMap<FilePath, FileRecord>,
    pub open_editors: Vec<FilePath>,
    pub active_editor: Option<FilePath>,
    pub compiler_config: CompilerConfig,
    pub generated_files_enabled: bool,
    pub host_statuses: Vec<HostStatus>,
    pub scenario_revision: Revision,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileRecord {
    pub path: FilePath,
    pub contents: String,
    pub generated: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CompilerConfig {
    Strict,
    Loose,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub id: String,
    pub file_path: FilePath,
    pub line: usize,
    pub column: usize,
    pub severity: String,
    pub message: String,
    pub source: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentLink {
    pub id: String,
    pub file_path: FilePath,
    pub target_path: FilePath,
    pub line: usize,
    pub column_start: usize,
    pub column_end: usize,
    pub status: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SemanticToken {
    pub id: String,
    pub file_path: FilePath,
    pub line: usize,
    pub column_start: usize,
    pub column_end: usize,
    pub token_type: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HostStatus {
    pub kind: String,
    pub path: FilePath,
    pub command_revision: Revision,
    pub diagnostics: Vec<Diagnostic>,
    pub error: Option<String>,
    pub status_revision: Revision,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuditCause {
    pub input_key: String,
    pub before: String,
    pub after: String,
    pub changed_node: String,
    pub collection: String,
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub updated: Vec<String>,
    pub reason: String,
    pub path: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCommand {
    pub op: String,
    pub key: String,
    pub old_key: Option<String>,
    pub new_key: Option<String>,
    pub scope: String,
    pub command_revision: Revision,
    pub policy: Option<String>,
    pub cause: AuditCause,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OutputFrame {
    pub kind: String,
    pub output_key: String,
    pub scope: String,
    pub revision: Revision,
    pub file_path: Option<FilePath>,
    pub diagnostics: Vec<Diagnostic>,
    pub links: Vec<DocumentLink>,
    pub tokens: Vec<SemanticToken>,
    pub status: Option<String>,
    pub cause: AuditCause,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FullState {
    pub source_files: Vec<FilePath>,
    pub import_edges: Vec<String>,
    pub diagnostics_by_file: BTreeMap<FilePath, Vec<Diagnostic>>,
    pub links_by_file: BTreeMap<FilePath, Vec<DocumentLink>>,
    pub tokens_by_file: BTreeMap<FilePath, Vec<SemanticToken>>,
    pub desired_resources: Vec<String>,
    pub module_graph: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceLedgerEntry {
    pub key: String,
    pub state: String,
    pub owners: Vec<String>,
    pub open_count: u32,
    pub close_count: u32,
    pub cancel_count: u32,
    pub last_command_revision: Revision,
    pub last_tx_id: u32,
    pub cause: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OutputLedger {
    pub diagnostics_by_file: BTreeMap<FilePath, Vec<Diagnostic>>,
    pub links_by_file: BTreeMap<FilePath, Vec<DocumentLink>>,
    pub tokens_by_file: BTreeMap<FilePath, Vec<SemanticToken>>,
    pub revisions_by_output_key: BTreeMap<String, Revision>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScopeEvent {
    pub op: String,
    pub scope: String,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HostStatusEvent {
    pub status: HostStatus,
    pub classification: String,
    pub reason: String,
    pub effect: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InvariantCheck {
    pub id: String,
    pub label: String,
    pub status: String,
    pub details: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CollectionDiff {
    pub collection: String,
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub updated: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InputChange {
    pub key: String,
    pub before: String,
    pub after: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChangedNode {
    pub id: String,
    pub summary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransactionTrace {
    pub tx_id: u32,
    pub revision: Revision,
    pub core_backed: bool,
    pub core_transaction_id: Option<u64>,
    pub core_revision: Option<u64>,
    pub label: String,
    pub input_changes: Vec<InputChange>,
    pub changed_nodes: Vec<ChangedNode>,
    pub collection_diffs: Vec<CollectionDiff>,
    pub resource_commands: Vec<ResourceCommand>,
    pub output_frames: Vec<OutputFrame>,
    pub scope_events: Vec<ScopeEvent>,
    pub host_status_events: Vec<HostStatusEvent>,
    pub invariant_checks: Vec<InvariantCheck>,
    pub audit_edges: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NaiveBugPolicy {
    pub skip_clear_diagnostics_for_deleted_file: bool,
    pub skip_document_link_rebaseline: bool,
    pub skip_watcher_close: bool,
    pub accept_stale_analysis_results: bool,
    pub skip_scope_close_output_clear: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ObservableState {
    pub diagnostics_by_file: BTreeMap<FilePath, Vec<Diagnostic>>,
    pub links_by_file: BTreeMap<FilePath, Vec<DocumentLink>>,
    pub tokens_by_file: BTreeMap<FilePath, Vec<SemanticToken>>,
    pub resources: Vec<ResourceLedgerEntry>,
    pub active_jobs: Vec<String>,
    pub watchers: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    pub mode: String,
    pub bug_policy: NaiveBugPolicy,
    pub inputs: CanonicalInputs,
    pub full: FullState,
    pub resource_ledger: BTreeMap<String, ResourceLedgerEntry>,
    pub output_ledger: OutputLedger,
    pub traces: Vec<TransactionTrace>,
    pub action_log: Vec<Action>,
    pub analysis_revisions: BTreeMap<FilePath, Revision>,
    pub closed_scopes: Vec<String>,
    pub selected_why: Option<String>,
    pub replay_result: Option<ReplayResult>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Action {
    Reset,
    SetMode { mode: String },
    SetBug { key: String, value: bool },
    DeleteFile { path: FilePath },
    SwitchBranch { branch: BranchId },
    RenameSchema,
    EditAppWithTypeError,
    FixApp,
    StartSlowAnalysis,
    InjectStaleAnalysisResult,
    ToggleGenerated,
    ChangeConfig { config: CompilerConfig },
    CloseAppTab,
    OpenFile { path: FilePath },
    SelectWhy { id: Option<String> },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReplayResult {
    pub status: String,
    pub trace_length: usize,
    pub checks: Vec<InvariantCheck>,
    pub final_observable_matches: bool,
}
