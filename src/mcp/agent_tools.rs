use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use adk_rust::prelude::*;
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Value, json};

use crate::cache::{Cache, query};
use crate::fs;

use super::evidence::EvidenceCollector;

const LIST_FILES_MAX_CHARS: usize = 40_000;
const READ_FILE_MAX_CHARS: usize = 80_000;
const FIND_SYMBOL_MAX_HITS: usize = 50;
const SESSION_CHAR_BUDGET: usize = 200_000;

pub fn build_all(
    root: PathBuf,
    cache: Arc<Mutex<Cache>>,
    collector: Arc<Mutex<EvidenceCollector>>,
    session_chars: Arc<Mutex<usize>>,
) -> Vec<Arc<dyn Tool>> {
    vec![
        list_files(root.clone(), session_chars.clone()),
        find_symbol(cache, session_chars.clone()),
        read_file(root, collector, session_chars),
    ]
}

#[derive(JsonSchema, Serialize)]
struct ListFilesArgs {
    /// Optional substring filter on relative paths (e.g. "src/" or ".rs").
    #[serde(skip_serializing_if = "Option::is_none")]
    path_contains: Option<String>,
}

fn list_files(root: PathBuf, session_chars: Arc<Mutex<usize>>) -> Arc<dyn Tool> {
    let handler = move |_ctx: Arc<dyn ToolContext>, args: Value| {
        let root = root.clone();
        let session_chars = session_chars.clone();
        async move {
            let filter = args
                .get("path_contains")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let mut files = fs::list(&root);
            if let Some(f) = &filter {
                files.retain(|p| p.contains(f.as_str()));
            }
            files.sort();

            let total = files.len();
            let mut acc = 0usize;
            let mut included: Vec<String> = Vec::new();
            for p in &files {
                let cost = p.len() + 1;
                if acc + cost > LIST_FILES_MAX_CHARS {
                    break;
                }
                acc += cost;
                included.push(p.clone());
            }
            let omitted = total.saturating_sub(included.len());

            bump_session(&session_chars, acc);
            let budget = session_budget_note(&session_chars);

            Ok(json!({
                "files": included,
                "count": included.len(),
                "total": total,
                "omitted": omitted,
                "filter": filter,
                "session_budget": budget,
            }))
        }
    };
    Arc::new(
        FunctionTool::new(
            "list_files",
            "List project files (gitignore-aware). Optional `path_contains` substring filter. \
             Returns relative paths only — no file contents. Use this to discover what exists; \
             read_file the candidates you actually need.",
            handler,
        )
        .with_parameters_schema::<ListFilesArgs>(),
    )
}

#[derive(JsonSchema, Serialize)]
struct FindSymbolArgs {
    /// Symbol name. Substring match (case-sensitive). e.g. "build_runner" or "Cache".
    name: String,
    /// Optional kind filter: function, method, class, struct, enum, trait, type, const, etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<String>,
    /// Optional substring filter on file path (e.g. "src/chat/" or ".rs").
    #[serde(skip_serializing_if = "Option::is_none")]
    path_contains: Option<String>,
}

fn find_symbol(
    cache: Arc<Mutex<Cache>>,
    session_chars: Arc<Mutex<usize>>,
) -> Arc<dyn Tool> {
    let handler = move |_ctx: Arc<dyn ToolContext>, args: Value| {
        let cache = cache.clone();
        let session_chars = session_chars.clone();
        async move {
            let name = args
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                return Ok(json!({ "error": "missing 'name' argument" }));
            }
            let kind = args
                .get("kind")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let path_contains = args
                .get("path_contains")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let guard = cache.lock().expect("cache mutex poisoned");
            let value = query::find_symbol(
                &guard,
                &name,
                kind.as_deref(),
                path_contains.as_deref(),
                FIND_SYMBOL_MAX_HITS,
            )
            .unwrap_or_else(|e| json!({ "error": format!("find_symbol failed: {e:#}") }));

            let cost = value.to_string().len();
            bump_session(&session_chars, cost);
            let mut value = value;
            if let Some(obj) = value.as_object_mut() {
                obj.insert(
                    "session_budget".to_string(),
                    Value::String(session_budget_note(&session_chars)),
                );
                obj.insert(
                    "note".to_string(),
                    Value::String(
                        "Locations only — call read_file to inspect the source before quoting."
                            .to_string(),
                    ),
                );
            }
            Ok(value)
        }
    };
    Arc::new(
        FunctionTool::new(
            "find_symbol",
            "Find where a symbol is defined. Searches the tree-sitter symbol index \
             (Rust / Python / TypeScript / JavaScript). \
             Returns {path, name, kind, start_line, end_line} locations. \
             Substring match on name. Optional `kind` and `path_contains` filters narrow results. \
             This returns POINTERS only — call read_file on the path:start_line region to see actual code.",
            handler,
        )
        .with_parameters_schema::<FindSymbolArgs>(),
    )
}

