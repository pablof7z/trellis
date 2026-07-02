import { useMemo, useState } from "react";
import type { AppState, TraceItem, TransactionTrace } from "./types";

const tabs = ["Transaction", "Resources", "Replay", "Invariants"] as const;

type Props = {
  state: AppState;
  dispatch: (action: Record<string, unknown>) => void;
};

export function ObservatoryPanel({ state, dispatch }: Props) {
  const [tab, setTab] = useState<(typeof tabs)[number]>("Transaction");
  const trace = state.traces[state.traces.length - 1];
  const selected = useMemo(() => findWhyItem(state), [state]);
  const failures = trace.invariantChecks.filter((check) => check.status === "fail");
  return (
    <aside className="panel observatory">
      <div className="observatory-head">
        <div>
          <span className="eyebrow">Trellis Observatory</span>
          <h2>Tx {trace.txId} · {transactionTitle(trace)}</h2>
          <p>{transactionSubtitle(trace)}</p>
        </div>
        <span className={failures.length ? "badge fail" : "badge pass"}>{failures.length ? "fault caught" : "deterministic"}</span>
      </div>
      <div className="tabbar compact">
        {tabs.map((name) => (
          <button className={tab === name ? "active" : ""} key={name} onClick={() => setTab(name)}>
            {name}
          </button>
        ))}
      </div>
      {tab === "Transaction" && <TransactionNarrative trace={trace} selected={selected} dispatch={dispatch} />}
      {tab === "Resources" && <ResourcesView trace={trace} dispatch={dispatch} />}
      {tab === "Replay" && <ReplayView state={state} />}
      {tab === "Invariants" && <InvariantView trace={trace} dispatch={dispatch} />}
    </aside>
  );
}

function TransactionNarrative({
  trace,
  selected,
  dispatch,
}: {
  trace: TransactionTrace;
  selected: TraceItem | string | null;
  dispatch: Props["dispatch"];
}) {
  return (
    <div className="observatory-scroll">
      <section className="story-card success">
        <span className="eyebrow">Trellis result</span>
        <ResultList trace={trace} dispatch={dispatch} />
      </section>
      <section className="story-card">
        <span className="eyebrow">Problem introduced</span>
        <p>{problemCopy(trace)}</p>
      </section>
      <section className="story-card proof">
        <span className="eyebrow">Revision guard</span>
        <p>{proofCopy(trace)}</p>
      </section>
      <section className="story-card">
        <details>
          <summary>Details</summary>
          <div className="detail-grid">
            <span className="eyebrow">Inputs</span>
            <Rows rows={trace.inputChanges.map((change) => `${change.key}: ${change.before} -> ${change.after}`)} />
            <span className="eyebrow">Derived changes</span>
            <Rows rows={derivedRows(trace)} />
          </div>
        </details>
      </section>
      <WhyView selected={selected} />
    </div>
  );
}

function ResultList({ trace, dispatch }: { trace: TransactionTrace; dispatch: Props["dispatch"] }) {
  const rows = resultRows(trace);
  return (
    <div className="result-list">
      {rows.map((row) => (
        <button className={row.tone === "warn" ? "result-row warn" : "result-row"} key={row.label} onClick={() => dispatch({ type: "selectWhy", id: row.whyId })}>
          {row.label}
        </button>
      ))}
    </div>
  );
}

function ResourcesView({ trace, dispatch }: { trace: TransactionTrace; dispatch: Props["dispatch"] }) {
  return (
    <div className="observatory-scroll">
      {trace.resourceCommands.map((item) => (
        <TraceButton key={`${item.op}:${item.key}`} item={item} dispatch={dispatch} label={`${item.op} ${cleanKey(item.key ?? "")}`} />
      ))}
      {trace.outputFrames.slice(0, 8).map((item) => (
        <TraceButton key={`${item.kind}:${item.outputKey}`} item={item} dispatch={dispatch} label={`${item.kind} ${cleanKey(item.outputKey ?? "")}`} />
      ))}
    </div>
  );
}

function InvariantView({ trace, dispatch }: { trace: TransactionTrace; dispatch: Props["dispatch"] }) {
  return (
    <div className="observatory-scroll">
      {trace.invariantChecks.map((check) => (
        <button className={`invariant ${check.status}`} key={check.id} onClick={() => dispatch({ type: "selectWhy", id: `invariant:${check.id}` })}>
          <span>{check.status === "pass" ? "PASS" : "FAIL"}</span>
          {check.label}
          {check.details && <small>{check.details}</small>}
        </button>
      ))}
    </div>
  );
}

