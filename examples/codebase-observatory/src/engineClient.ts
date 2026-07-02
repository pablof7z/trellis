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
