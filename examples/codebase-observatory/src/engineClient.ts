import type { AppState, EngineApi, ReplayResult } from "./types";

export async function loadEngine(): Promise<EngineApi> {
  const wasm = await import("./wasm/trellis_observatory_engine.js");
  await wasm.default();
  return {
    initialState: () => JSON.parse(wasm.initial_state()) as AppState,
    dispatch: (state, action) =>
      JSON.parse(wasm.dispatch(JSON.stringify(state), JSON.stringify(action))) as AppState,
    replay: (state) => JSON.parse(wasm.replay(JSON.stringify(state))) as ReplayResult,
  };
}

export const scenarioActions = [
  ["Delete legacy_user.tl", { type: "deleteFile", path: "src/legacy_user.tl" }],
  ["Switch to schema-v2", { type: "switchBranch", branch: "feature/schema-v2" }],
  ["Rename schema target", { type: "renameSchema" }],
  ["Edit with type error", { type: "editAppWithTypeError" }],
  ["Fix app.tl", { type: "fixApp" }],
  ["Start slow analysis", { type: "startSlowAnalysis" }],
  ["Inject stale result", { type: "injectStaleAnalysisResult" }],
  ["Toggle generated", { type: "toggleGenerated" }],
  ["Config strict/loose", { type: "changeConfig", config: "loose" }],
  ["Close app tab", { type: "closeAppTab" }],
] as const;

export const scenarioSteps = [
  {
    label: "Delete imported file",
    action: { type: "deleteFile", path: "src/legacy_user.tl" },
    story: "Next: remove legacy_user.tl and clear every output and resource owned by that file.",
    completed: "legacy_user.tl removed; owned diagnostics, links, tokens, jobs, and watchers reconciled.",
  },
  {
    label: "Rename import target",
    action: { type: "renameSchema" },
    story: "Next: rename schema.tl to schema_v2.tl and rebaseline visible document links.",
    completed: "schema.tl renamed; visible document links rebaselined without reopening the editor.",
  },
  {
    label: "Start slow analysis",
    action: { type: "startSlowAnalysis" },
    story: "Next: start a revisioned analysis job for the currently visible editor.",
    completed: "slow analysis started for the visible editor revision.",
  },
  {
    label: "Fix app.tl",
    action: { type: "fixApp" },
    story: "Next: commit a newer app.tl revision that supersedes older host results.",
    completed: "app.tl fixed; older analysis commands are no longer current.",
  },
  {
    label: "Inject late result",
    action: { type: "injectStaleAnalysisResult" },
    story: "Next: deliver the old analysis result and verify it cannot mutate editor output.",
    completed: "late analysis result arrived and was classified as stale.",
  },
] as const;

export function latestTrace(state: AppState) {
  return state.traces[state.traces.length - 1];
}

export function flattenDiagnostics(state: AppState) {
  return Object.values(state.outputLedger.diagnosticsByFile).flat();
}

export function activeContent(state: AppState) {
  const path = state.inputs.activeEditor ?? "src/app.tl";
  return state.inputs.files[path]?.contents ?? "";
}
