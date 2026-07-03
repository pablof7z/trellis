const FORMAT_VERSION = 1;
const traces = [
  ["normal-session", "Normal session", "/demos/flight-recorder/traces/normal-session.json"],
  ["seeded-leak", "Seeded leak", "/demos/flight-recorder/traces/seeded-leak.json"],
  ["teardown-cascade", "Teardown cascade", "/demos/flight-recorder/traces/teardown-cascade.json"],
];

const els = {
  select: document.querySelector("[data-trace-select]"),
  file: document.querySelector("[data-file]"),
  search: document.querySelector("[data-search]"),
  dropZone: document.querySelector("[data-drop-zone]"),
  status: document.querySelector("[data-status]"),
  scrubber: document.querySelector("[data-scrubber]"),
  position: document.querySelector("[data-position]"),
  timeline: document.querySelector("[data-timeline]"),
  transaction: document.querySelector("[data-transaction]"),
  receipt: document.querySelector("[data-receipt]"),
  diff: document.querySelector("[data-diff]"),
  selectedCommand: document.querySelector("[data-selected-command]"),
  version: document.querySelector("[data-version]"),
};

let traceFile;
let activeIndex = 0;
let selectedCommand = 0;

window.trellisDemoEvents = window.trellisDemoEvents ?? [];

function boot() {
  els.select.replaceChildren(
    ...traces.map(([id, label]) => new Option(label, id))
  );
  els.select.addEventListener("change", () => loadBundled(els.select.value));
  els.file.addEventListener("change", loadPickedFile);
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
  els.search.addEventListener("input", render);
  els.scrubber.addEventListener("input", () => {
    activeIndex = Number(els.scrubber.value);
    selectedCommand = 0;
    track("trace-scrub", { activeIndex });
    render();
  });
  loadBundled("normal-session");
}

async function loadBundled(id) {
  const trace = traces.find(([traceId]) => traceId === id) ?? traces[0];
  const response = await fetch(trace[2]);
  loadTrace(await response.json(), trace[1]);
}

async function loadPickedFile() {
  const file = els.file.files?.[0];
  if (!file) return;
  loadTraceFile(file);
}

async function loadTraceFile(file) {
  try {
    const loaded = loadTrace(JSON.parse(await file.text()), file.name);
    track(loaded ? "trace-file-load" : "trace-file-reject", { name: file.name });
  } catch (error) {
    setError(`could not read ${file.name}`, error.message);
  }
}

function loadTrace(nextTrace, label) {
  const error = validateTrace(nextTrace);
  if (error) {
    setError(label, error);
    return false;
  }
  traceFile = nextTrace;
  activeIndex = 0;
  selectedCommand = 0;
  els.scrubber.max = String(traceFile.steps.length - 1);
  els.version.textContent = `v${traceFile.formatVersion}`;
  setStatus(label, `${traceFile.steps.length} transactions loaded`);
  render();
  return true;
}

function validateTrace(candidate) {
  if (candidate?.formatVersion !== FORMAT_VERSION) {
    return `unsupported trace format ${candidate?.formatVersion ?? "missing"}; expected ${FORMAT_VERSION}`;
  }
  if (!Array.isArray(candidate.steps) || candidate.steps.length === 0) {
    return "trace file has no steps";
  }
  return null;
}

function render() {
  if (!traceFile) return;
  const step = traceFile.steps[activeIndex];
  const commands = filteredCommands(step);
  selectedCommand = Math.min(selectedCommand, Math.max(commands.length - 1, 0));
  els.scrubber.value = String(activeIndex);
  els.position.textContent = `${activeIndex + 1} / ${traceFile.steps.length}`;
  renderTimeline();
  renderTransaction(step, commands);
  renderReceipt(step, commands[selectedCommand]);
  renderDiff(step, traceFile.steps[activeIndex - 1]);
}

function renderTimeline() {
  els.timeline.replaceChildren(
    ...traceFile.steps.map((step, index) => {
      const button = document.createElement("button");
      const failed = step.trace.invariant_results?.some((check) => !check.passed);
      button.className = `timeline-step ${index === activeIndex ? "selected" : ""} ${failed ? "leak" : ""}`;
      button.type = "button";
      button.innerHTML = `<strong>${escapeHtml(step.name)}</strong><span>tx ${step.trace.transaction_id} / revision ${step.trace.revision}</span>`;
      button.addEventListener("click", () => {
        activeIndex = index;
        selectedCommand = 0;
        track("timeline-click", { activeIndex });
        render();
      });
      return button;
    })
  );
}

