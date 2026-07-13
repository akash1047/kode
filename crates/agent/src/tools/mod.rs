//! Sandboxed repository tools for the agent (list / grep / read + query).

mod grep;
mod list_dir;
mod path;
mod read_file;

use std::path::Path;

use serde_json::{json, Value};

use crate::index::SymbolIndex;
use crate::provider::{ToolDefinition, ToolFunction};

/// JSON tool schemas offered to the model.
pub fn tool_definitions(has_query: bool) -> Vec<ToolDefinition> {
    let mut tools = vec![
        ToolDefinition {
            kind: "function".into(),
            function: ToolFunction {
                name: "list_dir".into(),
                description: "List files and directories under the project root. Path is relative to project root (default \".\").".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Directory path relative to project root (default \".\")"
                        },
                        "max_entries": {
                            "type": "integer",
                            "description": "Maximum entries to return (default 200)"
                        }
                    }
                }),
            },
        },
        ToolDefinition {
            kind: "function".into(),
            function: ToolFunction {
                name: "grep".into(),
                description: "Search file contents under the project root with a regular expression. Returns file:line:match.".into(),
                parameters: json!({
                    "type": "object",
                    "required": ["pattern"],
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Regular expression to search for"
                        },
                        "path": {
                            "type": "string",
                            "description": "File or directory relative to project root (default \".\")"
                        },
                        "glob": {
                            "type": "string",
                            "description": "Optional filename filter, e.g. \"*.rs\""
                        },
                        "case_insensitive": {
                            "type": "boolean",
                            "description": "Case-insensitive search (default false)"
                        },
                        "max_matches": {
                            "type": "integer",
                            "description": "Max matches to return (default 50)"
                        }
                    }
                }),
            },
        },
        ToolDefinition {
            kind: "function".into(),
            function: ToolFunction {
                name: "read_file".into(),
                description: "Read a text file under the project root with optional line offset/limit. Returns numbered lines.".into(),
                parameters: json!({
                    "type": "object",
                    "required": ["path"],
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "File path relative to project root"
                        },
                        "offset": {
                            "type": "integer",
                            "description": "1-based start line (default 1)"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Max lines to return (default 200)"
                        }
                    }
                }),
            },
        },
    ];

    if has_query {
        tools.push(ToolDefinition {
            kind: "function".into(),
            function: ToolFunction {
                name: "search_symbols".into(),
                description: "Search the knowledge-graph index for code symbols by name or prefix. Returns matching symbols with file paths and line numbers.".into(),
                parameters: json!({
                    "type": "object",
                    "required": ["query"],
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Symbol name or prefix to search for"
                        }
                    }
                }),
            },
        });
        tools.push(ToolDefinition {
            kind: "function".into(),
            function: ToolFunction {
                name: "find_symbol".into(),
                description:
                    "Find the first symbol with an exact name match in the knowledge graph.".into(),
                parameters: json!({
                    "type": "object",
                    "required": ["name"],
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Exact symbol name"
                        }
                    }
                }),
            },
        });
        tools.push(ToolDefinition {
            kind: "function".into(),
            function: ToolFunction {
                name: "find_callers".into(),
                description:
                    "Find functions that call the named function (incoming call-graph edges)."
                        .into(),
                parameters: json!({
                    "type": "object",
                    "required": ["name"],
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Exact function name whose callers to list"
                        }
                    }
                }),
            },
        });
        tools.push(ToolDefinition {
            kind: "function".into(),
            function: ToolFunction {
                name: "find_callees".into(),
                description:
                    "Find functions called by the named function (outgoing call-graph edges)."
                        .into(),
                parameters: json!({
                    "type": "object",
                    "required": ["name"],
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Exact function name whose callees to list"
                        }
                    }
                }),
            },
        });
        tools.push(ToolDefinition {
            kind: "function".into(),
            function: ToolFunction {
                name: "impact_analysis".into(),
                description:
                    "List functions that transitively call the named function (change impact)."
                        .into(),
                parameters: json!({
                    "type": "object",
                    "required": ["name"],
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Exact function name to analyze"
                        },
                        "max_depth": {
                            "type": "integer",
                            "description": "Max reverse-call depth (default 8)"
                        }
                    }
                }),
            },
        });
        tools.push(ToolDefinition {
            kind: "function".into(),
            function: ToolFunction {
                name: "function_metrics".into(),
                description: "Fan-in / fan-out metrics for functions (optional name filter)."
                    .into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Optional exact function name"
                        }
                    }
                }),
            },
        });
        tools.push(ToolDefinition {
            kind: "function".into(),
            function: ToolFunction {
                name: "dead_code".into(),
                description:
                    "List functions with no recorded callers (heuristic unreferenced code).".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        });
    }

    tools
}

