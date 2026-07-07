import { describe, expect, it } from "vitest";
import {
  actionMeta,
  domainRows,
  labShowcases,
  lifecycleSummary,
  resourceRows,
  stepAt,
} from "./showcaseLabModel";

describe("showcase lab model", () => {
  it("exposes all flagship traces as interactive showcases", () => {
    expect(labShowcases.map((showcase) => showcase.trace.showcase)).toEqual([
      "workspace-sync-board",
      "mini-language-server",
      "fleetpulse",
    ]);
    expect(labShowcases.every((showcase) => showcase.trace.steps.length > 0)).toBe(true);
  });

  it("binds action buttons to structural trace steps", () => {
    const workspace = labShowcases[0];
    const step = stepAt(workspace, 0);

    expect(actionMeta(step)).toContain("resources");
    expect(step.trace.transaction_id).toBeGreaterThan(0);
    expect(step.trace.revision).toBeGreaterThan(0);
  });

  it("summarizes resource opens, closes, frames, and invariant passes", () => {
    const workspace = labShowcases[0];
    const summary = lifecycleSummary(stepAt(workspace, 0));

    expect(summary.opens).toBeGreaterThan(0);
    expect(summary.closes).toBeGreaterThan(0);
    expect(summary.frames).toBeGreaterThan(0);
    expect(summary.invariants).toBeGreaterThan(0);
  });

  it("derives domain rows from trace lifecycle facts", () => {
    const fleet = labShowcases[2];
    const step = stepAt(fleet, 0);

    expect(domainRows(fleet, step).some((row) => row.label === "Telemetry topics")).toBe(true);
    expect(resourceRows(step).some((row) => row.label.includes("fleet/topic"))).toBe(true);
  });
});
