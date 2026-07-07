import {
  outputFrameKind,
  resourceKeyLabel,
  showcaseTraces,
  summarizeStep,
  type OutputFrame,
  type ResourceCommand,
  type ShowcaseStep,
  type ShowcaseTrace,
} from "./traceContract";

export type ShowcaseKind = "workspace" | "language" | "fleet";

export type LabShowcase = {
  trace: ShowcaseTrace;
  kind: ShowcaseKind;
  title: string;
  summary: string;
  actionNoun: string;
};

export type LifecycleRow = {
  label: string;
  value: string;
  meta: string;
  tone: "open" | "close" | "neutral" | "fail" | "pass";
};

const copy: Record<string, Omit<LabShowcase, "trace">> = {
  "workspace-sync-board": {
    kind: "workspace",
    title: "Workspace Sync Board",
    summary: "Workspace route and permission inputs reconcile sync windows and board output frames.",
    actionNoun: "board action",
  },
  "mini-language-server": {
    kind: "language",
    title: "Mini Language Server",
    summary: "File graph changes reconcile watchers, diagnostics, semantic output, and workspace teardown.",
    actionNoun: "editor action",
  },
  fleetpulse: {
    kind: "fleet",
    title: "FleetPulse Telemetry",
    summary: "Permission and filter inputs reconcile telemetry topics, alert streams, cards, and late host statuses.",
    actionNoun: "dashboard action",
  },
};

export const labShowcases: LabShowcase[] = showcaseTraces.map((trace) => ({
  trace,
  ...copy[trace.showcase],
}));

export function stepAt(showcase: LabShowcase, index: number) {
  return showcase.trace.steps[index] ?? showcase.trace.steps[0];
}

export function actionLabel(step: ShowcaseStep) {
  const summary = summarizeStep(step);
  return `${step.name} · tx ${summary.tx} · rev ${summary.revision}`;
}

export function actionMeta(step: ShowcaseStep) {
  const summary = summarizeStep(step);
  return `${summary.resources} resources · ${summary.frames} frames · ${summary.diffs} diffs`;
}

export function resourceRows(step: ShowcaseStep): LifecycleRow[] {
  return step.trace.resource_commands.map((command) => ({
    label: resourceLabel(command),
    value: command.kind,
    meta: `scope ${command.scope} · policy ${command.transition_policy}`,
    tone: command.kind === "Open" ? "open" : command.kind === "Close" ? "close" : "neutral",
  }));
}

export function outputRows(step: ShowcaseStep): LifecycleRow[] {
  return step.trace.output_frames.map((frame) => ({
    label: outputLabel(frame),
    value: outputFrameKind(frame.kind),
    meta: `scope ${frame.scope} · rev ${frame.revision}`,
    tone: "neutral",
  }));
}

export function invariantRows(step: ShowcaseStep): LifecycleRow[] {
  return step.trace.invariant_results.map((result) => ({
    label: result.name,
    value: result.passed ? "PASS" : "FAIL",
    meta: `tx ${step.trace.transaction_id} · rev ${step.trace.revision}`,
    tone: result.passed ? "pass" : "fail",
  }));
}

export function hostRows(step: ShowcaseStep): LifecycleRow[] {
  return step.host_statuses.map((status) => ({
    label: status.target,
    value: status.status,
    meta: `command rev ${status.command_revision ?? "n/a"}`,
    tone: status.status === "Current" ? "pass" : "neutral",
  }));
}

export function diffRows(step: ShowcaseStep): LifecycleRow[] {
  return step.trace.collection_diffs.map((diff) => ({
    label: `${diff.kind} collection ${diff.node}`,
    value: `+${diff.added} -${diff.removed} ~${diff.updated}`,
    meta: `${diff.unchanged} unchanged`,
    tone: diff.removed > 0 ? "close" : diff.added > 0 ? "open" : "neutral",
  }));
}

