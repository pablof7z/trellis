import { useState } from "react";
import {
  actionLabel,
  actionMeta,
  diffRows,
  domainRows,
  hostRows,
  invariantRows,
  labShowcases,
  lifecycleSummary,
  outputRows,
  resourceRows,
  stepAt,
  type LabShowcase,
  type LifecycleRow,
} from "./showcaseLabModel";

export function ShowcaseLab() {
  const [showcaseIndex, setShowcaseIndex] = useState(0);
  const [stepIndex, setStepIndex] = useState(0);
  const showcase = labShowcases[showcaseIndex];
  const step = stepAt(showcase, stepIndex);
  const summary = lifecycleSummary(step);

  const chooseShowcase = (index: number) => {
    setShowcaseIndex(index);
    setStepIndex(0);
  };

  return (
    <section className="showcase-lab">
      <aside className="lab-sidebar panel">
        <span className="eyebrow">Interactive showcases</span>
        <div className="lab-showcase-list">
          {labShowcases.map((item, index) => (
            <button
              className={index === showcaseIndex ? "lab-showcase active" : "lab-showcase"}
              key={item.trace.showcase}
              onClick={() => chooseShowcase(index)}
            >
              <strong>{item.title}</strong>
              <span>{item.trace.script}</span>
            </button>
          ))}
        </div>
        <div className="lab-command">
          <span className="eyebrow">Headless source</span>
          <p>{showcase.trace.command.join(" ")}</p>
        </div>
      </aside>

      <section className="lab-demo panel">
        <header className="lab-head">
          <div>
            <span className="eyebrow">{showcase.actionNoun}</span>
            <h2>{showcase.title}</h2>
            <p>{showcase.summary}</p>
          </div>
          <div className="lab-metrics">
            <span>{summary.opens} opens</span>
            <span>{summary.closes} closes</span>
            <span>{summary.frames} frames</span>
            <span>{summary.hosts} hosts</span>
            <span>{summary.invariants} oracle pass</span>
          </div>
        </header>

        <div className="lab-actions">
          {showcase.trace.steps.map((item, index) => (
            <button className={index === stepIndex ? "active" : ""} key={item.name} onClick={() => setStepIndex(index)}>
              <strong>{actionLabel(item)}</strong>
              <span>{actionMeta(item)}</span>
            </button>
          ))}
        </div>

        <DomainSurface showcase={showcase} stepIndex={stepIndex} />

        <div className="lab-lifecycle">
          <LabPanel title="Resource lifecycle" rows={resourceRows(step)} empty="No resource commands for this action." />
          <LabPanel title="Output frames" rows={outputRows(step)} empty="No materialized output frames for this action." />
          <LabPanel title="Collection diffs" rows={diffRows(step)} empty="No collection diffs for this action." />
        </div>
      </section>

      <aside className="lab-observatory panel">
        <span className="eyebrow">Observatory</span>
        <h2>{step.name}</h2>
        <div className="lab-trace-id">
          <span>tx {step.trace.transaction_id}</span>
          <span>rev {step.trace.revision}</span>
          <span>{step.trace.phase_trace.length} phases</span>
        </div>
        <LabPanel title="Invariant results" rows={invariantRows(step)} empty="No invariant results recorded." />
        <LabPanel title="Host statuses" rows={hostRows(step)} empty="No host statuses for this action." />
        <div className="lab-status-block">
          <span className="eyebrow">Replay</span>
          <p>{showcase.trace.replay.status} across {showcase.trace.replay.compared_runs} runs</p>
          <span className="eyebrow">Seeded bug</span>
          <p>{showcase.trace.seeded_bug.status} · {showcase.trace.seeded_bug.issue}</p>
        </div>
      </aside>
    </section>
  );
}

function DomainSurface({ showcase, stepIndex }: { showcase: LabShowcase; stepIndex: number }) {
  const step = stepAt(showcase, stepIndex);
  const rows = domainRows(showcase, step);
  return (
    <section className={`lab-surface ${showcase.kind}`}>
      <div className="lab-surface-head">
        <span>{showcase.trace.showcase}</span>
        <strong>{step.name}</strong>
      </div>
      <div className="lab-domain-grid">
        {rows.map((row) => (
          <LifecycleItem key={`${row.label}:${row.value}:${row.meta}`} row={row} />
        ))}
      </div>
    </section>
  );
}

function LabPanel({ title, rows, empty }: { title: string; rows: LifecycleRow[]; empty: string }) {
  return (
    <section className="lab-panel">
      <h3>{title}</h3>
      <div className="lab-row-list">
        {rows.length === 0 ? <div className="empty">{empty}</div> : rows.map((row) => <LifecycleItem key={`${row.label}:${row.meta}`} row={row} />)}
      </div>
    </section>
  );
}

function LifecycleItem({ row }: { row: LifecycleRow }) {
  return (
    <div className={`lab-row ${row.tone}`}>
      <strong>{row.label}</strong>
      <span>{row.value}</span>
      <small>{row.meta}</small>
    </div>
  );
}
