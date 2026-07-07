import fleetpulse from "./trace-fixtures/fleetpulse.json";
import miniLanguageServer from "./trace-fixtures/mini-language-server.json";
import workspaceSyncBoard from "./trace-fixtures/workspace-sync-board.json";

export type ShowcaseTrace = {
  contract: string;
  format_version: number;
  showcase: string;
  script: string;
  command: string[];
  replay: ReplayMetadata;
  seeded_bug: SeededBugStatus;
  steps: ShowcaseStep[];
};

export type ReplayMetadata = {
  status: string;
  compared_runs: number;
  reason: string | null;
};

export type SeededBugStatus = {
  status: string;
  issue: string;
  reason: string;
};

export type ShowcaseStep = {
  name: string;
  host_statuses: ShowcaseHostStatus[];
  trace: ContractTrace;
};

export type ShowcaseHostStatus = {
  target: string;
  status: string;
  command_revision: number | null;
};

export type ContractTrace = {
  transaction_id: number;
  revision: number;
  staged_input_changes: StagedInputChange[];
  changed_inputs: number[];
  dirty_roots: number[];
  recomputed_derived_nodes: number[];
  changed_derived_nodes: number[];
  recomputed_collection_nodes: number[];
  changed_collection_nodes: number[];
  collection_diffs: CollectionDiff[];
  resource_commands: ResourceCommand[];
  output_frames: OutputFrame[];
  scope_events: ScopeEvent[];
  audit_log: AuditEntry[];
  audit_explanations: AuditExplanations;
  phase_trace: string[];
  invariant_results: InvariantResult[];
};

export type StagedInputChange = {
  node: number;
  outcome: string;
};

export type CollectionDiff = {
  node: number;
  kind: string;
  added: number;
  removed: number;
  updated: number;
  unchanged: number;
};

export type ResourceCommand = {
  key: string | string[];
  scope: number;
  kind: string;
  transition_policy: string;
};

export type OutputFrame = {
  output_key: number | string;
  scope: number;
  transaction_id: number;
  revision: number;
  kind: string | Record<string, string>;
};

export type ScopeEvent = {
  scope: number;
  kind: string;
};

export type AuditEntry = {
  transaction_id: number;
  revision: number;
  event: Record<string, unknown>;
};

export type AuditExplanations = {
  level: string;
  revision: number;
  transaction_id: number;
  node_changes: unknown[];
  resource_commands: unknown[];
  output_frames: unknown[];
};

export type InvariantResult = {
  name: string;
  passed: boolean;
};

export type TraceSelection =
  | { type: "resource"; index: number }
  | { type: "frame"; index: number }
  | { type: "host"; index: number }
  | { type: "scope"; index: number };

export type SelectionDetail = {
  title: string;
  rows: string[];
  causeRows: string[];
};

export const showcaseTraces = [
  workspaceSyncBoard,
  miniLanguageServer,
  fleetpulse,
] as ShowcaseTrace[];

export function summarizeStep(step: ShowcaseStep) {
  const trace = step.trace;
  return {
    tx: trace.transaction_id,
    revision: trace.revision,
    resources: trace.resource_commands.length,
    frames: trace.output_frames.length,
    scopes: trace.scope_events.length,
    hosts: step.host_statuses.length,
    invariants: trace.invariant_results.length,
    failedInvariants: trace.invariant_results.filter((result) => !result.passed).length,
    diffs: trace.collection_diffs.length,
  };
}

export function resourceKeyLabel(key: ResourceCommand["key"]) {
  return Array.isArray(key) ? key.join("/") : key;
}

export function outputFrameKind(kind: OutputFrame["kind"]) {
  if (typeof kind === "string") return kind;
  const [name, reason] = Object.entries(kind)[0] ?? ["Unknown", ""];
  return reason ? `${name}(${reason})` : name;
}

export function selectedDetail(step: ShowcaseStep, selection: TraceSelection | null): SelectionDetail {
  const trace = step.trace;
  if (!selection) {
    return {
      title: "Select a command or frame",
      rows: [`tx ${trace.transaction_id}`, `revision ${trace.revision}`],
      causeRows: causeRows(trace),
    };
  }
  if (selection.type === "resource") return resourceDetail(trace, trace.resource_commands[selection.index]);
  if (selection.type === "frame") return frameDetail(trace, trace.output_frames[selection.index]);
  if (selection.type === "host") return hostDetail(step, selection.index);
  return scopeDetail(trace, trace.scope_events[selection.index]);
}

export function traceCostRows(trace: ContractTrace) {
  return [
    `phase steps ${trace.phase_trace.length}`,
    `dirty roots ${trace.dirty_roots.length}`,
    `derived recomputes ${trace.recomputed_derived_nodes.length}`,
    `collection recomputes ${trace.recomputed_collection_nodes.length}`,
    `audit events ${trace.audit_log.length}`,
  ];
}

function resourceDetail(trace: ContractTrace, command?: ResourceCommand): SelectionDetail {
  if (!command) return missingDetail(trace);
  return {
    title: `${command.kind} ${resourceKeyLabel(command.key)}`,
    rows: [
      `scope ${command.scope}`,
      `revision ${trace.revision}`,
      `transaction ${trace.transaction_id}`,
      `policy ${command.transition_policy}`,
    ],
    causeRows: causeRows(trace),
  };
}

function frameDetail(trace: ContractTrace, frame?: OutputFrame): SelectionDetail {
  if (!frame) return missingDetail(trace);
  return {
    title: `${outputFrameKind(frame.kind)} output ${frame.output_key}`,
    rows: [
      `scope ${frame.scope}`,
      `revision ${frame.revision}`,
      `transaction ${frame.transaction_id}`,
    ],
    causeRows: causeRows(trace),
  };
}

function hostDetail(step: ShowcaseStep, index: number): SelectionDetail {
  const status = step.host_statuses[index];
  if (!status) return missingDetail(step.trace);
  return {
    title: status.target,
    rows: [
      `status ${status.status}`,
      `command revision ${status.command_revision ?? "n/a"}`,
      `transaction ${step.trace.transaction_id}`,
    ],
    causeRows: causeRows(step.trace),
  };
}

function scopeDetail(trace: ContractTrace, event?: ScopeEvent): SelectionDetail {
  if (!event) return missingDetail(trace);
  return {
    title: `${event.kind} scope ${event.scope}`,
    rows: [`scope ${event.scope}`, `revision ${trace.revision}`, `transaction ${trace.transaction_id}`],
    causeRows: causeRows(trace),
  };
}

function missingDetail(trace: ContractTrace): SelectionDetail {
  return {
    title: "No detail",
    rows: [`tx ${trace.transaction_id}`, `revision ${trace.revision}`],
    causeRows: causeRows(trace),
  };
}

function causeRows(trace: ContractTrace) {
  const inputs = trace.staged_input_changes.map((change) => `input node ${change.node}: ${change.outcome}`);
  const diffs = trace.collection_diffs
    .filter((diff) => diff.added || diff.removed || diff.updated)
    .map((diff) => `collection ${diff.node}: +${diff.added} -${diff.removed} ~${diff.updated}`);
  const explanations = trace.audit_explanations;
  const audit = explanations.resource_commands.length + explanations.output_frames.length;
  return [
    ...inputs,
    ...diffs,
    `audit level ${explanations.level}`,
    `audit explanations ${audit}`,
  ];
}
