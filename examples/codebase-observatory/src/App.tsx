import { useEffect, useMemo, useState } from "react";
import { flattenDiagnostics, latestTrace, loadEngine, scenarioSteps } from "./engineClient";
import type { AppState, EngineApi } from "./types";
import { EditorPane } from "./EditorPane";
import { ProjectExplorer } from "./ProjectExplorer";
import { RuntimePanel } from "./RuntimePanel";
import { ObservatoryPanel } from "./ObservatoryPanel";
import { TraceViewer } from "./TraceViewer";
import { ShowcaseLab } from "./ShowcaseLab";

const showcaseStepIndex = 1;
type AppView = "runtime" | "showcases" | "traces";

export default function App() {
  const [engine, setEngine] = useState<EngineApi | null>(null);
  const [state, setState] = useState<AppState | null>(null);
  const [scenarioIndex, setScenarioIndex] = useState(showcaseStepIndex);
  const [faultsOpen, setFaultsOpen] = useState(false);
  const [view, setView] = useState<AppView>("showcases");

  useEffect(() => {
    loadEngine().then((api) => {
      setEngine(api);
      setState(showcaseState(api));
    });
  }, []);

  const dispatch = (action: Record<string, unknown>) => {
    if (!engine || !state) return;
    setState(engine.dispatch(state, action));
  };

  const runNextStep = () => {
    const step = scenarioSteps[scenarioIndex] ?? scenarioSteps[scenarioSteps.length - 1];
    dispatch(step.action);
    setScenarioIndex((index) => Math.min(index + 1, scenarioSteps.length));
  };

  const reset = () => {
    if (!engine) return;
    setState(showcaseState(engine));
    setScenarioIndex(showcaseStepIndex);
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

  const trace = latestTrace(state);
  const diagnostics = flattenDiagnostics(state);
  const scenarioText = scenarioStatus(scenarioIndex);

  if (view === "showcases") {
    return (
      <main className="app-shell showcase-app">
        <Topbar state={state} trace={trace} failures={failures.length} view={view} setView={setView} />
        <ShowcaseLab />
      </main>
    );
  }

  if (view === "traces") {
    return (
      <main className="app-shell trace-app">
        <Topbar state={state} trace={trace} failures={failures.length} view={view} setView={setView} />
        <TraceViewer />
      </main>
    );
  }

  return (
    <main className="app-shell">
      <Topbar state={state} trace={trace} failures={failures.length} view={view} setView={setView} />

      <section className="scenario-bar">
        <div className="scenario-copy">
          <strong>Scenario: Reconcile stale editor state</strong>
          <span>{scenarioText}</span>
        </div>
        <button className="button primary" onClick={runNextStep} disabled={scenarioIndex >= scenarioSteps.length}>
          Run next step
        </button>
        <button className="button secondary" onClick={replayTrace}>Replay trace</button>
        <button className="button ghost" onClick={reset}>Reset</button>
        <details className="faults" open={faultsOpen} onToggle={(event) => setFaultsOpen(event.currentTarget.open)}>
          <summary>Fault injection</summary>
          <div className="fault-menu">
            <label>
              Engine
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
                {faultLabel(key)}
              </label>
            ))}
          </div>
        </details>
        <div className="scenario-metrics">
          <span>{diagnostics.length} diagnostics</span>
          <span>{Object.values(state.outputLedger.linksByFile).flat().length} links</span>
          <span>{Object.values(state.resourceLedger).filter((r) => r.state === "open").length} resources</span>
        </div>
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

function Topbar({
  state,
  trace,
  failures,
  view,
  setView,
}: {
  state: AppState;
  trace: ReturnType<typeof latestTrace>;
  failures: number;
  view: AppView;
  setView: (view: AppView) => void;
}) {
  return (
    <header className="topbar">
      <div>
        <h1>trellis observatory</h1>
        <p>
          {state.inputs.activeBranch} · {state.inputs.compilerConfig} config · {state.inputs.activeEditor ?? "no editor"} · tx {trace.txId}
        </p>
      </div>
      <div className="topbar-actions">
        <button className={view === "showcases" ? "active" : ""} aria-pressed={view === "showcases"} onClick={() => setView("showcases")}>
          Showcase lab
        </button>
        <button className={view === "traces" ? "active" : ""} aria-pressed={view === "traces"} onClick={() => setView("traces")}>
          Trace viewer
        </button>
        <button className={view === "runtime" ? "active" : ""} aria-pressed={view === "runtime"} onClick={() => setView("runtime")}>
          Runtime lab
        </button>
        <span className={`mode ${state.mode}`}>{state.mode === "trellis" ? "[TRELLIS]" : "[CALLBACKS]"}</span>
        <span className={failures ? "badge fail" : "badge pass"}>
          {failures ? `[FAIL] ${failures}` : "[PASS] invariants"}
        </span>
      </div>
    </header>
  );
}

const emptyState = { traces: [] } as unknown as AppState;

function showcaseState(engine: EngineApi) {
  return engine.dispatch(engine.initialState(), scenarioSteps[0].action);
}

function scenarioStatus(index: number) {
  const completed = scenarioSteps[index - 1];
  const next = scenarioSteps[index];
  if (!completed) return `Step 1 of ${scenarioSteps.length} · ${scenarioSteps[0].story}`;
  const prefix = `Step ${Math.min(index, scenarioSteps.length)} of ${scenarioSteps.length}`;
  if (!next) return `${prefix} · ${completed.completed}`;
  return `${prefix} complete · ${completed.completed}`;
}

function faultLabel(key: string) {
  const labels: Record<string, string> = {
    skipClearDiagnosticsForDeletedFile: "Keep diagnostics after file deletion",
    skipDocumentLinkRebaseline: "Keep old document links",
    skipWatcherClose: "Leave closed-file watcher alive",
    acceptStaleAnalysisResults: "Accept late analysis result",
    skipScopeCloseOutputClear: "Preserve output after scope close",
  };
  return labels[key] ?? key;
}