/// Execute a tool by name. Always returns a string for the model.
pub fn execute(root: &Path, name: &str, args: &Value, index: Option<&SymbolIndex>) -> String {
    let result = execute_inner(root, name, args, index);
    match result {
        Ok(s) => truncate_result(s, 64_000),
        Err(e) => format!("error: {e}"),
    }
}

fn execute_inner(
    root: &Path,
    name: &str,
    args: &Value,
    index: Option<&SymbolIndex>,
) -> Result<String, String> {
    match name {
        "list_dir" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
            let max = args
                .get("max_entries")
                .and_then(|v| v.as_u64())
                .unwrap_or(200) as usize;
            list_dir::list_dir(root, path, max.clamp(1, 1000))
        }
        "grep" => {
            let pattern = args
                .get("pattern")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "missing required argument: pattern".to_string())?;
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
            let glob = args.get("glob").and_then(|v| v.as_str());
            let case_insensitive = args
                .get("case_insensitive")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let max_matches = args
                .get("max_matches")
                .and_then(|v| v.as_u64())
                .unwrap_or(50) as usize;
            grep::grep(
                root,
                pattern,
                path,
                glob,
                case_insensitive,
                max_matches.clamp(1, 500),
            )
        }
        "read_file" => {
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "missing required argument: path".to_string())?;
            let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(200) as usize;
            read_file::read_file(root, path, offset, limit)
        }
        "search_symbols" => {
            let index = index.ok_or_else(|| {
                "No knowledge graph index available. Run `kode scan` first.".to_string()
            })?;
            let q = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            let results = index.search(q);
            if results.is_empty() {
                Ok("No matching symbols found.".into())
            } else {
                let mut lines = vec![format!("Found {} symbols:", results.len())];
                for s in results.iter().take(50) {
                    lines.push(format!(
                        "  - {} ({}) at {}:{}",
                        s.name,
                        s.kind,
                        s.file_path.display(),
                        s.start_line
                    ));
                }
                if results.len() > 50 {
                    lines.push(format!("  … and {} more", results.len() - 50));
                }
                Ok(lines.join("\n"))
            }
        }
        "find_symbol" => {
            let index = index.ok_or_else(|| {
                "No knowledge graph index available. Run `kode scan` first.".to_string()
            })?;
            let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
            match index.find_exact(name) {
                Some(s) => Ok(format!(
                    "{} ({}) at {}:{}-{} verified={}",
                    s.name,
                    s.kind,
                    s.file_path.display(),
                    s.start_line,
                    s.end_line,
                    s.verified
                )),
                None => Ok(format!("Symbol `{name}` not found.")),
            }
        }
        "find_callers" | "find_callees" | "impact_analysis" => {
            let index = index.ok_or_else(|| {
                "No knowledge graph index available. Run `kode scan` first.".to_string()
            })?;
            let symbol_name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
            if symbol_name.is_empty() {
                return Err("missing required argument: name".into());
            }
            let max_depth = args
                .get("max_depth")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize);
            let results = match name {
                "find_callers" => index.find_callers(symbol_name),
                "find_callees" => index.find_callees(symbol_name),
                "impact_analysis" => index.impact(symbol_name, max_depth.or(Some(8))),
                _ => unreachable!(),
            };
            if results.is_empty() {
                Ok(format!("No results for `{symbol_name}` ({name})."))
            } else {
                let mut lines = vec![format!(
                    "{name} of `{symbol_name}`: {} hit(s)",
                    results.len()
                )];
                for s in results.iter().take(50) {
                    lines.push(format!(
                        "  - {} ({}) at {}:{}",
                        s.name,
                        s.kind,
                        s.file_path.display(),
                        s.start_line
                    ));
                }
                Ok(lines.join("\n"))
            }
        }
        "function_metrics" => {
            let index = index.ok_or_else(|| {
                "No knowledge graph index available. Run `kode scan` first.".to_string()
            })?;
            let name_filter = args.get("name").and_then(|v| v.as_str());
            let rows = index.metrics(name_filter);
            if rows.is_empty() {
                Ok("No function metrics.".into())
            } else {
                let mut lines = vec![format!("function metrics: {} row(s)", rows.len())];
                for (name, fan_in, fan_out, path, line) in rows.iter().take(80) {
                    lines.push(format!(
                        "  - {name} fan_in={fan_in} fan_out={fan_out} at {}:{}",
                        path.display(),
                        line
                    ));
                }
                Ok(lines.join("\n"))
            }
        }
        "dead_code" => {
            let index = index.ok_or_else(|| {
                "No knowledge graph index available. Run `kode scan` first.".to_string()
            })?;
            let results = index.dead_code();
            if results.is_empty() {
                Ok("No unreferenced functions found.".into())
            } else {
                let mut lines = vec![format!("dead/unreferenced candidates: {}", results.len())];
                for s in results.iter().take(80) {
                    lines.push(format!(
                        "  - {} at {}:{}",
                        s.name,
                        s.file_path.display(),
                        s.start_line
                    ));
                }
                Ok(lines.join("\n"))
            }
        }
        other => Err(format!("unknown tool: {other}")),
    }
}

