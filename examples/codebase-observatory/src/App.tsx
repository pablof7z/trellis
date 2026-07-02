import { useEffect, useMemo, useState } from "react";
import { loadEngine, latestTrace, scenarioActions } from "./engineClient";
import type { AppState, EngineApi } from "./types";
import { EditorPane } from "./EditorPane";
import { ProjectExplorer } from "./ProjectExplorer";
import { RuntimePanel } from "./RuntimePanel";
import { ObservatoryPanel } from "./ObservatoryPanel";

export default function App() {
  const [engine, setEngine] = useState<EngineApi | null>(null);
  const [state, setState] = useState<AppState | null>(null);

  useEffect(() => {
    loadEngine().then((api) => {
      setEngine(api);
      setState(api.initialState());
    });
  }, []);

  const dispatch = (action: Record<string, unknown>) => {
    if (!engine || !state) return;
    setState(engine.dispatch(state, action));
  };

  const replayTrace = () => {
    if (!engine || !state) return;
    const replayResult = engine.replay(state);
    setState({ ...state, replayResult });
  };

  const failures = useMemo(
    () => latestTrace(state ?? emptyState)?.invariantChecks.filter((check) => check.status === "fail") ?? [],
    [state],
  );

  if (!state) {
    return <main className="loading">Loading Codebase Observatory...</main>;
  }

  return (
    <main className="app-shell">
      <header className="topbar">
        <div>
          <h1>Codebase Observatory</h1>
          <p>A tiny IDE that makes stale diagnostics, file watchers, analysis jobs, and editor outputs deterministic.</p>
        </div>
        <div className="status-strip">
          <span className={`mode ${state.mode}`}>{state.mode === "trellis" ? "Trellis" : "Naive callbacks"}</span>
          <span className={failures.length ? "badge fail" : "badge pass"}>
            {failures.length ? `${failures.length} invariant failures` : "all invariants pass"}
          </span>
        </div>
      </header>

      <section className="scenario-bar">
        {scenarioActions.map(([label, action]) => (
          <button key={label} onClick={() => dispatch(action)}>
            {label}
          </button>
        ))}
        <button onClick={replayTrace}>Replay trace</button>
        <button onClick={() => dispatch({ type: "reset" })}>Reset</button>
      </section>

      <section className="mode-row">
        <label>
          Mode
          <select value={state.mode} onChange={(event) => dispatch({ type: "setMode", mode: event.target.value })}>
            <option value="trellis">Trellis</option>
            <option value="naive">Naive callbacks</option>
          </select>
        </label>
        {Object.entries(state.bugPolicy).map(([key, value]) => (
          <label key={key} className="checkbox">
            <input
              type="checkbox"
              checked={value}
              onChange={(event) => dispatch({ type: "setBug", key, value: event.target.checked })}
            />
            {key.replace(/[A-Z]/g, " $&").toLowerCase()}
          </label>
        ))}
      </section>

      <section className="workspace">
        <ProjectExplorer state={state} dispatch={dispatch} />
        <EditorPane state={state} />
        <ObservatoryPanel state={state} dispatch={dispatch} />
      </section>
      <RuntimePanel state={state} />
    </main>
  );
}

const emptyState = { traces: [] } as unknown as AppState;
