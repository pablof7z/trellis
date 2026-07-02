import Editor from "@monaco-editor/react";
import { activeContent } from "./engineClient";
import type { AppState, Diagnostic } from "./types";

export function EditorPane({ state }: { state: AppState }) {
  const path = state.inputs.activeEditor ?? "src/app.tl";
  const diagnostics = state.outputLedger.diagnosticsByFile[path] ?? [];
  const links = state.outputLedger.linksByFile[path] ?? [];
  const tokens = state.outputLedger.tokensByFile[path] ?? [];
  return (
    <section className="editor-wrap">
      <div className="editor-head">
        <span>{path}</span>
        <span>{diagnostics.length} diagnostics</span>
        <span>{links.length} links</span>
        <span>{tokens.length} semantic tokens</span>
      </div>
      <div className="editor-grid">
        <div className="monaco-box">
          <Editor
            height="100%"
            language="rust"
            theme="vs-dark"
            value={activeContent(state)}
            options={{
              readOnly: true,
              minimap: { enabled: false },
              fontSize: 13,
              lineNumbersMinChars: 3,
              scrollBeyondLastLine: false,
              wordWrap: "on",
            }}
          />
        </div>
        <div className="inline-output">
          <div className="subhead">Inline diagnostics</div>
          {diagnostics.map((diagnostic) => (
            <DiagnosticRow key={diagnostic.id} diagnostic={diagnostic} />
          ))}
          {diagnostics.length === 0 && <div className="empty">No visible diagnostics</div>}
          <div className="subhead">Document links</div>
          {links.map((link) => (
            <div className={`link-row ${link.status}`} key={link.id}>
              line {link.line}: {link.targetPath}
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

function DiagnosticRow({ diagnostic }: { diagnostic: Diagnostic }) {
  return (
    <div className="diag-row">
      <span>{diagnostic.line}:{diagnostic.column}</span>
      <strong>{diagnostic.source}</strong>
      {diagnostic.message}
    </div>
  );
}
