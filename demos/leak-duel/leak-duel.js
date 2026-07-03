import { clearTimer } from "./leak-duel-dom.js";
import { renderAll } from "./leak-duel-render.js";

const DEFAULTS = { seed: 1337, chaos: 8, tick: 24 };
const MAX_TICK = 120;
const RUN_CUT = 30;

const els = {
  root: document.querySelector("[data-demo-root]"),
  seed: document.querySelector("[data-seed]"),
  chaos: document.querySelector("[data-chaos]"),
  chaosValue: document.querySelector("[data-chaos-value]"),
  tick: document.querySelector("[data-tick-range]"),
  tickValue: document.querySelector("[data-tick-value]"),
  step: document.querySelector("[data-step]"),
  run: document.querySelector("[data-run]"),
  replay: document.querySelector("[data-replay]"),
  reset: document.querySelector("[data-reset]"),
  tabs: document.querySelector("[data-mobile-tabs]"),
  timeline: document.querySelector("[data-timeline]"),
  rows: document.querySelector("[data-rows]"),
  receipt: document.querySelector("[data-receipt]"),
  inputs: document.querySelector("[data-inputs]"),
  proof: document.querySelector("[data-proof]"),
  commands: document.querySelector("[data-commands]"),
  raw: document.querySelector("[data-raw-json]"),
};

let wasm;
let state;
let timer;
let selected = urlParam("selected");
let activePanel = "ledger";
let replayProof = "[PENDING]";

window.trellisDemoEvents = window.trellisDemoEvents ?? [];

async function boot() {
  setControls(false);
  wasm = await import("./engine/trellis_observatory_engine.js");
  await wasm.default();
  els.seed.value = numberParam("seed", DEFAULTS.seed);
  els.chaos.value = numberParam("chaos", DEFAULTS.chaos);
  els.tick.value = numberParam("tick", numberParam("ticks", DEFAULTS.tick));
  wireEvents();
  setControls(true);
  render("ready");
}

function wireEvents() {
  els.seed.addEventListener("change", () => resetCut({ keepScenario: true }));
  els.chaos.addEventListener("input", () => {
    els.chaosValue.value = els.chaos.value;
    render("ready");
  });
  els.tick.addEventListener("input", () => {
    stopRun();
    render("paused");
  });
  els.step.addEventListener("click", () => {
    stopRun();
    els.tick.value = Math.min(Number(els.tick.value) + 1, MAX_TICK);
    render("paused");
  });
  els.run.addEventListener("click", toggleRun);
  els.replay.addEventListener("click", replaySeed);
  els.reset.addEventListener("click", () => resetCut({ keepScenario: false }));
  els.root.addEventListener("click", handleSelection);
  els.tabs.addEventListener("click", handleTab);
}

function render(engineState) {
  if (!wasm) return;
  state = snapshot();
  selected = state.selectedReceipt.key;
  els.root.dataset.activePanel = activePanel;
  els.tick.value = state.tick;
  els.tickValue.value = `tick ${state.tick}`;
  els.chaosValue.value = els.chaos.value;
  renderAll(els, state, {
    selected,
    activePanel,
    engineState: `[${engineState.toUpperCase()}]`,
    running: Boolean(timer),
    replayProof,
  });
  updateUrl();
  track("render", { tick: state.tick, selected });
}

function snapshot(override = {}) {
  const request = {
    seed: Number((override.seed ?? els.seed.value) || DEFAULTS.seed),
    chaos: Number((override.chaos ?? els.chaos.value) || DEFAULTS.chaos),
    ticks: Number((override.tick ?? els.tick.value) || DEFAULTS.tick),
    selected,
  };
  return JSON.parse(wasm.leak_duel(JSON.stringify(request)));
}

function toggleRun() {
  if (timer) {
    stopRun();
    render("paused");
    return;
  }
  els.run.textContent = "Pause";
  timer = setInterval(() => {
    const next = Math.min(Number(els.tick.value) + 1, Math.max(RUN_CUT, Number(els.tick.value)), MAX_TICK);
    els.tick.value = next;
    render(next >= RUN_CUT ? "completed" : "running");
    if (next >= RUN_CUT || next >= MAX_TICK) stopRun();
  }, prefersReducedMotion() ? 900 : 420);
  render("running");
}

function stopRun() {
  timer = clearTimer(timer);
  els.run.textContent = "Run cut";
}

function replaySeed() {
  stopRun();
  const left = JSON.stringify(snapshot());
  const right = JSON.stringify(snapshot());
  replayProof = left === right ? "[PASS] deterministic replay" : "[FAIL] replay diverged";
  els.tick.value = 0;
  selected = null;
  render("paused");
}

function resetCut({ keepScenario }) {
  stopRun();
  if (!keepScenario) {
    els.seed.value = DEFAULTS.seed;
    els.chaos.value = DEFAULTS.chaos;
  }
  els.tick.value = keepScenario ? 0 : DEFAULTS.tick;
  selected = null;
  replayProof = "[PENDING]";
  render("ready");
}

function handleSelection(event) {
  const tickButton = event.target.closest("[data-tick-select]");
  const streamButton = event.target.closest("[data-stream-select]");
  if (tickButton) {
    stopRun();
    els.tick.value = tickButton.dataset.tickSelect;
    render("paused");
  }
  if (streamButton) {
    selected = streamButton.dataset.streamSelect;
    render("paused");
  }
}

function handleTab(event) {
  const tab = event.target.closest("[data-panel-tab]");
  if (!tab) return;
  activePanel = tab.dataset.panelTab;
  document.querySelectorAll("[data-panel-tab]").forEach((button) => {
    button.classList.toggle("active", button === tab);
  });
  els.root.dataset.activePanel = activePanel;
}

function updateUrl() {
  const params = new URLSearchParams({
    seed: String(state.seed),
    chaos: String(state.chaos),
    tick: String(state.tick),
    selected,
  });
  history.replaceState(null, "", `?${params}`);
}

function setControls(enabled) {
  [els.seed, els.chaos, els.tick, els.step, els.run, els.replay, els.reset].forEach((control) => {
    control.disabled = !enabled;
  });
}

function numberParam(name, fallback) {
  const raw = urlParam(name);
  if (raw == null || raw === "") return fallback;
  const value = Number(raw);
  return Number.isFinite(value) ? value : fallback;
}

function urlParam(name) {
  return new URLSearchParams(location.search).get(name);
}

function prefersReducedMotion() {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

function track(name, payload = {}) {
  window.trellisDemoEvents.push({ name, payload, at: Date.now() });
}

boot().catch((error) => {
  console.error(error);
  els.root.innerHTML = `
    <section class="container leak-error">
      <h1>Leak Duel failed closed.</h1>
      <p>Could not load demos/leak-duel/engine/trellis_observatory_engine_bg.wasm. Rebuild the WASM bundle and reload.</p>
    </section>`;
});
