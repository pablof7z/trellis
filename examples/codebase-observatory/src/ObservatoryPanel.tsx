import { useMemo, useState } from "react";
import type { AppState, TraceItem } from "./types";

const tabs = [
  "Transaction",
  "Inputs",
  "Derived Diffs",
  "Resource Plan",
  "Output Frames",
  "Scopes",
  "Host Statuses",
  "Invariant Checks",
  "Replay",
  "Why?",
];

type Props = {
  state: AppState;
  dispatch: (action: Record<string, unknown>) => void;
};

export function ObservatoryPanel({ state, dispatch }: Props) {
  const [tab, setTab] = useState(tabs[0]);
  const trace = state.traces[state.traces.length - 1];
  const selected = useMemo(() => findWhyItem(state), [state]);
  return (
    <aside className="panel observatory">
      <div className="panel-title">Trellis Observatory</div>
      <div className="tabbar compact">
        {tabs.map((name) => (
          <button className={tab === name ? "active" : ""} key={name} onClick={() => setTab(name)}>
            {name}
          </button>
        ))}
      </div>
      {tab === "Transaction" && (
        <div className="trace-card">
          <h2>Tx {trace.txId} - {trace.label}</h2>
          <p>revision {trace.revision}</p>
          <Rows rows={trace.auditEdges} />
        </div>
      )}
      {tab === "Inputs" && <Rows rows={trace.inputChanges.map((c) => `${c.key}: ${c.before} -> ${c.after}`)} />}
      {tab === "Derived Diffs" && (
        <div className="stack">
          {trace.collectionDiffs.map((diff) => (
            <div className="trace-card" key={diff.collection}>
              <h3>{diff.collection}</h3>
              <p>added: {diff.added.join(", ") || "none"}</p>
              <p>removed: {diff.removed.join(", ") || "none"}</p>
            </div>
          ))}
        </div>
      )}
      {tab === "Resource Plan" && (
        <TraceList items={trace.resourceCommands} dispatch={dispatch} itemLabel={(item) => `${item.op} ${item.key}`} />
      )}
      {tab === "Output Frames" && (
        <TraceList items={trace.outputFrames} dispatch={dispatch} itemLabel={(item) => `${item.kind} ${item.outputKey}`} />
      )}
      {tab === "Scopes" && <Rows rows={trace.scopeEvents.map((event) => `${event.op} ${event.scope}: ${event.reason}`)} />}
      {tab === "Host Statuses" && (
        <div className="stack">
          {trace.hostStatusEvents.map((event) => (
            <button
              className={`trace-card selectable ${event.classification.includes("stale") ? "warn" : ""}`}
              key={`${event.status.path}:${event.status.commandRevision}`}
              onClick={() => dispatch({ type: "selectWhy", id: `host:${event.status.path}:${event.status.commandRevision}` })}
            >
              <h3>{event.status.kind} {event.status.path}@rev{event.status.commandRevision}</h3>
              <p>{event.classification}</p>
              <p>{event.effect}</p>
            </button>
          ))}
          {trace.hostStatusEvents.length === 0 && <div className="empty">No host statuses in this transaction</div>}
        </div>
      )}
      {tab === "Invariant Checks" && (
        <div className="stack">
          {trace.invariantChecks.map((check) => (
            <button
              className={`invariant ${check.status}`}
              key={check.id}
              onClick={() => dispatch({ type: "selectWhy", id: `invariant:${check.id}` })}
            >
              <span>{check.status === "pass" ? "PASS" : "FAIL"}</span>
              {check.label}
              {check.details && <small>{check.details}</small>}
            </button>
          ))}
        </div>
      )}
      {tab === "Replay" && <ReplayView state={state} />}
      {tab === "Why?" && <WhyView selected={selected} trace={trace} />}
    </aside>
  );
}

function TraceList({
  items,
  dispatch,
  itemLabel,
}: {
  items: TraceItem[];
  dispatch: Props["dispatch"];
  itemLabel: (item: TraceItem) => string;
}) {
  return (
    <div className="stack">
      {items.map((item) => (
        <button
          className={`trace-card selectable ${item.op === "Open" || item.kind?.startsWith("Baseline") ? "blue" : "gray"}`}
          key={`${item.op ?? item.kind}:${item.key ?? item.outputKey}`}
          onClick={() => dispatch({ type: "selectWhy", id: item.key ?? item.outputKey })}
        >
          <h3>{itemLabel(item)}</h3>
          <p>{item.scope}</p>
          <p>{item.cause.reason}</p>
        </button>
      ))}
    </div>
  );
}

function ReplayView({ state }: { state: AppState }) {
  const result = state.replayResult;
  if (!result) return <div className="empty">Click Replay trace in the scenario bar.</div>;
  return (
    <div className="stack">
      <div className={`trace-card ${result.status}`}>
        <h3>Replay {result.status}</h3>
        <p>Trace length: {result.traceLength}</p>
        <p>{result.finalObservableMatches ? "Final observable state matches." : "Final observable state diverged."}</p>
      </div>
      {result.checks.map((check) => (
        <div className={`invariant ${check.status}`} key={check.id}>
          <span>{check.status}</span>
          {check.label}
        </div>
      ))}
    </div>
  );
}

function WhyView({ selected, trace }: { selected: TraceItem | string | null; trace: AppState["traces"][number] }) {
  if (!selected) return <div className="empty">Select a resource command, output frame, host status, or invariant.</div>;
  if (typeof selected === "string") {
    return (
      <div className="trace-card">
        <h3>Why this status matters</h3>
        <p>{selected}</p>
        <p>Host statuses are later canonical input, not hidden callback state.</p>
      </div>
    );
  }
  return (
    <div className="trace-card why">
      <h3>{selected.op ?? selected.kind} {selected.key ?? selected.outputKey}</h3>
      <p>{selected.cause.reason}</p>
      <ol>
        {selected.cause.path.map((step) => (
          <li key={step}>{step}</li>
        ))}
      </ol>
      <p>Input: {selected.cause.inputKey}</p>
      <p>Derived node: {selected.cause.changedNode}</p>
    </div>
  );
}

function findWhyItem(state: AppState): TraceItem | string | null {
  const id = state.selectedWhy;
  if (!id) return null;
  const traces = [...state.traces].reverse();
  for (const trace of traces) {
    const item = [...trace.resourceCommands, ...trace.outputFrames].find((entry) => entry.key === id || entry.outputKey === id);
    if (item) return item;
    const host = trace.hostStatusEvents.find((event) => id === `host:${event.status.path}:${event.status.commandRevision}`);
    if (host) return `${host.classification}: ${host.reason}. Effect: ${host.effect}.`;
    const invariant = trace.invariantChecks.find((check) => id === `invariant:${check.id}`);
    if (invariant) return `${invariant.label}: ${invariant.details || "passed"}`;
  }
  return null;
}

function Rows({ rows }: { rows: string[] }) {
  if (rows.length === 0) return <div className="empty">No entries</div>;
  return <div className="runtime-list">{rows.map((row) => <div key={row}>{row}</div>)}</div>;
}
