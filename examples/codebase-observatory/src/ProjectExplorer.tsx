import type { AppState } from "./types";

type Props = {
  state: AppState;
  dispatch: (action: Record<string, unknown>) => void;
};

export function ProjectExplorer({ state, dispatch }: Props) {
  const srcFiles = fileRows(state, "src/");
  const generatedFiles = fileRows(state, "generated/");
  const removedFiles = latestRemovedFiles(state);
  return (
    <aside className="panel explorer">
      <div className="panel-title">Project</div>
      <div className="explorer-controls">
        <select
          aria-label="Branch"
          value={state.inputs.activeBranch}
          onChange={(event) => dispatch({ type: "switchBranch", branch: event.target.value })}
        >
          <option value="main">main</option>
          <option value="feature/schema-v2">feature/schema-v2</option>
        </select>
        <select
          aria-label="Compiler config"
          value={state.inputs.compilerConfig}
          onChange={(event) => dispatch({ type: "changeConfig", config: event.target.value })}
        >
          <option value="strict">strict</option>
          <option value="loose">loose</option>
        </select>
      </div>
      <label className="generated-toggle">
        <input type="checkbox" checked={state.inputs.generatedFilesEnabled} onChange={() => dispatch({ type: "toggleGenerated" })} />
        Show generated files
      </label>
      <div className="tree">
        <Folder name="src" rows={srcFiles} active={state.inputs.activeEditor} dispatch={dispatch} />
        {state.inputs.generatedFilesEnabled && <Folder name="generated" rows={generatedFiles} active={state.inputs.activeEditor} dispatch={dispatch} />}
        {removedFiles.length > 0 && (
          <div className="folder">
            <div className="folder-name muted">recently reconciled</div>
            {removedFiles.map((path) => (
              <div className="tree-row deleted" key={path}>
                <span className="file-name">{basename(path)}</span>
                <span className="file-meta">deleted</span>
              </div>
            ))}
          </div>
        )}
      </div>
      <div className="subhead">Open files</div>
      {state.inputs.openEditors.map((path) => (
        <div className="open-file" key={path}>
          {path}
        </div>
      ))}
    </aside>
  );
}

type FileRow = { path: string; name: string; diagnosticCount: number };

function Folder({
  name,
  rows,
  active,
  dispatch,
}: {
  name: string;
  rows: FileRow[];
  active: string | null;
  dispatch: Props["dispatch"];
}) {
  return (
    <div className="folder">
      <div className="folder-name"><span>▾</span>{name}/</div>
      {rows.map((row) => (
        <button
          className={row.path === active ? "tree-row active" : "tree-row"}
          key={row.path}
          onClick={() => dispatch({ type: "openFile", path: row.path })}
        >
          <span className="file-name">{row.name}</span>
          {row.diagnosticCount > 0 && <span className="file-meta danger">{plural(row.diagnosticCount, "error")}</span>}
        </button>
      ))}
    </div>
  );
}

function fileRows(state: AppState, prefix: string): FileRow[] {
  return Object.keys(state.inputs.files)
    .filter((path) => path.startsWith(prefix))
    .sort()
    .map((path) => ({
      path,
      name: basename(path),
      diagnosticCount: state.outputLedger.diagnosticsByFile[path]?.length ?? 0,
    }));
}

function latestRemovedFiles(state: AppState) {
  const trace = state.traces[state.traces.length - 1];
  return trace.collectionDiffs.find((diff) => diff.collection === "sourceFiles")?.removed ?? [];
}

function basename(path: string) {
  return path.split("/").pop() ?? path;
}

function plural(count: number, noun: string) {
  return `${count} ${noun}${count === 1 ? "" : "s"}`;
}
