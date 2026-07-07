use std::collections::{BTreeMap, BTreeSet};

use crate::showcase_trace::{ShowcaseTrace, build_showcase_trace, step_with_oracle};
use trellis_core::{DependencyList, Graph, ResourceKey};

/// Host command payload for file watcher resources.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WatchCommand {
    /// Watch a file path.
    Watch(String),
}

fn key(file: &str) -> ResourceKey {
    ResourceKey::from_segments(["watch", file])
}

fn files(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
    entries
        .iter()
        .map(|(path, contents)| ((*path).to_owned(), (*contents).to_owned()))
        .collect()
}

fn imports(contents: &str) -> Vec<String> {
    contents
        .lines()
        .filter_map(|line| line.strip_prefix("import "))
        .map(str::to_owned)
        .collect()
}

fn import_closure(root: &str, graph: &BTreeMap<String, Vec<String>>) -> BTreeSet<String> {
    if !graph.contains_key(root) {
        return BTreeSet::new();
    }
    let mut affected = BTreeSet::new();
    let mut pending = vec![root.to_owned()];
    while let Some(path) = pending.pop() {
        if !affected.insert(path.clone()) {
            continue;
        }
        if let Some(imports) = graph.get(&path) {
            pending.extend(imports.iter().rev().cloned());
        }
    }
    affected
}

fn diagnostics(files: &BTreeMap<String, String>, affected: &BTreeSet<String>) -> Vec<String> {
    affected
        .iter()
        .filter_map(|path| {
            files.get(path).and_then(|contents| {
                contents
                    .contains("error")
                    .then(|| format!("{path}: contains error"))
            })
        })
        .collect()
}

/// Built language-server example graph and its public input.
pub struct MiniLanguageServerExample {
    /// Example graph.
    pub graph: Graph<WatchCommand>,
    /// Root file whose import closure is observed.
    pub open_file: trellis_core::InputNode<String>,
    /// File contents canonical input.
    pub file_contents: trellis_core::InputNode<BTreeMap<String, String>>,
    /// Project scope owning watchers and diagnostic output.
    pub project_scope: trellis_core::ScopeId,
}

/// Builds the mini language-server proof graph.
pub fn build_graph(initial_files: BTreeMap<String, String>) -> MiniLanguageServerExample {
    let mut graph = Graph::<WatchCommand>::new_with_command_type();
    let mut tx = graph.begin_transaction().unwrap();
    let scope = tx.create_scope("project").unwrap();
    let file_contents = tx
        .input::<BTreeMap<String, String>>("file-contents")
        .unwrap();
    let open_file = tx.input::<String>("open-file").unwrap();
    tx.set_input(file_contents, initial_files).unwrap();
    tx.set_input(open_file, "a.tl".to_owned()).unwrap();
    let module_graph = tx
        .derived(
            "module-graph",
            DependencyList::new([file_contents.id()]).unwrap(),
            move |ctx| {
                Ok(ctx
                    .input(file_contents)?
                    .iter()
                    .map(|(path, contents)| (path.clone(), imports(contents)))
                    .collect::<BTreeMap<_, _>>())
            },
        )
        .unwrap();
    let affected = tx
        .set_collection(
            "affected-files",
            DependencyList::new([open_file.id(), module_graph.id()]).unwrap(),
            move |ctx| {
                Ok(import_closure(
                    ctx.input(open_file)?,
                    ctx.derived(module_graph)?,
                ))
            },
        )
        .unwrap();
    tx.open_close_planner(
        affected,
        scope,
        |file| key(file),
        |file| WatchCommand::Watch(file.clone()),
    )
    .unwrap();
    tx.materialized_output(
        "diagnostics",
        scope,
        DependencyList::new([file_contents.id(), affected.id()]).unwrap(),
        move |ctx| {
            Ok(diagnostics(
                ctx.input(file_contents)?,
                ctx.set_collection(affected)?,
            ))
        },
    )
    .unwrap();
    tx.commit().unwrap();
    drop(tx);
    MiniLanguageServerExample {
        graph,
        open_file,
        file_contents,
        project_scope: scope,
    }
}

/// Runs the headless `delete-file` showcase script.
pub fn delete_file_showcase_trace() -> ShowcaseTrace {
    build_showcase_trace(
        "mini-language-server",
        "delete-file",
        &[
            "cargo",
            "run",
            "-p",
            "trellis-examples",
            "--example",
            "mini_language_server",
            "--",
            "--script",
            "delete-file",
        ],
        || {
            let mut example = build_graph(files(&[("a.tl", "error"), ("b.tl", "ok")]));
            let mut steps = Vec::new();

            let mut tx = example.graph.begin_transaction().unwrap();
            tx.set_input(example.file_contents, files(&[("b.tl", "ok")]))
                .unwrap();
            let result = tx.commit().unwrap();
            drop(tx);
            steps.push(step_with_oracle("delete-file", &example.graph, &result));

            let mut tx = example.graph.begin_transaction().unwrap();
            tx.close_scope(example.project_scope).unwrap();
            let result = tx.commit().unwrap();
            drop(tx);
            steps.push(step_with_oracle("close-workspace", &example.graph, &result));

            steps
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use trellis_core::{OutputFrameKind, ResourceCommand};

    #[test]
    fn file_delete_clears_diagnostics_and_closes_watcher() {
        let mut example = build_graph(files(&[("a.tl", "error"), ("b.tl", "ok")]));

        let mut tx = example.graph.begin_transaction().unwrap();
        tx.set_input(example.file_contents, files(&[("b.tl", "ok")]))
            .unwrap();
        let result = tx.commit().unwrap();
        drop(tx);

        assert!(
            result
                .resource_plan
                .commands()
                .contains(&ResourceCommand::Close {
                    key: key("a.tl"),
                    scope: result.resource_plan.commands()[0].scope(),
                })
        );
        assert!(matches!(
            &result.output_frames[0].kind,
            OutputFrameKind::Delta(diags)
                if diags
                    .get::<Vec<String>>()
                    .is_some_and(Vec::is_empty)
        ));
        example.graph.assert_incremental_equals_full().unwrap();
    }

    #[test]
    fn import_change_rebaselines_diagnostics() {
        let mut example = build_graph(files(&[
            ("a.tl", "import b.tl"),
            ("b.tl", "error"),
            ("c.tl", "error"),
        ]));

        let mut tx = example.graph.begin_transaction().unwrap();
        tx.set_input(
            example.file_contents,
            files(&[
                ("a.tl", "import c.tl"),
                ("b.tl", "error"),
                ("c.tl", "error"),
            ]),
        )
        .unwrap();
        let result = tx.commit().unwrap();
        drop(tx);

        assert!(
            result
                .resource_plan
                .commands()
                .contains(&ResourceCommand::Close {
                    key: key("b.tl"),
                    scope: result.resource_plan.commands()[0].scope(),
                })
        );
        assert!(
            result
                .resource_plan
                .commands()
                .contains(&ResourceCommand::Open {
                    key: key("c.tl"),
                    scope: result.resource_plan.commands()[0].scope(),
                    command: WatchCommand::Watch("c.tl".to_owned()),
                })
        );
        assert!(matches!(
            &result.output_frames[0].kind,
            OutputFrameKind::Delta(diags)
                if diags
                    .get::<Vec<String>>()
                    .is_some_and(|diags| diags == &vec!["c.tl: contains error".to_owned()])
        ));
        example.graph.assert_incremental_equals_full().unwrap();
    }
}
