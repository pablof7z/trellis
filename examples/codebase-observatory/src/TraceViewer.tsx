import { useMemo, useState } from "react";
import {
  outputFrameKind,
  resourceKeyLabel,
  selectedDetail,
  showcaseTraces,
  summarizeStep,
  traceCostRows,
  type CollectionDiff,
  type ContractTrace,
  type OutputFrame,
  type ResourceCommand,
  type ScopeEvent,
  type ShowcaseStep,
  type TraceSelection,
} from "./traceContract";

const tabs = [
  "Transactions",
  "Graph",
  "Diffs",
  "Resources",
  "Frames",
  "Scopes",
  "Hosts",
  "Oracle",
  "Replay",
  "Conformance",
  "Cost",
] as const;

export function TraceViewer() {
  const [showcaseIndex, setShowcaseIndex] = useState(0);
  const [stepIndex, setStepIndex] = useState(0);
  const [tab, setTab] = useState<(typeof tabs)[number]>("Transactions");
  const [selection, setSelection] = useState<TraceSelection | null>(null);
  const showcase = showcaseTraces[showcaseIndex];
  const step = showcase.steps[stepIndex] ?? showcase.steps[0];
  const detail = useMemo(() => selectedDetail(step, selection), [step, selection]);

  const chooseShowcase = (index: number) => {
    setShowcaseIndex(index);
    setStepIndex(0);
    setSelection(null);
  };

  const chooseStep = (index: number) => {
    setStepIndex(index);
    setSelection(null);
  };

  return (
    <section className="trace-viewer">
      <aside className="trace-sidebar panel">
        <label className="trace-control">
          <span className="eyebrow">Trace source</span>
          <select value={showcaseIndex} onChange={(event) => chooseShowcase(Number(event.target.value))}>
            {showcaseTraces.map((trace, index) => (
              <option key={trace.showcase} value={index}>
                {trace.showcase}
              </option>
            ))}
          </select>
        </label>
        <div className="trace-summary">
          <h2>{showcase.script}</h2>
          <p>{showcase.contract} · v{showcase.format_version}</p>
          <p>{showcase.command.join(" ")}</p>
        </div>
        <div className="trace-steps">
          {showcase.steps.map((item, index) => (
            <button className={index === stepIndex ? "trace-step active" : "trace-step"} key={item.name} onClick={() => chooseStep(index)}>
              <strong>{item.name}</strong>
              <StepStats step={item} />
            </button>
          ))}
        </div>
      </aside>

      <section className="trace-main panel">
        <header className="trace-main-head">
          <div>
            <span className="eyebrow">tx {step.trace.transaction_id}</span>
            <h2>{step.name}</h2>
          </div>
          <MetricStrip step={step} />
        </header>
        <div className="tabbar trace-tabs">
          {tabs.map((name) => (
            <button className={tab === name ? "active" : ""} key={name} onClick={() => setTab(name)}>
              {name}
            </button>
          ))}
        </div>
        <div className="trace-tab-body">{renderTab(tab, showcase, step, setSelection)}</div>
      </section>

      <aside className="trace-detail panel">
        <span className="eyebrow">Selected detail</span>
        <h2>{detail.title}</h2>
        <Rows rows={detail.rows} />
        <span className="eyebrow">Structural cause</span>
        <Rows rows={detail.causeRows} />
      </aside>
    </section>
  );
}

function renderTab(
  tab: (typeof tabs)[number],
  showcase: (typeof showcaseTraces)[number],
  step: ShowcaseStep,
  setSelection: (selection: TraceSelection) => void,
) {
  const trace = step.trace;
  switch (tab) {
    case "Transactions":
      return <TransactionRows steps={showcase.steps} active={step.name} />;
    case "Graph":
      return <GraphRows trace={trace} />;
    case "Diffs":
      return <Rows rows={trace.collection_diffs.map(diffRow)} />;
    case "Resources":
      return <ResourceRows commands={trace.resource_commands} setSelection={setSelection} />;
    case "Frames":
      return <FrameRows frames={trace.output_frames} setSelection={setSelection} />;
    case "Scopes":
      return <ScopeRows events={trace.scope_events} setSelection={setSelection} />;
    case "Hosts":
      return <HostRows step={step} setSelection={setSelection} />;
    case "Oracle":
      return <Rows rows={trace.invariant_results.map((item) => `${item.passed ? "PASS" : "FAIL"} ${item.name}`)} />;
    case "Replay":
      return <Rows rows={[`status ${showcase.replay.status}`, `compared runs ${showcase.replay.compared_runs}`, showcase.replay.reason ?? "no replay drift"]} />;
    case "Conformance":
      return <Rows rows={[`contract ${showcase.contract}`, `seeded bug ${showcase.seeded_bug.status}`, showcase.seeded_bug.reason, `issue ${showcase.seeded_bug.issue}`]} />;
    case "Cost":
      return <Rows rows={traceCostRows(trace)} />;
  }
}

