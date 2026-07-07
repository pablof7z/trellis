import { describe, expect, it } from "vitest";
import {
  outputFrameKind,
  resourceKeyLabel,
  selectedDetail,
  showcaseTraces,
  summarizeStep,
} from "./traceContract";

describe("trace contract fixtures", () => {
  it("loads all flagship headless traces", () => {
    expect(showcaseTraces.map((trace) => trace.showcase)).toEqual([
      "workspace-sync-board",
      "mini-language-server",
      "fleetpulse",
    ]);
    expect(showcaseTraces.every((trace) => trace.contract === "trellis.showcase.trace")).toBe(true);
  });

  it("summarizes structural transaction counts", () => {
    const [workspace] = showcaseTraces;
    const summary = summarizeStep(workspace.steps[0]);

    expect(summary.tx).toBeGreaterThan(0);
    expect(summary.revision).toBeGreaterThan(0);
    expect(summary.resources).toBeGreaterThan(0);
    expect(summary.frames).toBeGreaterThan(0);
  });

  it("formats resource keys and frame kinds from serialized traces", () => {
    expect(resourceKeyLabel(["sync", "project", "mobile"])).toBe("sync/project/mobile");
    expect(outputFrameKind({ Rebaseline: "Requested" })).toBe("Rebaseline(Requested)");
  });

  it("builds command detail with revision and structural cause", () => {
    const [workspace] = showcaseTraces;
    const step = workspace.steps[0];
    const detail = selectedDetail(step, { type: "resource", index: 0 });

    expect(detail.title).toContain("sync/project/mobile");
    expect(detail.rows.some((row) => row.startsWith("revision "))).toBe(true);
    expect(detail.causeRows.some((row) => row.includes("input node"))).toBe(true);
  });
});
