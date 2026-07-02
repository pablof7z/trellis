import { describe, expect, it } from "vitest";
import { flattenDiagnostics, latestTrace } from "./engineClient";
import type { AppState } from "./types";

describe("UI state helpers", () => {
  it("flattens diagnostics by file deterministically", () => {
    const state = {
      outputLedger: {
        diagnosticsByFile: {
          "a.tl": [{ id: "a", filePath: "a.tl", line: 1, column: 1, severity: "error", message: "A", source: "parser" }],
          "b.tl": [{ id: "b", filePath: "b.tl", line: 2, column: 1, severity: "error", message: "B", source: "resolver" }],
        },
      },
    } as unknown as AppState;
    expect(flattenDiagnostics(state).map((diagnostic) => diagnostic.id)).toEqual(["a", "b"]);
  });

  it("returns the latest trace", () => {
    const state = { traces: [{ txId: 1 }, { txId: 2 }] } as AppState;
    expect(latestTrace(state).txId).toBe(2);
  });
});