function renderTransaction(step, commands) {
  const trace = step.trace;
  els.transaction.replaceChildren(
    fact("Changed input nodes", trace.changed_inputs.join(", ") || "none"),
    fact("Collection diffs", summarizeDiffs(trace.collection_diffs)),
    fact("Invariant checks", summarizeInvariants(trace.invariant_results)),
    ...commands.map((command, index) => commandButton(command, index))
  );
}

function commandButton(command, index) {
  const button = document.createElement("button");
  button.className = `command-row ${index === selectedCommand ? "selected" : ""}`;
  button.dataset.kind = command.kind;
  button.type = "button";
  button.innerHTML = `<strong>${formatKind(command.kind)}</strong><span>${escapeHtml(command.key)}</span><span>scope ${command.scope}</span>`;
  button.addEventListener("click", () => {
    selectedCommand = index;
    track("command-click", { kind: command.kind, key: command.key });
    render();
  });
  return button;
}

function renderReceipt(step, command) {
  if (!command) {
    els.selectedCommand.textContent = "no command";
    els.receipt.replaceChildren(fact("No command selected", "Change the search or choose a transaction with commands."));
    return;
  }
  els.selectedCommand.textContent = `${formatKind(command.kind)} ${command.key}`;
  els.receipt.replaceChildren(
    receiptStep("input", `transaction ${step.trace.transaction_id} changed input nodes ${step.trace.changed_inputs.join(", ") || "none"}`),
    receiptStep("derived nodes", `collections recomputed: ${step.trace.recomputed_collection_nodes.join(", ") || "none"}`),
    receiptStep("diff", summarizeDiffs(step.trace.collection_diffs)),
    receiptStep("command", `${formatKind(command.kind)} ${command.key} in scope ${command.scope}`)
  );
}

function renderDiff(step, previous) {
  const currentKeys = keysFor(step);
  const previousKeys = previous ? keysFor(previous) : new Set();
  const added = [...currentKeys].filter((key) => !previousKeys.has(key));
  const removed = [...previousKeys].filter((key) => !currentKeys.has(key));
  els.diff.replaceChildren(
    diffRow("added command keys", added),
    diffRow("removed command keys", removed),
    diffRow("phase count", [`${step.trace.phase_trace.length} phases`])
  );
}

function filteredCommands(step) {
  const query = els.search.value.trim().toLowerCase();
  const commands = step.trace.resource_commands ?? [];
  if (!query) return commands;
  return commands.filter((command) => command.key.toLowerCase().includes(query));
}

function keysFor(step) {
  return new Set((step.trace.resource_commands ?? []).map((command) => `${command.kind}:${command.key}`));
}

function summarizeDiffs(diffs = []) {
  if (!diffs.length) return "no structural diff";
  return diffs.map((diff) => `node ${diff.node}: +${diff.added} -${diff.removed} =${diff.unchanged}`).join("; ");
}

function summarizeInvariants(checks = []) {
  if (!checks.length) return "none recorded";
  return checks.map((check) => `${check.passed ? "pass" : "fail"}: ${check.name}`).join("; ");
}

function formatKind(kind) {
  return `[${kind.toUpperCase()}]`;
}

function fact(label, detail) {
  const node = document.createElement("div");
  node.className = "trace-fact";
  node.append(el("strong", label), el("span", detail));
  return node;
}

function receiptStep(label, detail) {
  const node = document.createElement("div");
  node.className = "receipt-step";
  node.append(el("strong", label), el("span", detail));
  return node;
}

function diffRow(label, values) {
  const node = document.createElement("div");
  node.className = "diff-row";
  node.append(el("strong", label), el("span", values.length ? values.join(", ") : "none"));
  return node;
}

function setStatus(label, detail) {
  els.status.classList.remove("error");
  els.status.replaceChildren(el("strong", label), el("span", detail));
}

function setError(label, detail) {
  traceFile = null;
  els.status.classList.add("error");
  els.status.replaceChildren(el("strong", label), el("span", detail));
  track("trace-load-error", { label, detail });
}

function el(tag, text) {
  const node = document.createElement(tag);
  node.textContent = text;
  return node;
}

function escapeHtml(value) {
  return String(value).replace(/[&<>"']/g, (char) => ({
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    '"': "&quot;",
    "'": "&#039;",
  })[char]);
}

function track(name, payload = {}) {
  window.trellisDemoEvents.push({ name, payload, at: Date.now() });
}

boot();