function TransactionRows({ steps, active }: { steps: ShowcaseStep[]; active: string }) {
  return (
    <div className="trace-list">
      {steps.map((step) => {
        const summary = summarizeStep(step);
        return (
          <div className={step.name === active ? "trace-card active" : "trace-card"} key={step.name}>
            <strong>{step.name}</strong>
            <span>tx {summary.tx} · rev {summary.revision} · {summary.resources} commands · {summary.frames} frames</span>
          </div>
        );
      })}
    </div>
  );
}

function GraphRows({ trace }: { trace: ContractTrace }) {
  return (
    <Rows
      rows={[
        `changed inputs ${trace.changed_inputs.join(",") || "none"}`,
        `dirty roots ${trace.dirty_roots.join(",") || "none"}`,
        `derived recomputed ${trace.recomputed_derived_nodes.join(",") || "none"}`,
        `collections recomputed ${trace.recomputed_collection_nodes.join(",") || "none"}`,
        `phase trace ${trace.phase_trace.join(" -> ")}`,
      ]}
    />
  );
}

function ResourceRows({
  commands,
  setSelection,
}: {
  commands: ResourceCommand[];
  setSelection: (selection: TraceSelection) => void;
}) {
  return (
    <div className="trace-list">
      {commands.map((command, index) => (
        <button className="trace-card interactive" key={`${command.kind}:${index}`} onClick={() => setSelection({ type: "resource", index })}>
          <strong>{command.kind} {resourceKeyLabel(command.key)}</strong>
          <span>scope {command.scope} · policy {command.transition_policy}</span>
        </button>
      ))}
      {commands.length === 0 && <div className="empty padded">No resource commands for this transaction.</div>}
    </div>
  );
}

function FrameRows({
  frames,
  setSelection,
}: {
  frames: OutputFrame[];
  setSelection: (selection: TraceSelection) => void;
}) {
  return (
    <div className="trace-list">
      {frames.map((frame, index) => (
        <button className="trace-card interactive" key={`${frame.output_key}:${index}`} onClick={() => setSelection({ type: "frame", index })}>
          <strong>{outputFrameKind(frame.kind)} output {frame.output_key}</strong>
          <span>scope {frame.scope} · rev {frame.revision}</span>
        </button>
      ))}
      {frames.length === 0 && <div className="empty padded">No output frames for this transaction.</div>}
    </div>
  );
}

function ScopeRows({
  events,
  setSelection,
}: {
  events: ScopeEvent[];
  setSelection: (selection: TraceSelection) => void;
}) {
  return (
    <div className="trace-list">
      {events.map((event, index) => (
        <button className="trace-card interactive" key={`${event.scope}:${index}`} onClick={() => setSelection({ type: "scope", index })}>
          <strong>{event.kind} scope {event.scope}</strong>
          <span>scope lifecycle event</span>
        </button>
      ))}
      {events.length === 0 && <div className="empty padded">No scope lifecycle events for this transaction.</div>}
    </div>
  );
}

function HostRows({
  step,
  setSelection,
}: {
  step: ShowcaseStep;
  setSelection: (selection: TraceSelection) => void;
}) {
  return (
    <div className="trace-list">
      {step.host_statuses.map((status, index) => (
        <button className="trace-card interactive" key={`${status.target}:${index}`} onClick={() => setSelection({ type: "host", index })}>
          <strong>{status.target}</strong>
          <span>{status.status} · command rev {status.command_revision ?? "n/a"}</span>
        </button>
      ))}
      {step.host_statuses.length === 0 && <div className="empty padded">No host statuses for this step.</div>}
    </div>
  );
}

function MetricStrip({ step }: { step: ShowcaseStep }) {
  const summary = summarizeStep(step);
  return (
    <div className="trace-metrics">
      <span>rev {summary.revision}</span>
      <span>{summary.diffs} diffs</span>
      <span>{summary.resources} resources</span>
      <span>{summary.frames} frames</span>
      <span>{summary.hosts} hosts</span>
      <span>{summary.failedInvariants} fails</span>
    </div>
  );
}

function StepStats({ step }: { step: ShowcaseStep }) {
  const summary = summarizeStep(step);
  return <span>tx {summary.tx} · rev {summary.revision} · {summary.resources}/{summary.frames}</span>;
}

function Rows({ rows }: { rows: string[] }) {
  if (rows.length === 0) return <div className="empty padded">No entries</div>;
  return <div className="trace-list">{rows.map((row) => <div className="trace-card" key={row}>{row}</div>)}</div>;
}

function diffRow(diff: CollectionDiff) {
  return `node ${diff.node} ${diff.kind}: +${diff.added} -${diff.removed} ~${diff.updated} unchanged ${diff.unchanged}`;
}
