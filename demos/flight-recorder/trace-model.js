import {
  FORMAT_VERSION,
  nodeLabel,
  normalizeLabelRegistry,
  outputLabel,
  resourceLabel,
  scopeLabel,
  validateLabelRegistry,
} from "./trace-labels.js";

export { FORMAT_VERSION };

export const bundledTraces = [
  ["normal-session", "Normal session", "/demos/flight-recorder/traces/normal-session.json"],
  ["seeded-leak", "Seeded leak", "/demos/flight-recorder/traces/seeded-leak.json"],
  ["teardown-cascade", "Teardown cascade", "/demos/flight-recorder/traces/teardown-cascade.json"],
];

const requiredTraceArrays = [
  "changed_inputs",
  "collection_diffs",
  "resource_commands",
  "output_frames",
  "phase_trace",
  "invariant_results",
];

export function normalizeTraceEnvelope(candidate, source) {
  const errors = validateTrace(candidate);
  if (errors.length) return { errors };
  const provenance = provenanceFor(candidate, source);
  const labelRegistry = normalizeLabelRegistry(candidate.labelRegistry);
  const steps = candidate.steps.map((step, index) => normalizeStep(step, index, labelRegistry));
  return {
    errors: [],
    trace: {
      formatVersion: candidate.formatVersion,
      title: source.label,
      provenance,
      labelRegistry,
      steps,
      raw: candidate,
      invariantSummary: summarizeTraceInvariants(steps),
    },
  };
}

export function validateTrace(candidate) {
  const errors = [];
  if (!candidate || typeof candidate !== "object" || Array.isArray(candidate)) {
    return ["root: expected a JSON object trace envelope"];
  }
  if (candidate.formatVersion !== FORMAT_VERSION) {
    errors.push(`formatVersion: unsupported value ${formatValue(candidate.formatVersion)}; expected ${FORMAT_VERSION}`);
  }
  if (!Array.isArray(candidate.steps) || candidate.steps.length === 0) {
    errors.push("steps: missing required non-empty array");
    return errors;
  }
  validateLabelRegistry(candidate.labelRegistry, errors);
  candidate.steps.forEach((step, index) => {
    if (!step || typeof step !== "object") {
      errors.push(`steps[${index}]: expected an object`);
      return;
    }
    if (!step.trace || typeof step.trace !== "object" || Array.isArray(step.trace)) {
      errors.push(`steps[${index}].trace: missing required object`);
      return;
    }
    for (const field of requiredTraceArrays) {
      if (!Array.isArray(step.trace[field])) {
        errors.push(`steps[${index}].trace.${field}: missing required array`);
      }
    }
    if (step.trace.transaction_id == null) {
      errors.push(`steps[${index}].trace.transaction_id: missing required value`);
    }
    if (step.trace.revision == null) {
      errors.push(`steps[${index}].trace.revision: missing required value`);
    }
    if (!step.trace.audit_explanations || typeof step.trace.audit_explanations !== "object" || Array.isArray(step.trace.audit_explanations)) {
      errors.push(`steps[${index}].trace.audit_explanations: missing required object`);
    } else {
      for (const field of ["node_changes", "resource_commands", "output_frames"]) {
        if (!Array.isArray(step.trace.audit_explanations[field])) {
          errors.push(`steps[${index}].trace.audit_explanations.${field}: missing required array`);
        }
      }
    }
  });
  return errors;
}

export function itemMatchesQuery(item, query) {
  if (!query) return true;
  return JSON.stringify(item).toLowerCase().includes(query);
}

export function kindMatches(command, kindFilter) {
  const kind = command.kind.toLowerCase();
  if (kindFilter === "all") return true;
  if (kindFilter === "other") return !["open", "close"].includes(kind);
  return kind === kindFilter;
}

export function invariantMatches(check, filter) {
  if (filter === "all") return true;
  return filter === "fail" ? !check.passed : check.passed;
}

export function evidenceLabel(selection) {
  if (!selection || selection.type === "transaction") return "transaction receipt";
  return `${selection.type} ${selection.index + 1}`;
}

export function replayStatus(trace) {
  const replay = trace?.provenance?.replay;
  if (!replay || replay.status === "unavailable") {
    return { status: "unavailable", label: "replay unavailable" };
  }
  return {
    status: replay.status === "pass" ? "pass" : "fail",
    label: `replay ${replay.status === "pass" ? "pass" : "fail"}`,
    checks: replay.checks ?? [],
  };
}

