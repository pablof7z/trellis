use crate::types::{CompilerConfig, Diagnostic, DocumentLink, FilePath, SemanticToken};

pub fn imports(path: &str, contents: &str, files: &[FilePath]) -> Vec<DocumentLink> {
    contents
        .lines()
        .enumerate()
        .filter_map(|(line_idx, line)| {
            let start = line.find("import \"")? + 8;
            let rest = &line[start..];
            let end = rest.find('"')?;
            let raw = &rest[..end];
            let target = resolve(path, raw);
            Some(DocumentLink {
                id: format!("link:{path}:{}:{target}", line_idx + 1),
                file_path: path.to_owned(),
                target_path: target.clone(),
                line: line_idx + 1,
                column_start: start + 1,
                column_end: start + end + 1,
                status: if files.contains(&target) {
                    "resolved"
                } else {
                    "missing"
                }
                .to_owned(),
            })
        })
        .collect()
}

pub fn diagnostics(
    path: &str,
    contents: &str,
    links: &[DocumentLink],
    config: &CompilerConfig,
    has_verified_field: bool,
) -> Vec<Diagnostic> {
    let mut result = Vec::new();
    for link in links {
        if link.status == "missing" {
            result.push(diag(
                path,
                link.line,
                link.column_start,
                "resolver",
                format!("Missing import target {}", link.target_path),
            ));
        }
    }
    for (idx, line) in contents.lines().enumerate() {
        if line.contains("TODO_ERROR") {
            result.push(diag(path, idx + 1, 1, "parser", "TODO_ERROR marker"));
        }
        if line.contains("SYNTAX_ERROR") {
            result.push(diag(path, idx + 1, 1, "parser", "SYNTAX_ERROR marker"));
        }
        if matches!(config, CompilerConfig::Strict) && line.contains("add(1, \"two\")") {
            result.push(diag(
                path,
                idx + 1,
                line.find("add").unwrap_or(0) + 1,
                "typecheck",
                "Type mismatch: add(number, string)",
            ));
        }
        if line.contains("user.email_verified") && !has_verified_field {
            result.push(diag(
                path,
                idx + 1,
                line.find("email_verified").unwrap_or(0) + 1,
                "typecheck",
                "Unknown field email_verified",
            ));
        }
    }
    result
}

pub fn semantic_tokens(path: &str, contents: &str) -> Vec<SemanticToken> {
    let mut tokens = Vec::new();
    for (line_idx, line) in contents.lines().enumerate() {
        for (needle, token_type) in [
            ("module", "keyword"),
            ("import", "keyword"),
            ("let", "keyword"),
            ("fn", "keyword"),
            ("type", "keyword"),
            ("TODO_ERROR", "error"),
            ("SYNTAX_ERROR", "error"),
        ] {
            if let Some(start) = line.find(needle) {
                tokens.push(token(
                    path,
                    line_idx + 1,
                    start + 1,
                    needle.len(),
                    token_type,
                ));
            }
        }
        if let Some(start) = line.find('"')
            && let Some(end) = line[start + 1..].find('"')
        {
            tokens.push(token(path, line_idx + 1, start + 1, end + 2, "string"));
        }
    }
    tokens
}

fn resolve(path: &str, raw: &str) -> FilePath {
    let dir = path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("");
    let clean = raw.strip_prefix("./").unwrap_or(raw);
    if dir.is_empty() {
        clean.to_owned()
    } else {
        format!("{dir}/{clean}")
    }
}

fn diag(
    path: &str,
    line: usize,
    column: usize,
    source: &str,
    message: impl ToString,
) -> Diagnostic {
    let message = message.to_string();
    Diagnostic {
        id: format!("diag:{path}:{line}:{column}:{source}:{message}"),
        file_path: path.to_owned(),
        line,
        column,
        severity: "error".to_owned(),
        message,
        source: source.to_owned(),
    }
}

fn token(path: &str, line: usize, column: usize, width: usize, token_type: &str) -> SemanticToken {
    SemanticToken {
        id: format!("tok:{path}:{line}:{column}:{token_type}"),
        file_path: path.to_owned(),
        line,
        column_start: column,
        column_end: column + width,
        token_type: token_type.to_owned(),
    }
}
