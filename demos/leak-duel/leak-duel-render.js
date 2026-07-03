import { badge, commandKind, el, fact, shortKey, signed } from "./leak-duel-dom.js";

export function renderAll(els, state, view) {
  renderHeader(state, view);
  renderCounters(state);
  renderTimeline(els.timeline, state.activity, state.tick);
  renderRows(els.rows, state.rows, view.selected);
  renderReceipt(els.receipt, state);
  renderInputs(els.inputs, state.inputs, state.shouldOpen);
  renderProof(els.proof, state, view);
  renderCommands(els.commands, state.proof.currentCommands);
  els.raw.textContent = JSON.stringify(state, null, 2);
}

function renderHeader(state, view) {
  document.querySelector("[data-engine-state]").textContent = view.engineState;
  document.querySelector("[data-run-state]").textContent = view.running ? "[RUNNING]" : "[PAUSED]";
  document.querySelector("[data-tick-chip]").textContent = `tick ${state.tick}`;
  document.querySelector("[data-tx-chip]").textContent = `tx ${state.proof.transactionId}`;
  document.querySelector("[data-rev-chip]").textContent = `rev ${state.proof.revision}`;
  document.querySelector("[data-verdict-chip]").textContent = verdict(state);
  document.querySelector("[data-replay-proof]").textContent = view.replayProof;
}

function renderCounters(state) {
  renderSide("naive", state.naive);
  renderSide("trellis", state.trellis);
}

function renderSide(name, stats) {
  document.querySelector(`[data-${name}-verdict]`).textContent = `[${stats.verdict.toUpperCase()}]`;
  document.querySelector(`[data-${name}-open]`).textContent = stats.open;
  document.querySelector(`[data-${name}-should]`).textContent = stats.shouldOpen;
  document.querySelector(`[data-${name}-delta]`).textContent = signed(stats.delta);
  document.querySelector(`[data-${name}-orphaned]`).textContent = stats.orphaned;
  document.querySelector(`[data-${name}-dupes]`).textContent = stats.duplicateHandles;
}

function renderTimeline(target, activity, tick) {
  target.replaceChildren(
    ...activity.map((item) => {
      const button = el("button", `timeline-row ${item.tick === tick ? "selected" : ""}`);
      button.type = "button";
      button.dataset.tickSelect = item.tick;
      button.append(
        el("strong", "", `${item.tick}. ${item.label}`),
        el("span", "", item.detail),
        el("em", "", item.naiveNote),
        el("code", "", item.trellisNote),
      );
      return button;
    }),
  );
}

function renderRows(target, rows, selected) {
  target.replaceChildren(
    ...rows.map((row) => {
      const item = el("button", `stream-row ${row.key === selected ? "selected" : ""}`);
      item.type = "button";
      item.dataset.streamSelect = row.key;
      const desired = row.shouldOpen ? "[OPEN]" : "[CLOSE]";
      const drift = row.naiveOpen === row.trellisOpen && row.trellisOpen === (row.shouldOpen ? 1 : 0)
        ? "[MATCH]"
        : "[DRIFT]";
      item.append(
        el("strong", "", row.label),
        el("span", row.shouldOpen ? "kind-open" : "kind-close", desired),
        el("span", "", `cb ${row.naiveOpen}`),
        el("span", "", `Trellis ${row.trellisOpen}`),
        el("span", drift === "[DRIFT]" ? "kind-close" : "", drift),
      );
      return item;
    }),
  );
}

function renderReceipt(target, state) {
  const receipt = state.selectedReceipt;
  const command = state.proof.selectedCommand;
  const diff = state.proof.desiredDiff;
  const commandText = command
    ? `${commandKind(command.kind)} ${command.key} scope ${command.scope} tx ${command.transactionId} rev ${command.revision}`
    : "[NO COMMAND] selected stream unchanged in current transaction";
  target.replaceChildren(
    headerLine(receipt.title, receipt.status),
    fact("stream key", receipt.key),
    fact("canonical input", inputSummary(state.inputs)),
    fact("desiredAttachments diff", diffSummary(diff, receipt.key)),
    fact("Trellis command", commandText, "fact-row command-proof"),
    fact("applied ledger", command?.appliedLedgerResult ?? "existing ledger state carried forward"),
    fact("callback note", callbackNote(state.rows, receipt.key)),
    fact("cause path", command?.cause ?? receipt.steps.at(-1)?.detail ?? "no prior command"),
  );
}