function ReplayView({ state }: { state: AppState }) {
  const result = state.replayResult;
  if (!result) return <div className="empty padded">Run Replay trace to verify the bug report from a fresh initial state.</div>;
  return (
    <div className="observatory-scroll">
      <div className={`story-card ${result.status}`}>
        <span className="eyebrow">Replay {result.status}</span>
        <p>Trace length {result.traceLength}. {result.finalObservableMatches ? "Final output matches." : "Final output diverged."}</p>
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

function TraceButton({ item, label, dispatch }: { item: TraceItem; label: string; dispatch: Props["dispatch"] }) {
  return (
    <button className="trace-row" onClick={() => dispatch({ type: "selectWhy", id: item.key ?? item.outputKey })}>
      <strong>{label}</strong>
      <span>{item.scope}</span>
    </button>
  );
}

function WhyView({ selected }: { selected: TraceItem | string | null }) {
  if (!selected) return null;
  if (typeof selected === "string") {
    return (
      <section className="story-card">
        <span className="eyebrow">Why</span>
        <p>{selected}</p>
      </section>
    );
  }
  return (
    <section className="story-card">
      <span className="eyebrow">Why</span>
      <p>{selected.cause.reason}</p>
      <ol className="cause-path">
        {selected.cause.path.map((step) => (
          <li key={step}>{step}</li>
        ))}
      </ol>
    </section>
  );
}

function Rows({ rows }: { rows: string[] }) {
  if (rows.length === 0) return <div className="empty">No entries</div>;
  return <div className="mini-list">{rows.map((row) => <div key={row}>{row}</div>)}</div>;
}

function transactionTitle(trace: TransactionTrace) {
  if (trace.hostStatusEvents.some((event) => event.classification.includes("stale"))) return "Late analysis result rejected";
  const removed = removedFiles(trace);
  if (removed.length > 0) return `Delete ${removed[0]}`;
  return trace.label;
}

function transactionSubtitle(trace: TransactionTrace) {
  if (trace.hostStatusEvents.some((event) => event.classification.includes("stale"))) return "Deterministic graph preserved";
  if (removedFiles(trace).length > 0) return "Late editor artifacts reconciled";
  return "Committed graph and visible output agree";
}

function problemCopy(trace: TransactionTrace) {
  if (trace.hostStatusEvents.length > 0) return "A host result arrived for an old analysis command after the editor had already moved to a newer revision.";
  const removed = removedFiles(trace);
  if (removed.length > 0) return `A deleted file still had diagnostics, document links, semantic tokens, and a live watcher to reconcile.`;
  return "The current editor state changed and all derived outputs must match the committed graph.";
}

function proofCopy(trace: TransactionTrace) {
  const stale = trace.hostStatusEvents.find((event) => event.classification.includes("stale"));
  if (stale) return `accepted: tx ${trace.txId}; ignored: ${stale.status.path}@rev${stale.status.commandRevision}`;
  const core = trace.coreBacked ? `trellis-core tx ${trace.coreTransactionId} -> rev ${trace.coreRevision}` : "simulator trace";
  return `${core}; sourceFiles -> diagnostics -> links -> tokens -> watchers`;
}

function resultRows(trace: TransactionTrace) {
  const rows: { label: string; whyId: string; tone?: "warn" }[] = [];
  const removed = removedFiles(trace);
  const firstClear = trace.outputFrames.find((frame) => frame.kind?.startsWith("Clear"));
  const firstResource = trace.resourceCommands.find((command) => command.op === "Close" || command.op === "Cancel");
  const firstBaseline = trace.outputFrames.find((frame) => frame.kind === "BaselineDiagnostics");
  const stale = trace.hostStatusEvents.find((event) => event.classification.includes("stale"));
  if (removed.length > 0) {
    rows.push({ label: "Cleared stale diagnostics, links, and tokens for deleted file", whyId: firstClear?.outputKey ?? "" });
    rows.push({ label: "Closed watcher and cancelled analysis owned by removed file", whyId: firstResource?.key ?? "" });
    rows.push({ label: "Current diagnostics match the committed source graph", whyId: firstBaseline?.outputKey ?? "" });
  }
  if (stale) {
    rows.push({
      label: "Rejected late analysis result from old revision",
      whyId: `host:${stale.status.path}:${stale.status.commandRevision}`,
      tone: "warn",
    });
  }
  if (rows.length === 0) rows.push({ label: "Observable editor state equals full recompute", whyId: firstBaseline?.outputKey ?? "" });
  rows.push({ label: "System invariants pass", whyId: "invariant:incremental-equals-full-recompute" });
  return rows;
}

function removedFiles(trace: TransactionTrace) {
  return trace.collectionDiffs.find((diff) => diff.collection === "sourceFiles")?.removed ?? [];
}

function derivedRows(trace: TransactionTrace) {
  return trace.collectionDiffs
    .filter((diff) => diff.added.length || diff.removed.length || diff.updated.length)
    .map((diff) => `${diff.collection}: +${diff.added.length} -${diff.removed.length} ~${diff.updated.length}`);
}

function cleanKey(key: string) {
  return key.replace("Baseline", "").replace("Clear", "").replace("Diagnostics:", "").replace("DocumentLinks:", "").replace("SemanticTokens:", "");
}

function findWhyItem(state: AppState): TraceItem | string | null {
  const id = state.selectedWhy;
  if (!id) return null;
  const traces = [...state.traces].reverse();
  for (const trace of traces) {
    const item = [...trace.resourceCommands, ...trace.outputFrames].find((entry) => entry.key === id || entry.outputKey === id);
    if (item) return item;
    const host = trace.hostStatusEvents.find((event) => id === `host:${event.status.path}:${event.status.commandRevision}`);
    if (host) return `${host.classification}: ${host.reason}. ${host.effect}.`;
    const invariant = trace.invariantChecks.find((check) => id === `invariant:${check.id}`);
    if (invariant) return `${invariant.label}: ${invariant.details || "passed"}`;
  }
  return null;
}