function normalizeStep(step, index, labels) {
  const trace = step.trace;
  return {
    index,
    name: step.name || `transaction ${index + 1}`,
    raw: step,
    txId: trace.transaction_id,
    revision: trace.revision,
    coreBacked: Boolean(trace.core_backed ?? trace.coreBacked ?? false),
    coreTransactionId: trace.core_transaction_id ?? trace.coreTransactionId ?? null,
    coreRevision: trace.core_revision ?? trace.coreRevision ?? null,
    changedInputs: trace.changed_inputs.map((input) => nodeLabel(labels, input)),
    stagedInputChanges: trace.staged_input_changes ?? [],
    dirtyRoots: trace.dirty_roots ?? [],
    recomputedDerivedNodes: trace.recomputed_derived_nodes ?? [],
    changedDerivedNodes: trace.changed_derived_nodes ?? [],
    recomputedCollectionNodes: trace.recomputed_collection_nodes ?? [],
    changedCollectionNodes: trace.changed_collection_nodes ?? [],
    collectionDiffs: trace.collection_diffs.map((diff) => normalizeCollectionDiff(diff, labels)),
    resourceCommands: trace.resource_commands.map((command) => normalizeResourceCommand(command, labels)),
    outputFrames: trace.output_frames.map((frame) => normalizeOutputFrame(frame, labels)),
    scopeEvents: (trace.scope_events ?? []).map((event) => normalizeScopeEvent(event, labels)),
    auditLog: trace.audit_log ?? [],
    auditExplanations: trace.audit_explanations,
    phaseTrace: trace.phase_trace.map(String),
    invariantResults: trace.invariant_results.map(normalizeInvariant),
  };
}

function normalizeCollectionDiff(diff, labels) {
  const node = diff.node ?? diff.collection ?? "unknown";
  return {
    node: nodeLabel(labels, node),
    kind: diff.kind ?? "Collection",
    added: countOrList(diff.added),
    removed: countOrList(diff.removed),
    updated: countOrList(diff.updated),
    unchanged: Number(diff.unchanged ?? 0),
    raw: diff,
  };
}

function normalizeResourceCommand(command, labels) {
  const key = command.key ?? command.output_key ?? "unknown";
  const scope = command.scope ?? "unknown";
  return {
    kind: command.kind ?? command.op ?? "Unknown",
    transitionPolicy: command.transition_policy ?? "Unknown",
    key: resourceLabel(labels, key),
    rawKey: key,
    scope: scopeLabel(labels, scope),
    rawScope: scope,
    revision: command.command_revision ?? command.revision ?? null,
    cause: command.cause ?? null,
    raw: command,
  };
}

function normalizeOutputFrame(frame, labels) {
  const key = frame.output_key ?? frame.outputKey ?? "unknown";
  const scope = frame.scope ?? "unknown";
  return {
    kind: frame.kind ?? "OutputFrame",
    key: outputLabel(labels, key),
    rawKey: key,
    scope: scopeLabel(labels, scope),
    rawScope: scope,
    revision: frame.revision ?? null,
    filePath: frame.file_path ?? frame.filePath ?? null,
    diagnostics: frame.diagnostics ?? [],
    links: frame.links ?? [],
    tokens: frame.tokens ?? [],
    status: frame.status ?? null,
    cause: frame.cause ?? null,
    raw: frame,
  };
}

function normalizeScopeEvent(event, labels) {
  const scope = event.scope ?? "unknown";
  return {
    kind: event.kind ?? event.op ?? "ScopeEvent",
    scope: scopeLabel(labels, scope),
    rawScope: scope,
    reason: event.reason ?? "",
    raw: event,
  };
}

function normalizeInvariant(check) {
  const passed = check.passed ?? check.status === "pass";
  return {
    name: check.name ?? check.label ?? check.id ?? "unnamed invariant",
    passed: Boolean(passed),
    details: check.details ?? "",
    raw: check,
  };
}

function provenanceFor(candidate, source) {
  const raw = candidate.provenance ?? {};
  const sourceType = source.kind === "uploaded" ? "uploadedTrace" : "bundledFixture";
  return {
    sourceType,
    sourceLabel: source.kind === "uploaded" ? "[UPLOADED TRACE]" : "[BUNDLED FIXTURE]",
    generator: raw.generator ?? raw.generatorCommand ?? "unknown",
    repoCommit: raw.repoCommit ?? raw.commit ?? "unknown",
    buildId: raw.buildId ?? "unknown",
    coreBacked: Boolean(raw.coreBacked ?? candidate.steps.some((step) => step.trace.core_backed)),
    replay: raw.replay ?? { status: "unavailable", reason: "no deterministic replay capsule is bundled with this trace" },
  };
}

function summarizeTraceInvariants(steps) {
  const checks = steps.flatMap((step) => step.invariantResults);
  const failed = checks.filter((check) => !check.passed).length;
  return {
    total: checks.length,
    failed,
    label: checks.length ? `${checks.length - failed} pass / ${failed} fail` : "none recorded",
  };
}

function countOrList(value) {
  if (Array.isArray(value)) return value;
  return Number(value ?? 0);
}

function formatValue(value) {
  return value == null ? "missing" : JSON.stringify(value);
}
