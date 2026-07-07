import { evidenceLabel, invariantMatches, itemMatchesQuery, kindMatches, replayStatus } from "./trace-model.js";
import { labelSummary } from "./trace-labels.js";

const tabs = ["resources", "outputs", "scopes", "invariants", "replay", "raw"];

export function renderAll(state, els, handlers) {
  if (!state.trace) return;
  renderHeader(state, els);
  renderTimeline(state, els, handlers);
  renderPhaseLadder(state.trace.steps[state.activeIndex], els);
  renderTransaction(state, els, handlers);
  renderReceipt(state, els);
  renderLedger(state, els, handlers);
}

export function renderLoading(els, label, detail) {
  els.status.className = "status-line loading";
  els.status.replaceChildren(el("strong", label), el("span", detail));
}

export function renderError(els, label, errors) {
  els.status.className = "status-line error";
  const list = document.createElement("ul");
  for (const error of errors) list.append(el("li", error));
  els.status.replaceChildren(el("strong", label), list);
}

export function renderLoadedStatus(els, label, detail) {
  els.status.className = "status-line loaded";
  els.status.replaceChildren(el("strong", label), el("span", detail));
}

function renderHeader(state, els) {
  const { trace } = state;
  const replay = replayStatus(trace);
  els.sourceBadge.textContent = trace.provenance.sourceLabel;
  els.sourceBadge.dataset.source = trace.provenance.sourceType;
  els.traceTitle.textContent = trace.title;
  els.traceSubtitle.textContent = `${trace.steps.length} transactions; ${trace.invariantSummary.label}; ${replay.label}`;
  els.manifest.replaceChildren(
    manifest("format", `v${trace.formatVersion}`),
    manifest("source", trace.provenance.sourceLabel),
    manifest("generator", trace.provenance.generator),
    manifest("commit", trace.provenance.repoCommit),
    manifest("build", trace.provenance.buildId),
    manifest("core-backed", trace.provenance.coreBacked ? "yes" : "no"),
    manifest("transactions", String(trace.steps.length)),
    manifest("labels", labelSummary(trace.labelRegistry)),
    manifest("invariants", trace.invariantSummary.label),
    manifest("replay", replay.status)
  );
}

function renderTimeline(state, els, handlers) {
  els.scrubber.max = String(state.trace.steps.length - 1);
  els.scrubber.value = String(state.activeIndex);
  els.position.textContent = `${state.activeIndex + 1} / ${state.trace.steps.length}`;
  els.timeline.replaceChildren(
    ...state.trace.steps.map((step, index) => {
      const failed = step.invariantResults.some((check) => !check.passed);
      const button = el("button", "", "timeline-step");
      button.type = "button";
      button.dataset.timelineIndex = String(index);
      button.classList.toggle("selected", index === state.activeIndex);
      button.classList.toggle("failed", failed);
      button.append(
        row("strong", `tx ${step.txId} / rev ${step.revision}`),
        row("span", step.name),
        row("small", `${step.changedInputs.length} inputs | ${step.resourceCommands.length} commands | ${step.outputFrames.length} frames`)
      );
      button.addEventListener("click", () => handlers.selectTransaction(index));
      return button;
    })
  );
}

function renderPhaseLadder(step, els) {
  els.phases.replaceChildren(
    ...step.phaseTrace.map((phase, index) => {
      const node = el("span", `${String(index + 1).padStart(2, "0")} ${phase}`);
      node.className = "phase-node";
      return node;
    })
  );
}

function renderTransaction(state, els, handlers) {
  const step = state.trace.steps[state.activeIndex];
  const query = state.query.toLowerCase();
  const commands = indexed(step.resourceCommands, (command) => {
    return kindMatches(command, state.kindFilter) && itemMatchesQuery(command, query);
  });
  const frames = indexed(step.outputFrames, (frame) => itemMatchesQuery(frame, query));
  const invariants = indexed(step.invariantResults, (check) => {
    return invariantMatches(check, state.invariantFilter) && itemMatchesQuery(check, query);
  });
  els.transaction.replaceChildren(
    section("Changed Inputs", step.changedInputs.map((input) => line(input)), "none"),
    section("Collection Diffs", step.collectionDiffs.map(diffLine), "no structural diff"),
    evidenceSection("Resource Commands", commands, "no commands after filter", "command", handlers),
    evidenceSection("Output Frames", frames, "no output frames recorded", "frame", handlers),
    section("Scope Lifecycle", step.scopeEvents.map(scopeLine), "no scope lifecycle events recorded"),
    section("Audit Log", step.auditLog.map((entry) => codeLine(entry)), "no audit log entries recorded"),
    evidenceSection("Invariants", invariants, "no invariants after filter", "invariant", handlers)
  );
}

