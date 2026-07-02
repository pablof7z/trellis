import { useState } from "react";
import { flattenDiagnostics } from "./engineClient";
import type { AppState } from "./types";

const tabs = ["Problems", "Document Links", "Semantic Tokens", "Watchers", "Analysis Jobs", "Output Frames", "Host Statuses"];

export function RuntimePanel({ state }: { state: AppState }) {
  const [tab, setTab] = useState(tabs[0]);
  const resources = Object.values(state.resourceLedger).sort((a, b) => a.key.localeCompare(b.key));
  const trace = state.traces[state.traces.length - 1];
  return (
    <section className="runtime panel">
      <div className="tabbar">
        {tabs.map((name) => (
          <button className={tab === name ? "active" : ""} key={name} onClick={() => setTab(name)}>
            {name}
          </button>
        ))}
      </div>
      {tab === "Problems" && (
        <div className="runtime-list">{flattenDiagnostics(state).map((d) => <div key={d.id}>{d.filePath}:{d.line} {d.message}</div>)}</div>
      )}
      {tab === "Document Links" && <Rows rows={Object.entries(state.outputLedger.linksByFile).flatMap(([file, links]) => links.map((l) => `${file} -> ${l.targetPath} (${l.status})`))} />}
      {tab === "Semantic Tokens" && <Rows rows={Object.entries(state.outputLedger.tokensByFile).flatMap(([file, tokens]) => tokens.map((t) => `${file}:${t.line} ${t.tokenType}`))} />}
      {tab === "Watchers" && <Rows rows={resources.filter((r) => r.key.includes("Watch")).map((r) => `${r.key} ${r.state} owners=${r.owners.join(",")}`)} />}
      {tab === "Analysis Jobs" && <Rows rows={resources.filter((r) => r.key.startsWith("AnalysisJob(")).map((r) => `${r.key} ${r.state}`)} />}
      {tab === "Output Frames" && <Rows rows={trace.outputFrames.map((f) => `${f.kind} ${f.outputKey} rev${f.revision}`)} />}
      {tab === "Host Statuses" && <Rows rows={trace.hostStatusEvents.map((h) => `${h.status.kind} ${h.status.path}@rev${h.status.commandRevision}: ${h.classification}`)} />}
    </section>
  );
}

function Rows({ rows }: { rows: string[] }) {
  if (rows.length === 0) return <div className="empty">No entries</div>;
  return <div className="runtime-list">{rows.map((row) => <div key={row}>{row}</div>)}</div>;
}