#[derive(JsonSchema, Serialize)]
struct ReadFileArgs {
    /// Path relative to project root, e.g. "src/main.rs".
    path: String,
    /// First line to include (1-based, inclusive). Defaults to 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    start_line: Option<u64>,
    /// Last line to include (1-based, inclusive). Defaults to end of file.
    #[serde(skip_serializing_if = "Option::is_none")]
    end_line: Option<u64>,
}

fn read_file(
    root: PathBuf,
    collector: Arc<Mutex<EvidenceCollector>>,
    session_chars: Arc<Mutex<usize>>,
) -> Arc<dyn Tool> {
    let handler = move |_ctx: Arc<dyn ToolContext>, args: Value| {
        let root = root.clone();
        let collector = collector.clone();
        let session_chars = session_chars.clone();
        async move {
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if path.is_empty() {
                return Ok(json!({ "error": "missing 'path' argument" }));
            }
            let start = args
                .get("start_line")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize);
            let end = args
                .get("end_line")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize);

            let span = match fs::read_span(&root, &path, start, end) {
                Ok(s) => s,
                Err(e) => return Ok(json!({ "path": path, "error": e })),
            };

            let (content, truncated, returned_end, total_bytes) = cap_content(&span.content);
            let actual_end_line = if truncated {
                // Recompute end_line from the truncated content's last newline.
                let lines_in_slice = content.lines().count().max(1);
                span.start_line + lines_in_slice - 1
            } else {
                span.end_line
            };

            {
                let mut c = collector.lock().expect("collector mutex poisoned");
                c.record_read(&path, span.start_line, actual_end_line);
            }

            bump_session(&session_chars, content.len());

            let mut out = json!({
                "path": span.path,
                "start_line": span.start_line,
                "end_line": actual_end_line,
                "total_lines": span.total_lines,
                "content": content,
                "session_budget": session_budget_note(&session_chars),
            });
            if truncated {
                if let Some(obj) = out.as_object_mut() {
                    obj.insert(
                        "truncated".to_string(),
                        Value::String(format!(
                            "... file continues, returned {} of {} bytes. Call read_file again with start_line={} to continue.",
                            content.len(),
                            total_bytes,
                            actual_end_line + 1
                        )),
                    );
                }
            }
            let _ = returned_end;
            Ok(out)
        }
    };
    Arc::new(
        FunctionTool::new(
            "read_file",
            "Read UTF-8 text from a project file. Optional `start_line` and `end_line` (1-based, inclusive). \
             Returns the actual source bytes. \
             This is the verification step — never claim a function/flag/command exists without reading it here first. \
             Truncates at 80k characters with a continuation marker.",
            handler,
        )
        .with_parameters_schema::<ReadFileArgs>(),
    )
}

fn cap_content(content: &str) -> (String, bool, usize, usize) {
    let total = content.len();
    if total <= READ_FILE_MAX_CHARS {
        return (content.to_string(), false, total, total);
    }
    let mut cut = READ_FILE_MAX_CHARS;
    while cut > 0 && !content.is_char_boundary(cut) {
        cut -= 1;
    }
    if let Some(nl) = content[..cut].rfind('\n') {
        cut = nl;
    }
    (content[..cut].to_string(), true, cut, total)
}

fn bump_session(counter: &Arc<Mutex<usize>>, by: usize) {
    let mut g = counter.lock().expect("session counter mutex poisoned");
    *g = g.saturating_add(by);
}

fn session_budget_note(counter: &Arc<Mutex<usize>>) -> String {
    let used = *counter.lock().expect("session counter mutex poisoned");
    format!("{used} / {SESSION_CHAR_BUDGET} chars used")
}