function renderReceipt(state, els) {
  const step = state.trace.steps[state.activeIndex];
  const selected = selectedEvidence(step, state.selection);
  els.selectedEvidence.textContent = evidenceLabel(state.selection);
  els.receipt.replaceChildren(
    fact("tx / revision", `tx ${step.txId} / rev ${step.revision}`),
    fact("phase", selected.phase),
    fact("owner scope", selected.scope),
    fact("triggering input", step.changedInputs.join(", ") || "none"),
    fact("structural diff", summarizeDiffs(step.collectionDiffs)),
    fact("cause chain", selected.cause),
    rawBlock(selected.raw)
  );
}

function renderLedger(state, els, handlers) {
  const step = state.trace.steps[state.activeIndex];
  els.ledgerTabs.replaceChildren(
    ...tabs.map((tab) => {
      const button = el("button", tab);
      button.type = "button";
      button.role = "tab";
      button.ariaSelected = String(state.ledgerTab === tab);
      button.classList.toggle("selected", state.ledgerTab === tab);
      button.addEventListener("click", () => handlers.selectLedgerTab(tab));
      return button;
    })
  );
  const renderers = {
    resources: () => table(["kind", "policy", "key", "scope"], step.resourceCommands.map((cmd) => [kindBadge(cmd.kind), cmd.transitionPolicy, cmd.key, cmd.scope])),
    outputs: () => step.outputFrames.length ? table(["kind", "key", "scope"], step.outputFrames.map((frame) => [frame.kind, frame.key, frame.scope])) : empty("no output frames recorded"),
    scopes: () => stack([section("Scope Events", step.scopeEvents.map(scopeLine), "none"), section("Audit Log", step.auditLog.map(codeLine), "none")]),
    invariants: () => table(["status", "name", "details"], step.invariantResults.map((check) => [check.passed ? "pass" : "fail", check.name, check.details || "-"])),
    replay: () => replayPanel(state.trace),
    raw: () => rawBlock(step.raw, true),
  };
  els.ledgerPanel.replaceChildren(renderers[state.ledgerTab]());
}

function selectedEvidence(step, selection) {
  if (selection?.type === "command") return commandReceipt(step, step.resourceCommands[selection.index]);
  if (selection?.type === "frame") return frameReceipt(step, step.outputFrames[selection.index]);
  if (selection?.type === "invariant") return invariantReceipt(step, step.invariantResults[selection.index]);
  return {
    phase: step.phaseTrace.at(-1) ?? "unknown",
    scope: "transaction",
    cause: "changed input -> recompute -> structural diff -> receipt",
    raw: step.raw,
  };
}

function commandReceipt(step, command) {
  return {
    phase: "ProduceResourcePlans",
    scope: command?.scope ?? "unknown",
    cause: command?.cause ? causeText(command.cause) : `${step.changedInputs.join(", ") || "input"} -> ${summarizeDiffs(step.collectionDiffs)} -> ${kindBadge(command?.kind)} ${command?.key}`,
    raw: command?.raw ?? {},
  };
}

function frameReceipt(step, frame) {
  return {
    phase: "ProduceOutputFrames",
    scope: frame?.scope ?? "unknown",
    cause: frame?.cause ? causeText(frame.cause) : `${step.changedInputs.join(", ") || "input"} -> output ${frame?.kind ?? "frame"} ${frame?.key ?? ""}`,
    raw: frame?.raw ?? {},
  };
}

function invariantReceipt(step, check) {
  return {
    phase: "ReturnTransactionResult",
    scope: "trace invariant",
    cause: `${step.changedInputs.join(", ") || "input"} -> invariant ${check?.passed ? "passed" : "failed"}: ${check?.name ?? "unknown"}`,
    raw: check?.raw ?? {},
  };
}

