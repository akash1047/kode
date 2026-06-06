use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use adk_rust::prelude::*;
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Value, json};

use crate::cache::{Cache, query};
use crate::fs;

const DEFAULT_SEARCH_HITS: usize = 100;
const LIST_FILES_MAX_CHARS: usize = 40_000;

#[derive(JsonSchema, Serialize)]
struct ReadRequest {
    /// Path relative to project root, e.g. "src/main.rs".
    path: String,
    /// First line to include (1-based, inclusive). Defaults to 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    start_line: Option<u64>,
    /// Last line to include (1-based, inclusive). Defaults to end of file.
    #[serde(skip_serializing_if = "Option::is_none")]
    end_line: Option<u64>,
}

#[derive(JsonSchema, Serialize)]
struct ReadArgs {
    /// One or more files to read. Batch as many as you need in a single call to minimize tool hops.
    reads: Vec<ReadRequest>,
}

#[derive(JsonSchema, Serialize)]
struct SearchArgs {
    /// Rust-flavored regex pattern.
    pattern: String,
    /// Optional substring filter on relative paths (e.g. ".py" or "src/services/").
    #[serde(skip_serializing_if = "Option::is_none")]
    path_contains: Option<String>,
    /// Cap on hits returned. Defaults to 100.
    #[serde(skip_serializing_if = "Option::is_none")]
    max_hits: Option<u64>,
    /// Case-insensitive matching. Defaults to false.
    #[serde(skip_serializing_if = "Option::is_none")]
    case_insensitive: Option<bool>,
}

#[derive(JsonSchema, Serialize)]
struct ListFilesArgs {
    /// Optional substring filter on relative paths (e.g. "src/" or ".rs").
    #[serde(skip_serializing_if = "Option::is_none")]
    path_contains: Option<String>,
}

#[derive(JsonSchema, Serialize)]
struct FindSymbolArgs {
    /// Symbol name to look for. Substring match (case-sensitive). e.g. "build_runner" or "Cache".
    name: String,
    /// Optional kind filter: function | method | class | struct | enum | trait | type | const | macro | module | interface | impl | export | import.
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<String>,
    /// Optional substring filter on file path (e.g. "src/chat/" or ".rs").
    #[serde(skip_serializing_if = "Option::is_none")]
    path_contains: Option<String>,
    /// Cap on hits. Defaults to 100.
    #[serde(skip_serializing_if = "Option::is_none")]
    max_hits: Option<u64>,
}

pub fn build_all(root: PathBuf, cache: Arc<Mutex<Cache>>) -> Vec<Arc<dyn Tool>> {
    vec![
        list_files(root.clone()),
        find_symbol(cache),
        read_project_files(root.clone()),
        search_project(root),
    ]
}

fn list_files(root: PathBuf) -> Arc<dyn Tool> {
    let handler = move |_ctx: Arc<dyn ToolContext>, args: Value| {
        let root = root.clone();
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

            Ok(json!({
                "files": included,
                "count": included.len(),
                "total": total,
                "omitted": omitted,
                "filter": filter,
            }))
        }
    };
    Arc::new(
        FunctionTool::new(
            "list_files",
            "List project files (gitignore-aware). Optional `path_contains` substring filter on paths (e.g. \"src/commands/\" or \".rs\"). \
             Returns relative paths only — no file contents. \
             Use this to DISCOVER what files exist before reading. \
             search_project does NOT find files by name — it searches file contents; for filename lookups always use list_files.",
            handler,
        )
        .with_parameters_schema::<ListFilesArgs>(),
    )
}

fn find_symbol(cache: Arc<Mutex<Cache>>) -> Arc<dyn Tool> {
    let handler = move |_ctx: Arc<dyn ToolContext>, args: Value| {
        let cache = cache.clone();
        async move {
            let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if name.is_empty() {
                return Ok(json!({ "error": "missing 'name' argument" }));
            }
            let kind = args.get("kind").and_then(|v| v.as_str()).map(|s| s.to_string());
            let path_contains = args
                .get("path_contains")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let limit = args
                .get("max_hits")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize)
                .unwrap_or(100);

            let guard = cache.lock().expect("cache mutex poisoned");
            let value = query::find_symbol(&guard, &name, kind.as_deref(), path_contains.as_deref(), limit)
                .unwrap_or_else(|e| json!({ "error": format!("find_symbol failed: {e:#}") }));
            Ok(value)
        }
    };
    Arc::new(
        FunctionTool::new(
            "find_symbol",
            "Locate a symbol definition by name in the pre-built tree-sitter index (Rust/Python/TypeScript/JavaScript/Go/Java/C/C++/C#/Ruby). \
             Returns {path, name, kind, start_line, end_line} hits — locations only, not source. \
             Substring match on name. Filter by `kind` or `path_contains` to narrow. \
             You MUST call read_project_files on the hits before quoting anything from them.",
            handler,
        )
        .with_parameters_schema::<FindSymbolArgs>(),
    )
}

