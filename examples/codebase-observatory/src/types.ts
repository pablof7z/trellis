export type AppState = {
  mode: "trellis" | "naive";
  bugPolicy: Record<string, boolean>;
  inputs: {
    activeBranch: string;
    files: Record<string, { path: string; contents: string; generated: boolean }>;
    openEditors: string[];
    activeEditor: string | null;
    compilerConfig: "strict" | "loose";
    generatedFilesEnabled: boolean;
    scenarioRevision: number;
  };
  full: {
    sourceFiles: string[];
    importEdges: string[];
    diagnosticsByFile: Record<string, Diagnostic[]>;
    linksByFile: Record<string, DocumentLink[]>;
    tokensByFile: Record<string, SemanticToken[]>;
    desiredResources: string[];
    moduleGraph: string[];
  };
  resourceLedger: Record<string, ResourceEntry>;
  outputLedger: {
    diagnosticsByFile: Record<string, Diagnostic[]>;
    linksByFile: Record<string, DocumentLink[]>;
    tokensByFile: Record<string, SemanticToken[]>;
    revisionsByOutputKey: Record<string, number>;
  };
  traces: TransactionTrace[];
  actionLog: Record<string, unknown>[];
  selectedWhy: string | null;
  replayResult: ReplayResult | null;
};

export type Diagnostic = {
  id: string;
  filePath: string;
  line: number;
  column: number;
  severity: string;
  message: string;
  source: string;
};

export type DocumentLink = {
  id: string;
  filePath: string;
  targetPath: string;
  line: number;
  columnStart: number;
  columnEnd: number;
  status: string;
};

export type SemanticToken = {
  id: string;
  filePath: string;
  line: number;
  columnStart: number;
  columnEnd: number;
  tokenType: string;
};

export type ResourceEntry = {
  key: string;
  state: string;
  owners: string[];
  openCount: number;
  closeCount: number;
  cancelCount: number;
  lastCommandRevision: number;
  lastTxId: number;
  cause: string;
};

export type TransactionTrace = {
  txId: number;
  revision: number;
  coreBacked: boolean;
  coreTransactionId: number | null;
  coreRevision: number | null;
  label: string;
  inputChanges: { key: string; before: string; after: string }[];
  changedNodes: { id: string; summary: string }[];
  collectionDiffs: { collection: string; added: string[]; removed: string[]; updated: string[] }[];
  resourceCommands: TraceItem[];
  outputFrames: TraceItem[];
  scopeEvents: { op: string; scope: string; reason: string }[];
  hostStatusEvents: {
    status: { kind: string; path: string; commandRevision: number };
    classification: string;
    reason: string;
    effect: string;
  }[];
  invariantChecks: { id: string; label: string; status: "pass" | "fail"; details: string }[];
  auditEdges: string[];
};

export type TraceItem = {
  kind?: string;
  op?: string;
  key?: string;
  outputKey?: string;
  scope: string;
  revision?: number;
  commandRevision?: number;
  filePath?: string;
  cause: {
    inputKey: string;
    before: string;
    after: string;
    changedNode: string;
    collection: string;
    reason: string;
    path: string[];
  };
};

export type ReplayResult = {
  status: "pass" | "fail";
  traceLength: number;
  finalObservableMatches: boolean;
  checks: TransactionTrace["invariantChecks"];
};

export type EngineApi = {
  initialState: () => AppState;
  dispatch: (state: AppState, action: Record<string, unknown>) => AppState;
  replay: (state: AppState) => ReplayResult;
};