function renderInputs(target, inputs, shouldOpen) {
  const online = inputs.networkOnline ? "[ONLINE]" : "[OFFLINE FAIL-CLOSED]";
  target.replaceChildren(
    fact("workspace", inputs.workspace),
    fact("joined rooms", [...inputs.joinedRooms].join(", ") || "empty"),
    fact("permission grants", [...inputs.permissionGrants].join(", ") || "empty"),
    fact("follows", [...inputs.follows].join(", ") || "empty"),
    fact("network", online),
    fact("desired open", shouldOpen.map(shortKey).join("; ") || "none"),
  );
}

function renderProof(target, state, view) {
  const proof = state.proof;
  target.replaceChildren(
    fact("wasm", proof.wasmBundle),
    fact("graph terms", `${proof.inputNode} -> ${proof.collection} -> ${proof.scope}`),
    fact("transaction", `tx ${proof.transactionId} / rev ${proof.revision}`),
    fact("deterministic replay", `${proof.deterministicReplay}; ${view.replayProof}`),
    ...proof.invariants.map((check) => {
      const row = fact(check.passed ? "[PASS]" : "[FAIL]", `${check.name}: ${check.detail}`, "fact-row invariant-row");
      row.dataset.status = check.passed ? "pass" : "fail";
      return row;
    }),
  );
}

function renderCommands(target, commands) {
  if (!commands.length) {
    target.replaceChildren(fact("resource plan", "no open/close commands in this transaction"));
    return;
  }
  target.replaceChildren(
    ...commands.map((command) => {
      const row = el("div", "command-row");
      row.dataset.kind = command.kind;
      row.append(
        badge(commandKind(command.kind), command.kind === "OPEN" ? "open" : "close"),
        el("code", "", command.key),
        el("span", "", `${command.scope} / tx ${command.transactionId} / rev ${command.revision}`),
        el("em", "", command.cause),
      );
      return row;
    }),
  );
}

function headerLine(title, status) {
  const node = el("div", "receipt-title");
  node.append(el("strong", "", title), badge(`[${status.toUpperCase()}]`));
  return node;
}

function inputSummary(inputs) {
  return `workspace=${inputs.workspace}; rooms=${[...inputs.joinedRooms].join(",") || "empty"}; grants=${[...inputs.permissionGrants].join(",") || "empty"}; follows=${[...inputs.follows].join(",") || "empty"}; online=${inputs.networkOnline}`;
}

function diffSummary(diff, key) {
  const parts = [`+${diff.added.length}`, `-${diff.removed.length}`, `=${diff.unchanged.length}`];
  if (diff.added.includes(key)) parts.push(`${shortKey(key)} added`);
  if (diff.removed.includes(key)) parts.push(`${shortKey(key)} removed`);
  if (diff.unchanged.includes(key)) parts.push(`${shortKey(key)} unchanged`);
  return parts.join(" / ");
}

function callbackNote(rows, key) {
  const row = rows.find((candidate) => candidate.key === key);
  if (!row) return "stream absent from both ledgers";
  const expected = row.shouldOpen ? 1 : 0;
  return `callbacks=${row.naiveOpen}, Trellis=${row.trellisOpen}, expected=${expected}`;
}

function verdict(state) {
  const drift = state.naive.delta !== 0 || state.naive.orphaned > 0 || state.naive.duplicateHandles > 0;
  const reconciled = state.trellis.delta === 0 && state.trellis.orphaned === 0 && state.trellis.duplicateHandles === 0;
  if (drift && reconciled) return "[CALLBACK DRIFT / TRELLIS PASS]";
  if (reconciled) return "[TRELLIS PASS]";
  return "[CHECK FAIL]";
}
