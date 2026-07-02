import { useEffect, useRef } from "react";
import Editor, { type Monaco, type OnMount } from "@monaco-editor/react";
import { activeContent } from "./engineClient";
import type { AppState, Diagnostic, DocumentLink } from "./types";

export function EditorPane({ state }: { state: AppState }) {
  const path = state.inputs.activeEditor ?? "src/app.tl";
  const diagnostics = state.outputLedger.diagnosticsByFile[path] ?? [];
  const links = state.outputLedger.linksByFile[path] ?? [];
  const tokens = state.outputLedger.tokensByFile[path] ?? [];
  const editorRef = useRef<Parameters<OnMount>[0] | null>(null);
  const monacoRef = useRef<Monaco | null>(null);
  const decorationRef = useRef<ReturnType<Parameters<OnMount>[0]["createDecorationsCollection"]> | null>(null);

  useEffect(() => {
    const editor = editorRef.current;
    const monaco = monacoRef.current;
    const model = editor?.getModel();
    if (!monaco || !model) return;
    monaco.editor.setModelMarkers(model, "trellis", diagnostics.map((diagnostic) => marker(monaco, diagnostic)));
    decorationRef.current?.set(diagnostics.map((diagnostic) => decoration(monaco, diagnostic)));
  }, [diagnostics]);

  const handleMount: OnMount = (editor, monaco) => {
    editorRef.current = editor;
    monacoRef.current = monaco;
    decorationRef.current = editor.createDecorationsCollection();
    monaco.editor.setModelMarkers(editor.getModel(), "trellis", diagnostics.map((diagnostic) => marker(monaco, diagnostic)));
    decorationRef.current.set(diagnostics.map((diagnostic) => decoration(monaco, diagnostic)));
  };

  return (
    <section className="editor-wrap">
      <div className="editor-head">
        <strong>{path}</strong>
        <span>{diagnostics.length} diagnostics</span>
        <span>{links.length} links</span>
        <span>{tokens.length} semantic tokens</span>
      </div>
      <div className="editor-body">
        <Editor
          height="100%"
          language="rust"
          theme="vs-dark"
          value={activeContent(state)}
          onMount={handleMount}
          options={{
            readOnly: true,
            minimap: { enabled: false },
            fontSize: 13,
            lineHeight: 21,
            lineNumbersMinChars: 3,
            renderLineHighlight: "all",
            scrollBeyondLastLine: false,
            wordWrap: "on",
            glyphMargin: true,
            folding: false,
            overviewRulerBorder: false,
          }}
        />
        <div className="line-diagnostics">
          {diagnostics.slice(0, 3).map((diagnostic) => (
            <DiagnosticRow key={diagnostic.id} diagnostic={diagnostic} />
          ))}
        </div>
        <div className="editor-overlays">
          {diagnostics.length === 0 && <div className="editor-note success">No visible diagnostics in this editor.</div>}
          <LinkSummary links={links} />
        </div>
      </div>
    </section>
  );
}

function DiagnosticRow({ diagnostic }: { diagnostic: Diagnostic }) {
  return (
    <div className="diag-row" style={{ top: `${6 + (diagnostic.line - 1) * 21}px` }}>
      <span>{diagnostic.line}:{diagnostic.column}</span>
      {diagnostic.message}
    </div>
  );
}

function LinkSummary({ links }: { links: DocumentLink[] }) {
  const missing = links.filter((link) => link.status === "missing").length;
  return (
    <div className={missing ? "editor-note warn" : "editor-note"}>
      {links.length} import links · {missing ? `${missing} missing target` : "all resolved"}
    </div>
  );
}

function marker(monaco: Monaco, diagnostic: Diagnostic) {
  return {
    severity: monaco.MarkerSeverity.Error,
    message: diagnostic.message,
    source: diagnostic.source,
    startLineNumber: diagnostic.line,
    startColumn: diagnostic.column,
    endLineNumber: diagnostic.line,
    endColumn: diagnostic.column + Math.max(4, diagnostic.message.split(" ")[0].length),
  };
}

function decoration(monaco: Monaco, diagnostic: Diagnostic) {
  const endColumn = diagnostic.column + Math.max(4, diagnostic.message.split(" ")[0].length);
  return {
    range: new monaco.Range(diagnostic.line, diagnostic.column, diagnostic.line, endColumn),
    options: {
      glyphMarginClassName: "diagnostic-glyph",
      inlineClassName: "diagnostic-squiggle",
      hoverMessage: { value: diagnostic.message },
    },
  };
}
