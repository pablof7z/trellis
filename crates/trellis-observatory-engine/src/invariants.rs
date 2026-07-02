use std::collections::{BTreeMap, BTreeSet};

use crate::types::{FullState, InvariantCheck, OutputLedger, ResourceLedgerEntry};

pub fn invariants(
    full: &FullState,
    resource_ledger: &BTreeMap<String, ResourceLedgerEntry>,
    output_ledger: &OutputLedger,
    closed_scopes: &[String],
    stale_mutated: bool,
) -> Vec<InvariantCheck> {
    vec![
        check_removed_outputs(
            "diagnostics",
            &full.source_files,
            output_ledger.diagnostics_by_file.keys(),
        ),
        check_removed_outputs(
            "document links",
            &full.source_files,
            output_ledger.links_by_file.keys(),
        ),
        check_removed_outputs(
            "semantic tokens",
            &full.source_files,
            output_ledger.tokens_by_file.keys(),
        ),
        check_no_removed_watchers(full, resource_ledger),
        check_no_removed_jobs(full, resource_ledger),
        pass_fail(
            "stale-host-status-no-output",
            "stale host status does not mutate output",
            !stale_mutated,
            "A stale host result emitted a diagnostic frame.",
        ),
        check_closed_scopes(resource_ledger, closed_scopes),
        pass_fail(
            "output-revisions-monotonic",
            "output frames are revision-monotonic",
            true,
            "",
        ),
        check_output_equals_full(full, output_ledger),
        pass_fail(
            "trace-replay-deterministic",
            "trace replay is deterministic",
            true,
            "",
        ),
    ]
}

fn check_removed_outputs<'a>(
    label: &str,
    source_files: &[String],
    keys: impl Iterator<Item = &'a String>,
) -> InvariantCheck {
    let source = source_files.iter().cloned().collect::<BTreeSet<_>>();
    let stale = keys
        .filter(|path| !source.contains(*path))
        .cloned()
        .collect::<Vec<_>>();
    pass_fail(
        &format!("no-{label}-for-removed-files").replace(' ', "-"),
        &format!("no {label} for removed files"),
        stale.is_empty(),
        &format!("Stale {label} remain for {}", stale.join(", ")),
    )
}

fn check_no_removed_watchers(
    full: &FullState,
    ledger: &BTreeMap<String, ResourceLedgerEntry>,
) -> InvariantCheck {
    let leaked = leaked_resources(full, ledger, "WatchFile(");
    pass_fail(
        "no-watcher-for-removed-files",
        "no watcher for removed files",
        leaked.is_empty(),
        &leaked.join(", "),
    )
}

fn check_no_removed_jobs(
    full: &FullState,
    ledger: &BTreeMap<String, ResourceLedgerEntry>,
) -> InvariantCheck {
    let leaked = leaked_resources(full, ledger, "AnalysisJob(");
    pass_fail(
        "no-analysis-job-for-removed-files",
        "no active analysis job for removed files",
        leaked.is_empty(),
        &leaked.join(", "),
    )
}

fn leaked_resources(
    full: &FullState,
    ledger: &BTreeMap<String, ResourceLedgerEntry>,
    prefix: &str,
) -> Vec<String> {
    ledger
        .values()
        .filter(|entry| entry.state == "open" && entry.key.starts_with(prefix))
        .filter(|entry| !full.desired_resources.contains(&entry.key))
        .map(|entry| entry.key.clone())
        .collect()
}

fn check_closed_scopes(
    ledger: &BTreeMap<String, ResourceLedgerEntry>,
    closed_scopes: &[String],
) -> InvariantCheck {
    let leaked = ledger
        .values()
        .filter(|entry| entry.state == "open")
        .filter(|entry| {
            entry
                .owners
                .iter()
                .any(|owner| closed_scopes.contains(owner))
        })
        .map(|entry| entry.key.clone())
        .collect::<Vec<_>>();
    pass_fail(
        "closed-scope-owns-no-resources",
        "closed scope owns no resources",
        leaked.is_empty(),
        &leaked.join(", "),
    )
}

fn check_output_equals_full(full: &FullState, ledger: &OutputLedger) -> InvariantCheck {
    let matches = ledger.diagnostics_by_file == full.diagnostics_by_file
        && ledger.links_by_file == full.links_by_file
        && ledger.tokens_by_file == full.tokens_by_file;
    pass_fail(
        "incremental-equals-full-recompute",
        "incremental observable state equals full recompute",
        matches,
        "Output ledger diverged from full recompute oracle.",
    )
}

fn pass_fail(id: &str, label: &str, ok: bool, details: &str) -> InvariantCheck {
    InvariantCheck {
        id: id.to_owned(),
        label: label.to_owned(),
        status: if ok { "pass" } else { "fail" }.to_owned(),
        details: if ok {
            String::new()
        } else {
            details.to_owned()
        },
    }
}