function evidenceSection(title, items, emptyText, type, handlers) {
  return section(title, items.map(({ item, index }) => {
    const commandKind = type === "command" ? ` ${item.kind.toLowerCase()}` : "";
    const button = el("button", "", `evidence-row ${type}${commandKind}`);
    button.type = "button";
    button.append(row("strong", evidenceTitle(type, item)), row("span", evidenceDetail(type, item)));
    button.addEventListener("click", () => handlers.selectEvidence(type, index));
    return button;
  }), emptyText);
}

function evidenceTitle(type, item) {
  if (type === "command") return `${kindBadge(item.kind)} ${item.key}`;
  if (type === "frame") return `${item.kind} ${item.key}`;
  return `${item.passed ? "pass" : "fail"} ${item.name}`;
}
function evidenceDetail(type, item) {
  if (type === "command" || type === "frame") return `scope ${item.scope}`;
  return item.details || "recorded invariant result";
}
const indexed = (items, predicate) => items.map((item, index) => ({ item, index })).filter(({ item }) => predicate(item));

function section(title, rows, emptyText) {
  const node = el("section", "", "evidence-section");
  node.append(el("h3", title), rows.length ? stack(rows) : empty(emptyText));
  return node;
}

function table(headings, rows) {
  const tableNode = el("table", "", "ledger-table");
  tableNode.append(tr(headings, "th"), ...rows.map((cells) => tr(cells, "td")));
  return tableNode;
}

function replayPanel(trace) {
  const replay = replayStatus(trace);
  const rows = [
    fact("status", replay.status),
    fact("labels", labelSummary(trace.labelRegistry)),
  ];
  if (replay.checks?.length) rows.push(...replay.checks.map((check) => fact(check.label, check.status)));
  if (trace.provenance.replay?.reason) rows.push(fact("reason", trace.provenance.replay.reason));
  return stack(rows);
}

function manifest(label, value) {
  const node = el("div");
  node.append(el("dt", label), el("dd", value));
  return node;
}

function fact(label, value) {
  const node = el("div", "", "receipt-fact");
  node.append(el("strong", label), el("span", String(value || "none")));
  return node;
}

function rawBlock(value, expanded = false) {
  const details = el("details", "", "raw-block");
  details.open = expanded;
  details.append(el("summary", "raw JSON excerpt"), el("pre", JSON.stringify(value, null, 2)));
  return details;
}

function diffLine(diff) {
  return line(`${diff.node}: +${formatDiffValue(diff.added)} -${formatDiffValue(diff.removed)} ~${formatDiffValue(diff.updated)} =${diff.unchanged}`);
}

function scopeLine(event) {
  return line(`${event.kind} ${event.scope}${event.reason ? `: ${event.reason}` : ""}`);
}

function codeLine(value) {
  return line(typeof value === "string" ? value : JSON.stringify(value));
}

function line(text) { return el("div", String(text), "ledger-line"); }

function stack(nodes) {
  const node = el("div", "", "ledger-stack");
  node.append(...nodes);
  return node;
}

function empty(text) { return el("p", text, "empty-state"); }
function row(tag, text) { return el(tag, String(text)); }

function tr(cells, cellTag) {
  const rowNode = document.createElement("tr");
  rowNode.append(...cells.map((cell) => el(cellTag, String(cell))));
  return rowNode;
}
function kindBadge(kind = "unknown") { return `[${String(kind).toUpperCase()}]`; }

function summarizeDiffs(diffs) {
  return diffs.length ? diffs.map((diff) => `${diff.node} +${formatDiffValue(diff.added)} -${formatDiffValue(diff.removed)}`).join("; ") : "no structural diff";
}
function causeText(cause) {
  return `${cause.inputKey ?? "input"} -> ${cause.collection ?? "collection"} -> ${cause.reason ?? "receipt"}`;
}

function formatDiffValue(value) { return Array.isArray(value) ? value.length : value; }

function el(tag, text = "", className = "") {
  const node = document.createElement(tag);
  if (text) node.textContent = text;
  if (className) node.className = className;
  return node;
}
