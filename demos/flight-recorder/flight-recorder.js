import {
  bundledTraces,
  normalizeTraceEnvelope,
} from "./trace-model.js";
import {
  renderAll,
  renderError,
  renderLoadedStatus,
  renderLoading,
} from "./trace-render.js";

const els = {
  select: document.querySelector("[data-trace-select]"),
  file: document.querySelector("[data-file]"),
  search: document.querySelector("[data-search]"),
  kindFilter: document.querySelector("[data-kind-filter]"),
  invariantFilter: document.querySelector("[data-invariant-filter]"),
  dropZone: document.querySelector("[data-drop-zone]"),
  sourceBadge: document.querySelector("[data-source-badge]"),
  traceTitle: document.querySelector("[data-trace-title]"),
  traceSubtitle: document.querySelector("[data-trace-subtitle]"),
  manifest: document.querySelector("[data-manifest]"),
  status: document.querySelector("[data-status]"),
  scrubber: document.querySelector("[data-scrubber]"),
  position: document.querySelector("[data-position]"),
  timeline: document.querySelector("[data-timeline]"),
  phases: document.querySelector("[data-phases]"),
  transaction: document.querySelector("[data-transaction]"),
  receipt: document.querySelector("[data-receipt]"),
  selectedEvidence: document.querySelector("[data-selected-evidence]"),
  ledgerTabs: document.querySelector("[data-ledger-tabs]"),
  ledgerPanel: document.querySelector("[data-ledger-panel]"),
  liveStatus: document.querySelector("[data-live-status]"),
  copyProof: document.querySelector("[data-copy-proof]"),
};

const state = {
  trace: null,
  activeIndex: 0,
  selection: { type: "transaction", index: 0 },
  ledgerTab: "resources",
  query: "",
  kindFilter: "all",
  invariantFilter: "all",
};

window.trellisDemoEvents = window.trellisDemoEvents ?? [];

const handlers = {
  selectTransaction(index) {
    state.activeIndex = clamp(index, 0, state.trace.steps.length - 1);
    state.selection = { type: "transaction", index: state.activeIndex };
    track("timeline-select", { activeIndex: state.activeIndex });
    render();
  },
  selectEvidence(type, index) {
    state.selection = { type, index };
    track("evidence-select", { type, index, activeIndex: state.activeIndex });
    render();
  },
  selectLedgerTab(tab) {
    state.ledgerTab = tab;
    track("ledger-tab-select", { tab });
    render();
  },
};

function boot() {
  els.select.replaceChildren(
    ...bundledTraces.map(([id, label]) => new Option(label, id))
  );
  els.select.addEventListener("change", () => loadBundled(els.select.value));
  els.file.addEventListener("change", loadPickedFile);
  els.search.addEventListener("input", () => updateFilter("query", els.search.value));
  els.kindFilter.addEventListener("change", () => updateFilter("kindFilter", els.kindFilter.value));
  els.invariantFilter.addEventListener("change", () => updateFilter("invariantFilter", els.invariantFilter.value));
  els.scrubber.addEventListener("input", () => handlers.selectTransaction(Number(els.scrubber.value)));
  els.copyProof.addEventListener("click", copyProofPacket);
  wireDropZone();
  wireKeyboard();
  loadBundled("normal-session");
}

async function loadBundled(id) {
  const trace = bundledTraces.find(([traceId]) => traceId === id) ?? bundledTraces[0];
  renderLoading(els, "loading bundled trace", trace[1]);
  try {
    const response = await fetch(trace[2]);
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    loadTrace(await response.json(), { kind: "bundled", label: trace[1] });
  } catch (error) {
    clearTrace();
    renderError(els, "bundled trace failed", [error.message]);
  }
}

async function loadPickedFile() {
  const file = els.file.files?.[0];
  if (!file) return;
  await loadTraceFile(file);
}

async function loadTraceFile(file) {
  renderLoading(els, "loading uploaded trace", file.name);
  try {
    loadTrace(JSON.parse(await file.text()), { kind: "uploaded", label: file.name });
    track("trace-file-load", { name: file.name });
  } catch (error) {
    clearTrace();
    const detail = error instanceof SyntaxError
      ? [`invalid JSON: ${error.message}`]
      : [error.message];
    renderError(els, "uploaded trace rejected", detail);
    track("trace-file-reject", { name: file.name, error: detail.join("; ") });
  }
}

