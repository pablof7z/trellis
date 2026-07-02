import { useState } from "react";
import { flattenDiagnostics } from "./engineClient";
import type { AppState } from "./types";

const tabs = ["Problems", "Links", "Tokens", "Watchers", "Jobs", "Frames", "Hosts"];

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
        <table className="problems-table">
          <thead>
            <tr>
              <th>Severity</th>
              <th>File</th>
              <th>Line</th>
              <th>Message</th>
              <th>Source</th>
            </tr>
          </thead>
          <tbody>
            {flattenDiagnostics(state).map((d) => (
              <tr key={d.id}>
                <td><span className="severity-dot">●</span> Error</td>
                <td>{d.filePath}</td>
                <td>{d.line}</td>
                <td>{formatMessage(d.message)}</td>
                <td>{d.source}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      {tab === "Links" && <Rows rows={Object.entries(state.outputLedger.linksByFile).flatMap(([file, links]) => links.map((l) => `${file} -> ${l.targetPath} (${l.status})`))} />}
      {tab === "Tokens" && <Rows rows={Object.entries(state.outputLedger.tokensByFile).flatMap(([file, tokens]) => tokens.map((t) => `${file}:${t.line} ${t.tokenType}`))} />}
      {tab === "Watchers" && <Rows rows={resources.filter((r) => r.key.includes("Watch")).map((r) => `${r.key} ${r.state} owners=${r.owners.join(",")}`)} />}
      {tab === "Jobs" && <Rows rows={resources.filter((r) => r.key.startsWith("AnalysisJob(")).map((r) => `${r.key} ${r.state}`)} />}
      {tab === "Frames" && <Rows rows={trace.outputFrames.map((f) => `${f.kind} ${f.outputKey} rev${f.revision}`)} />}
      {tab === "Hosts" && <Rows rows={trace.hostStatusEvents.map((h) => `${h.status.kind} ${h.status.path}@rev${h.status.commandRevision}: ${h.classification}`)} />}
    </section>
  );
}

function Rows({ rows }: { rows: string[] }) {
  if (rows.length === 0) return <div className="empty">No entries</div>;
  return <div className="runtime-list">{rows.map((row) => <div key={row}>{row}</div>)}</div>;
}

function formatMessage(message: string) {
  return message.replace("add(number, string)", "`add(number, string)`").replace("email_verified", "`email_verified`");
}