fn truncate_result(s: String, max: usize) -> String {
    if s.len() <= max {
        return s;
    }
    let mut end = max.saturating_sub(20);
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…\n[truncated: {} bytes total]", &s[..end], s.len())
}

/// Short one-line preview for UI status.
pub fn preview(result: &str, max_chars: usize) -> String {
    let line = result.lines().next().unwrap_or(result);
    if line.chars().count() <= max_chars {
        return line.to_string();
    }
    line.chars()
        .take(max_chars.saturating_sub(1))
        .collect::<String>()
        + "…"
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn dispatch_list_and_read() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("hi.txt"), "hello\nworld\n").unwrap();
        let listed = execute(dir.path(), "list_dir", &json!({"path": "."}), None);
        assert!(listed.contains("hi.txt"));
        let read = execute(
            dir.path(),
            "read_file",
            &json!({"path": "hi.txt", "limit": 5}),
            None,
        );
        assert!(read.contains("hello"));
    }

    #[test]
    fn unknown_tool_errors() {
        let dir = tempdir().unwrap();
        let out = execute(dir.path(), "nope", &json!({}), None);
        assert!(out.contains("unknown tool"));
    }

    #[test]
    fn tool_definitions_without_query() {
        let defs = tool_definitions(false);
        assert!(defs.iter().any(|t| t.function.name == "list_dir"));
        assert!(!defs.iter().any(|t| t.function.name == "search_symbols"));
    }

    #[test]
    fn tool_definitions_with_query() {
        let defs = tool_definitions(true);
        assert!(defs.iter().any(|t| t.function.name == "search_symbols"));
        assert!(defs.iter().any(|t| t.function.name == "find_symbol"));
        assert!(defs.iter().any(|t| t.function.name == "find_callers"));
        assert!(defs.iter().any(|t| t.function.name == "find_callees"));
        assert!(defs.iter().any(|t| t.function.name == "impact_analysis"));
        assert!(defs.iter().any(|t| t.function.name == "function_metrics"));
        assert!(defs.iter().any(|t| t.function.name == "dead_code"));
    }
}