function loadTrace(rawTrace, source) {
  const { errors, trace } = normalizeTraceEnvelope(rawTrace, source);
  if (errors.length) {
    clearTrace();
    renderError(els, source.kind === "uploaded" ? "uploaded trace rejected" : "bundled trace rejected", errors);
    track("trace-load-error", { source: source.kind, errors });
    return false;
  }
  state.trace = trace;
  state.activeIndex = 0;
  state.selection = { type: "transaction", index: 0 };
  state.ledgerTab = "resources";
  renderLoadedStatus(els, trace.provenance.sourceLabel, `${trace.steps.length} transactions loaded`);
  render();
  return true;
}

function render() {
  renderAll(state, els, handlers);
}

function updateFilter(key, value) {
  state[key] = value;
  state.selection = { type: "transaction", index: state.activeIndex };
  track("filter-change", { key, value });
  render();
}

function wireDropZone() {
  els.dropZone.addEventListener("dragover", (event) => {
    event.preventDefault();
    els.dropZone.classList.add("dragging");
  });
  els.dropZone.addEventListener("dragleave", () => {
    els.dropZone.classList.remove("dragging");
  });
  els.dropZone.addEventListener("drop", (event) => {
    event.preventDefault();
    els.dropZone.classList.remove("dragging");
    const file = event.dataTransfer?.files?.[0];
    if (file) loadTraceFile(file);
  });
}

function wireKeyboard() {
  document.addEventListener("keydown", (event) => {
    if (event.key === "/" && !isTextInput(event.target)) {
      event.preventDefault();
      els.search.focus();
      return;
    }
    if (!state.trace || isTextInput(event.target)) return;
    if (event.key === "ArrowLeft") {
      event.preventDefault();
      handlers.selectTransaction(state.activeIndex - 1);
    }
    if (event.key === "ArrowRight") {
      event.preventDefault();
      handlers.selectTransaction(state.activeIndex + 1);
    }
    if (event.key === "Enter" && event.target?.dataset?.timelineIndex) {
      event.preventDefault();
      handlers.selectTransaction(Number(event.target.dataset.timelineIndex));
    }
  });
}

async function copyProofPacket() {
  if (!state.trace) return;
  const step = state.trace.steps[state.activeIndex];
  const packet = {
    issue: "#157",
    source: state.trace.provenance,
    selectedTransaction: {
      name: step.name,
      txId: step.txId,
      revision: step.revision,
      phaseTrace: step.phaseTrace,
      changedInputs: step.changedInputs,
      collectionDiffs: step.collectionDiffs.map((diff) => diff.raw),
      resourceCommands: step.resourceCommands.map((command) => command.raw),
      outputFrames: step.outputFrames.map((frame) => frame.raw),
      scopeEvents: step.scopeEvents.map((event) => event.raw),
      auditLog: step.auditLog,
      invariantResults: step.invariantResults.map((check) => check.raw),
    },
    selection: state.selection,
  };
  await writeClipboard(JSON.stringify(packet, null, 2));
  renderLoadedStatus(els, "proof packet copied", `tx ${step.txId} / rev ${step.revision}`);
  track("proof-packet-copy", { txId: step.txId, revision: step.revision });
}

function clearTrace() {
  state.trace = null;
  els.sourceBadge.textContent = "[NO TRACE]";
  els.sourceBadge.dataset.source = "none";
  els.traceTitle.textContent = "Flight Recorder";
  els.traceSubtitle.textContent = "No valid trace loaded.";
  els.manifest.replaceChildren();
  els.timeline.replaceChildren();
  els.phases.replaceChildren();
  els.transaction.replaceChildren();
  els.receipt.replaceChildren();
  els.ledgerTabs.replaceChildren();
  els.ledgerPanel.replaceChildren();
}

function isTextInput(target) {
  return ["INPUT", "TEXTAREA", "SELECT"].includes(target?.tagName);
}

function clamp(value, min, max) {
  return Math.max(min, Math.min(max, value));
}

async function writeClipboard(text) {
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(text);
    return;
  }
  const textarea = document.createElement("textarea");
  textarea.value = text;
  document.body.append(textarea);
  textarea.select();
  document.execCommand("copy");
  textarea.remove();
}

function track(name, payload = {}) {
  window.trellisDemoEvents.push({ name, payload, at: Date.now() });
}

boot();