export function domainRows(showcase: LabShowcase, step: ShowcaseStep): LifecycleRow[] {
  if (showcase.kind === "workspace") return workspaceRows(step);
  if (showcase.kind === "language") return languageRows(step);
  return fleetRows(step);
}

export function lifecycleSummary(step: ShowcaseStep) {
  const resources = step.trace.resource_commands;
  return {
    opens: resources.filter((command) => command.kind === "Open").length,
    closes: resources.filter((command) => command.kind === "Close").length,
    frames: step.trace.output_frames.length,
    hosts: step.host_statuses.length,
    invariants: step.trace.invariant_results.filter((result) => result.passed).length,
  };
}

function workspaceRows(step: ShowcaseStep): LifecycleRow[] {
  const resources = step.trace.resource_commands;
  return [
    sectionRow("Project windows", resources, "sync/project"),
    sectionRow("Comment windows", resources, "sync/comments"),
    sectionRow("Profile hydration", resources, "sync/profile"),
    frameRow(step, "Board output"),
  ].filter(Boolean) as LifecycleRow[];
}

function languageRows(step: ShowcaseStep): LifecycleRow[] {
  const watchers = step.trace.resource_commands.filter((command) =>
    resourceKeyLabel(command.key).startsWith("watch/"),
  );
  return [
    {
      label: "File watchers",
      value: `${watchers.length} command${watchers.length === 1 ? "" : "s"}`,
      meta: watchers.map((command) => `${command.kind} ${resourceKeyLabel(command.key).replace("watch/", "")}`).join(", ") || "no watcher changes",
      tone: watchers.some((command) => command.kind === "Close") ? "close" : "neutral",
    },
    frameRow(step, "Diagnostics output"),
    ...diffRows(step),
  ];
}

function fleetRows(step: ShowcaseStep): LifecycleRow[] {
  const topics = step.trace.resource_commands.filter((command) =>
    resourceKeyLabel(command.key).startsWith("fleet/topic/"),
  );
  const alerts = step.trace.resource_commands.filter((command) =>
    resourceKeyLabel(command.key).startsWith("fleet/alert/"),
  );
  return [
    {
      label: "Telemetry topics",
      value: `${topics.length} command${topics.length === 1 ? "" : "s"}`,
      meta: topics.map((command) => `${command.kind} ${lastSegment(command)}`).join(", ") || "no topic changes",
      tone: topics.some((command) => command.kind === "Close") ? "close" : "neutral",
    },
    {
      label: "Alert streams",
      value: `${alerts.length} command${alerts.length === 1 ? "" : "s"}`,
      meta: alerts.map((command) => `${command.kind} ${lastSegment(command)}`).join(", ") || "no alert changes",
      tone: alerts.some((command) => command.kind === "Close") ? "close" : "neutral",
    },
    frameRow(step, "Dashboard frame"),
    ...hostRows(step),
  ];
}

function sectionRow(label: string, resources: ResourceCommand[], prefix: string) {
  const matches = resources.filter((command) => resourceKeyLabel(command.key).startsWith(prefix));
  if (matches.length === 0) return null;
  return {
    label,
    value: `${matches.length} command${matches.length === 1 ? "" : "s"}`,
    meta: matches.map((command) => `${command.kind} ${lastSegment(command)}`).join(", "),
    tone: matches.some((command) => command.kind === "Close") ? "close" : "open",
  };
}

function frameRow(step: ShowcaseStep, label: string): LifecycleRow {
  const frames = step.trace.output_frames;
  return {
    label,
    value: `${frames.length} frame${frames.length === 1 ? "" : "s"}`,
    meta: frames.map(outputLabel).join(", ") || "no output frames",
    tone: "neutral",
  };
}

function resourceLabel(command: ResourceCommand) {
  return resourceKeyLabel(command.key);
}

function outputLabel(frame: OutputFrame) {
  return `${outputFrameKind(frame.kind)} output ${frame.output_key}`;
}

function lastSegment(command: ResourceCommand) {
  const label = resourceKeyLabel(command.key);
  return label.split("/").at(-1) ?? label;
}
