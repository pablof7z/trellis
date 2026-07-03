const els = {
  seed: document.querySelector("[data-seed]"),
  chaos: document.querySelector("[data-chaos]"),
  chaosValue: document.querySelector("[data-chaos-value]"),
  step: document.querySelector("[data-step]"),
  run: document.querySelector("[data-run]"),
  reset: document.querySelector("[data-reset]"),
  tick: document.querySelector("[data-tick]"),
  rows: document.querySelector("[data-rows]"),
  receipt: document.querySelector("[data-receipt]"),
  activity: document.querySelector("[data-activity]"),
};

let wasm;
let ticks = Number(new URLSearchParams(location.search).get("ticks") ?? 18);
let selected = new URLSearchParams(location.search).get("selected");
let timer;

window.trellisDemoEvents = window.trellisDemoEvents ?? [];

async function boot() {
  wasm = await import("./engine/trellis_observatory_engine.js");
  await wasm.default();
  const params = new URLSearchParams(location.search);
  els.seed.value = params.get("seed") ?? "1337";
  els.chaos.value = params.get("chaos") ?? "7";
  els.chaosValue.value = els.chaos.value;
  render();
}

function snapshot() {
  const request = {
    seed: Number(els.seed.value || 1337),
    chaos: Number(els.chaos.value || 7),
    ticks,
    selected,
  };
  return JSON.parse(wasm.leak_duel(JSON.stringify(request)));
}

function render() {
  const state = snapshot();
  selected = state.selectedReceipt.key;
  els.tick.textContent = `tick ${state.tick}`;
  renderSide("naive", state.naive);
  renderSide("trellis", state.trellis);
  renderRows(state.rows);
  renderReceipt(state.selectedReceipt);
  renderActivity(state.activity);
  updateUrl(state);
}

function renderSide(name, stats) {
  setText(`[data-${name}-verdict]`, stats.verdict);
  setText(`[data-${name}-open]`, stats.open);
  setText(`[data-${name}-should]`, stats.shouldOpen);
  setText(`[data-${name}-delta]`, signed(stats.delta));
  setText(
    `[data-${name}-detail]`,
    `${stats.orphaned} orphaned, ${stats.duplicateHandles} duplicate handles`
  );
}

function renderRows(rows) {
  els.rows.replaceChildren(
    ...rows.map((row) => {
      const button = document.createElement("button");
      button.className = `attachment-row ${row.key === selected ? "selected" : ""}`;
      button.type = "button";
      const intent = row.shouldOpen ? "[OPEN]" : "[CLOSE]";
      const intentClass = row.shouldOpen ? "kind-open" : "kind-close";
      button.innerHTML = `
        <span>${escapeHtml(row.label)}</span>
        <span class="${intentClass}">${intent}</span>
        <span>callbacks ${row.naiveOpen}</span>
        <span>Trellis ${row.trellisOpen}</span>
      `;
      button.addEventListener("click", () => {
        selected = row.key;
        track("why-open-click", { key: row.key });
        render();
      });
      return button;
    })
  );
}

function renderReceipt(receipt) {
  els.receipt.replaceChildren(
    el("h3", receipt.title),
    el("p", `${receipt.status}: ${receipt.key}`),
    ...receipt.steps.map((step) => {
      const item = document.createElement("div");
      item.className = "receipt-step";
      item.append(el("strong", step.label), el("span", step.detail));
      return item;
    })
  );
}

function renderActivity(items) {
  els.activity.replaceChildren(
    ...items.map((item) => {
      const row = document.createElement("div");
      row.className = "activity-row";
      row.append(
        el("strong", `${item.tick}. ${item.label}`),
        el("span", item.detail),
        el("em", item.naiveNote),
        el("code", item.trellisNote)
      );
      return row;
    })
  );
}

function updateUrl(state) {
  const params = new URLSearchParams({
    seed: String(state.seed),
    chaos: String(state.chaos),
    ticks: String(state.tick),
    selected,
  });
  history.replaceState(null, "", `?${params}`);
}

function setText(selector, value) {
  document.querySelector(selector).textContent = value;
}

function signed(value) {
  return value > 0 ? `+${value}` : String(value);
}

function el(tag, text) {
  const node = document.createElement(tag);
  node.textContent = text;
  return node;
}

function escapeHtml(value) {
  return value.replace(/[&<>"']/g, (char) => ({
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

els.chaos.addEventListener("input", () => {
  ticks = Math.min(ticks, 120);
  els.chaosValue.value = els.chaos.value;
  track("chaos-slider", { chaos: Number(els.chaos.value) });
  render();
});

els.seed.addEventListener("change", () => {
  ticks = 0;
  selected = null;
  render();
});

els.step.addEventListener("click", () => {
  ticks = Math.min(ticks + 1, 120);
  track("step", { ticks });
  render();
});

els.reset.addEventListener("click", () => {
  clearInterval(timer);
  ticks = 0;
  selected = null;
  track("reset");
  render();
});

els.run.addEventListener("click", () => {
  clearInterval(timer);
  ticks = 0;
  track("run-30-second-cut");
  timer = setInterval(() => {
    ticks += 1;
    render();
    if (ticks >= 30) clearInterval(timer);
  }, 1000);
});

boot().catch((error) => {
  console.error(error);
  document.querySelector("[data-demo-root]").innerHTML =
    '<section class="container page-hero"><h1>Leak Duel failed to load.</h1><p class="lead">The wasm bundle did not initialize. Run the local build script and reload.</p></section>';
});
