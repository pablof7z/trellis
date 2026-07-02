import type { AppState } from "./types";

type Props = {
  state: AppState;
  dispatch: (action: Record<string, unknown>) => void;
};

export function ProjectExplorer({ state, dispatch }: Props) {
  const files = Object.keys(state.inputs.files).sort();
  return (
    <aside className="panel explorer">
      <div className="panel-title">Project Explorer</div>
      <label>
        Branch
        <select
          value={state.inputs.activeBranch}
          onChange={(event) => dispatch({ type: "switchBranch", branch: event.target.value })}
        >
          <option value="main">main</option>
          <option value="feature/schema-v2">feature/schema-v2</option>
        </select>
      </label>
      <label>
        Config
        <select
          value={state.inputs.compilerConfig}
          onChange={(event) => dispatch({ type: "changeConfig", config: event.target.value })}
        >
          <option value="strict">strict</option>
          <option value="loose">loose</option>
        </select>
      </label>
      <label className="checkbox">
        <input
          type="checkbox"
          checked={state.inputs.generatedFilesEnabled}
          onChange={() => dispatch({ type: "toggleGenerated" })}
        />
        generated files
      </label>
      <div className="tree">
        {files.map((path) => (
          <button
            className={path === state.inputs.activeEditor ? "tree-row active" : "tree-row"}
            key={path}
            onClick={() => dispatch({ type: "openFile", path })}
          >
            <span>{path.startsWith("generated/") ? "gen" : "src"}</span>
            {path}
          </button>
        ))}
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