fn read_project_files(root: PathBuf) -> Arc<dyn Tool> {
    let handler = move |_ctx: Arc<dyn ToolContext>, args: Value| {
        let root = root.clone();
        async move {
            let reads = args.get("reads").and_then(|v| v.as_array()).cloned();
            let Some(reads) = reads else {
                return Ok(json!({ "error": "missing 'reads' array argument" }));
            };
            if reads.is_empty() {
                return Ok(json!({ "error": "'reads' array is empty" }));
            }

            let mut results = Vec::with_capacity(reads.len());
            for req in reads {
                let path = req.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string();
                if path.is_empty() {
                    results.push(json!({ "error": "missing 'path' in read request" }));
                    continue;
                }
                let start = req.get("start_line").and_then(|v| v.as_u64()).map(|n| n as usize);
                let end = req.get("end_line").and_then(|v| v.as_u64()).map(|n| n as usize);

                results.push(match fs::read_span(&root, &path, start, end) {
                    Ok(span) => json!({
                        "path": span.path,
                        "start_line": span.start_line,
                        "end_line": span.end_line,
                        "total_lines": span.total_lines,
                        "content": span.content,
                    }),
                    Err(e) => json!({ "path": path, "error": e }),
                });
            }

            Ok(json!({ "results": results, "count": results.len() }))
        }
    };
    Arc::new(
        FunctionTool::new(
            "read_project_files",
            "Read UTF-8 text from one or many project files in a single call. Pass `reads` as an array of {path, start_line?, end_line?}. \
             BATCH AGGRESSIVELY — combining 5 reads into one call costs 1 tool hop instead of 5. \
             Provide start_line/end_line (1-based, inclusive) to read just a span when you know the region from find_symbol or search_project hits. \
             This is the only source of truth — quote and cite from what this returns, never from filenames or symbol names alone.",
            handler,
        )
        .with_parameters_schema::<ReadArgs>(),
    )
}

fn search_project(root: PathBuf) -> Arc<dyn Tool> {
    let handler = move |_ctx: Arc<dyn ToolContext>, args: Value| {
        let root = root.clone();
        async move {
            let pattern = args["pattern"].as_str().unwrap_or("").to_string();
            if pattern.is_empty() {
                return Ok(json!({ "error": "missing 'pattern' argument" }));
            }
            let path_contains = args
                .get("path_contains")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let max_hits = args
                .get("max_hits")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize)
                .unwrap_or(DEFAULT_SEARCH_HITS);
            let case_insensitive = args
                .get("case_insensitive")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let opts = fs::SearchOpts {
                pattern: pattern.as_str(),
                path_contains: path_contains.as_deref(),
                max_hits,
                case_insensitive,
            };

            Ok(match fs::search(&root, opts) {
                Ok(hits) => {
                    let count = hits.len();
                    let truncated = count >= max_hits;
                    if count == 0 {
                        json!({
                            "hits": [],
                            "count": 0,
                            "truncated": false,
                            "note": "0 matches. search_project searches file CONTENTS, not filenames — for filenames use list_files."
                        })
                    } else {
                        json!({ "hits": hits, "count": count, "truncated": truncated })
                    }
                }
                Err(e) => json!({ "error": e }),
            })
        }
    };
    Arc::new(
        FunctionTool::new(
            "search_project",
            "Regex search across project files (gitignore-aware). Returns {path, line, col, snippet} hits. \
             Use for content that find_symbol does not capture: error strings, config keys, literal text, call sites. \
             Hits show a snippet only — read_project_files on the path:line range before quoting.",
            handler,
        )
        .with_parameters_schema::<SearchArgs>(),
    )
}
