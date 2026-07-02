use std::collections::{BTreeMap, BTreeSet};

use crate::types::{OutputFrame, OutputLedger, ResourceCommand, ResourceLedgerEntry, Revision};

pub fn empty_output_ledger() -> OutputLedger {
    OutputLedger {
        diagnostics_by_file: BTreeMap::new(),
        links_by_file: BTreeMap::new(),
        tokens_by_file: BTreeMap::new(),
        revisions_by_output_key: BTreeMap::new(),
    }
}

pub fn apply_resource_commands(
    ledger: &mut BTreeMap<String, ResourceLedgerEntry>,
    commands: &[ResourceCommand],
    tx_id: u32,
) {
    for command in commands {
        let key = command
            .new_key
            .as_ref()
            .or(command.old_key.as_ref())
            .unwrap_or(&command.key)
            .clone();
        let entry = ledger.entry(key.clone()).or_insert(ResourceLedgerEntry {
            key: key.clone(),
            state: "closed".to_owned(),
            owners: Vec::new(),
            open_count: 0,
            close_count: 0,
            cancel_count: 0,
            last_command_revision: 0,
            last_tx_id: 0,
            cause: String::new(),
        });
        match command.op.as_str() {
            "Open" => {
                entry.state = "open".to_owned();
                entry.open_count += 1;
                if !entry.owners.contains(&command.scope) {
                    entry.owners.push(command.scope.clone());
                }
            }
            "Close" => {
                entry.state = "closed".to_owned();
                entry.close_count += 1;
                entry.owners.retain(|owner| owner != &command.scope);
            }
            "Cancel" => {
                entry.state = "cancelled".to_owned();
                entry.cancel_count += 1;
                entry.owners.retain(|owner| owner != &command.scope);
            }
            _ => {}
        }
        entry.last_command_revision = command.command_revision;
        entry.last_tx_id = tx_id;
        entry.cause = command.cause.reason.clone();
        entry.owners.sort();
    }
}

pub fn apply_output_frames(ledger: &mut OutputLedger, frames: &[OutputFrame]) {
    for frame in frames {
        ledger
            .revisions_by_output_key
            .insert(frame.output_key.clone(), frame.revision);
        if let Some(path) = &frame.file_path {
            match frame.kind.as_str() {
                "ClearDiagnostics" => {
                    ledger.diagnostics_by_file.remove(path);
                }
                "BaselineDiagnostics" => {
                    if frame.diagnostics.is_empty() {
                        ledger.diagnostics_by_file.remove(path);
                    } else {
                        ledger
                            .diagnostics_by_file
                            .insert(path.clone(), frame.diagnostics.clone());
                    }
                }
                "ClearDocumentLinks" => {
                    ledger.links_by_file.remove(path);
                }
                "BaselineDocumentLinks" => {
                    ledger
                        .links_by_file
                        .insert(path.clone(), frame.links.clone());
                }
                "ClearSemanticTokens" => {
                    ledger.tokens_by_file.remove(path);
                }
                "BaselineSemanticTokens" => {
                    ledger
                        .tokens_by_file
                        .insert(path.clone(), frame.tokens.clone());
                }
                _ => {}
            }
        }
    }
}

pub fn resource_commands(
    before: &[String],
    after: &[String],
    revision: Revision,
    label: &str,
) -> Vec<ResourceCommand> {
    let before = before.iter().cloned().collect::<BTreeSet<_>>();
    let after = after.iter().cloned().collect::<BTreeSet<_>>();
    let mut commands = Vec::new();
    for key in before.difference(&after) {
        commands.push(command(close_op(key), key, revision, label));
    }
    for key in after.difference(&before) {
        commands.push(command("Open", key, revision, label));
    }
    commands.sort_by(|a, b| a.key.cmp(&b.key).then(a.op.cmp(&b.op)));
    commands
}

fn close_op(key: &str) -> &'static str {
    if key.starts_with("AnalysisJob(") {
        "Cancel"
    } else {
        "Close"
    }
}

fn command(op: &str, key: &str, revision: Revision, label: &str) -> ResourceCommand {
    ResourceCommand {
        op: op.to_owned(),
        key: key.to_owned(),
        old_key: None,
        new_key: None,
        scope: scope_for(key),
        command_revision: revision,
        policy: None,
        cause: crate::compute::cause(
            "files/config/hostStatuses",
            label,
            "resourcePlan",
            "resource demand reconciled against desired graph",
        ),
    }
}

fn scope_for(key: &str) -> String {
    let path = key
        .split_once('(')
        .and_then(|(_, rest)| rest.split_once(')').map(|(inner, _)| inner))
        .unwrap_or("workspace")
        .split_once("@rev")
        .map(|(path, _)| path)
        .unwrap_or("workspace");
    if key.starts_with("WorkspaceIndex(") {
        format!("WorkspaceScope({path})")
    } else {
        format!("FileScope({path})")
    }
}
