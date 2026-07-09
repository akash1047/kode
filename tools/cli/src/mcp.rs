use std::path::Path;
use std::sync::{Arc, Mutex};

use kode_graph::NodeKind as GraphNodeKind;
use kode_query::{QueryEngine, QueryError};
use kode_storage::RepositoryStorage;
use rmcp::model::*;
use rmcp::service::RequestContext;
use rmcp::transport;
use rmcp::{serve_server, ErrorData, RoleServer, ServerHandler};

/// Run MCP server on stdio transport.
pub async fn run_stdio(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let storage = open_repo_storage(path)?;
    let engine = QueryEngine::new(storage);
    let handler = KodeHandler {
        engine: Mutex::new(engine),
    };
    let svc = serve_server(handler, transport::stdio()).await?;
    svc.waiting().await?;
    Ok(())
}

/// Run MCP server on HTTP transport (raw TCP).
pub async fn run_http(path: &str, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await?;
    loop {
        let (stream, _) = listener.accept().await?;
        let handler = KodeHandler {
            engine: Mutex::new(QueryEngine::new(open_repo_storage(path)?)),
        };
        tokio::spawn(async move {
            let _ = serve_server(handler, stream).await;
        });
    }
}

fn open_repo_storage(path: &str) -> Result<RepositoryStorage, Box<dyn std::error::Error>> {
    let path = Path::new(path);
    let absolute = if path.is_relative() {
        std::env::current_dir()?.join(path)
    } else {
        path.to_path_buf()
    };
    let db_path = absolute.join(".kode").join("cache.db");
    if !db_path.exists() {
        return Err("Repository has not been scanned yet. Run `kode scan` first.".into());
    }
    let backend = kode_storage::SqliteBackend::open(&db_path)?;
    let repo_id = absolute.display().to_string();
    Ok(RepositoryStorage::open(Box::new(backend), &repo_id)?)
}

struct KodeHandler {
    engine: Mutex<QueryEngine>,
}

impl ServerHandler for KodeHandler {
    fn get_info(&self) -> ServerInfo {
        let info = Implementation::new("kode-mcp", "0.1.0")
            .with_description("Evidence-first code intelligence server");
        ServerInfo::new(ServerCapabilities::default()).with_server_info(info)
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResult, ErrorData>>
           + rmcp::service::MaybeSendFuture
           + '_ {
        std::future::ready(self.handle_tool(request))
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, ErrorData>>
           + rmcp::service::MaybeSendFuture
           + '_ {
        std::future::ready(Ok(self.make_tool_list()))
    }
}

fn to_json_object(value: serde_json::Value) -> Arc<JsonObject> {
    Arc::new(match value {
        serde_json::Value::Object(map) => map,
        _ => serde_json::Map::new(),
    })
}

impl KodeHandler {
    fn make_tool_list(&self) -> ListToolsResult {
        ListToolsResult {
            tools: vec![
                Tool::new(
                    "find_symbol",
                    "Find a symbol by exact name",
                    to_json_object(serde_json::json!({
                        "type": "object",
                        "properties": {
                            "name": {
                                "type": "string",
                                "description": "Exact symbol name to find"
                            }
                        },
                        "required": ["name"]
                    })),
                ),
                Tool::new(
                    "search_symbols",
                    "Search symbols by name prefix",
                    to_json_object(serde_json::json!({
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "Symbol name prefix to search for"
                            },
                            "kind": {
                                "type": "string",
                                "description": "Optional node kind filter (e.g. function, struct, trait)"
                            }
                        },
                        "required": ["query"]
                    })),
                ),
                Tool::new(
                    "get_symbol_details",
                    "Get full metadata for a symbol by exact name",
                    to_json_object(serde_json::json!({
                        "type": "object",
                        "properties": {
                            "name": {
                                "type": "string",
                                "description": "Exact symbol name"
                            }
                        },
                        "required": ["name"]
                    })),
                ),
                Tool::new(
                    "symbols_by_kind",
                    "List all symbols of a given kind",
                    to_json_object(serde_json::json!({
                        "type": "object",
                        "properties": {
                            "kind": {
                                "type": "string",
                                "description": "Node kind (function, struct, enum, trait, module, file, etc.)"
                            }
                        },
                        "required": ["kind"]
                    })),
                ),
                Tool::new(
                    "read_file",
                    "Read a file from the repository with optional line range",
                    to_json_object(serde_json::json!({
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "File path relative to repository root"
                            },
                            "start_line": {
                                "type": "integer",
                                "description": "Starting line (1-indexed, optional)"
                            },
                            "end_line": {
                                "type": "integer",
                                "description": "Ending line (1-indexed, inclusive, optional)"
                            }
                        },
                        "required": ["path"]
                    })),
                ),
            ],
            next_cursor: None,
            meta: None,
        }
    }

    fn handle_tool(&self, request: CallToolRequestParams) -> Result<CallToolResult, ErrorData> {
        let args = request.arguments.unwrap_or_default();
        match request.name.as_ref() {
            "find_symbol" => self.handle_find_symbol(&args),
            "search_symbols" => self.handle_search_symbols(&args),
            "get_symbol_details" => self.handle_get_symbol_details(&args),
            "symbols_by_kind" => self.handle_symbols_by_kind(&args),
            "read_file" => self.handle_read_file(&args),
            _ => Err(ErrorData::invalid_params("unknown tool", None)),
        }
    }

    fn engine(&self) -> std::sync::MutexGuard<'_, QueryEngine> {
        self.engine.lock().unwrap()
    }

    fn handle_find_symbol(&self, args: &JsonObject) -> Result<CallToolResult, ErrorData> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ErrorData::invalid_params("missing 'name' field", None))?;
        let result = self.engine().find_symbol(name).map_err(to_rmcp_error)?;
        match result {
            Some(sym) => Ok(CallToolResult::success(vec![ContentBlock::text(
                serde_json::to_string_pretty(&format_symbol(&sym)).unwrap_or_else(|_| "{}".into()),
            )])),
            None => Ok(CallToolResult::success(vec![ContentBlock::text("null")])),
        }
    }

    fn handle_search_symbols(&self, args: &JsonObject) -> Result<CallToolResult, ErrorData> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ErrorData::invalid_params("missing 'query' field", None))?;
        if query.is_empty() {
            return Ok(CallToolResult::error(vec![ContentBlock::text(
                "query too broad: empty query matches all symbols",
            )]));
        }
        let kind = args
            .get("kind")
            .and_then(|v| v.as_str())
            .map(|s| s.parse::<GraphNodeKind>())
            .transpose()
            .map_err(|e| ErrorData::invalid_params(format!("invalid kind: {e}"), None))?;
        let results = self
            .engine()
            .search_symbols(query, kind)
            .map_err(to_rmcp_error)?;
        let json = serde_json::to_string_pretty(
            &results
                .into_iter()
                .map(|s| format_symbol(&s))
                .collect::<Vec<_>>(),
        )
        .unwrap_or_else(|_| "[]".into());
        Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
    }

    fn handle_get_symbol_details(&self, args: &JsonObject) -> Result<CallToolResult, ErrorData> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ErrorData::invalid_params("missing 'name' field", None))?;
        let result = self.engine().find_symbol(name).map_err(to_rmcp_error)?;
        match result {
            Some(sym) => {
                let detail = serde_json::json!({
                    "name": sym.name,
                    "kind": sym.kind.as_str(),
                    "file_path": sym.file_path.to_string_lossy(),
                    "start_line": sym.start_line,
                    "start_column": sym.start_column,
                    "end_line": sym.end_line,
                    "end_column": sym.end_column,
                    "verified": sym.verified,
                });
                Ok(CallToolResult::success(vec![ContentBlock::text(
                    serde_json::to_string_pretty(&detail).unwrap_or_else(|_| "{}".into()),
                )]))
            }
            None => Ok(CallToolResult::success(vec![ContentBlock::text("null")])),
        }
    }

    fn handle_symbols_by_kind(&self, args: &JsonObject) -> Result<CallToolResult, ErrorData> {
        let kind_str = args
            .get("kind")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ErrorData::invalid_params("missing 'kind' field", None))?;
        let kind = kind_str.parse::<GraphNodeKind>().map_err(|e| {
            ErrorData::invalid_params(
                format!("unknown node kind: {e}. Valid kinds: repository, workspace, file, module, function, struct, enum, trait, impl_block, type_alias, constant, static, import, export"),
                None,
            )
        })?;
        let results = self.engine().symbols_by_kind(kind).map_err(to_rmcp_error)?;
        let json = serde_json::to_string_pretty(
            &results
                .into_iter()
                .map(|s| format_symbol(&s))
                .collect::<Vec<_>>(),
        )
        .unwrap_or_else(|_| "[]".into());
        Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
    }

    fn handle_read_file(&self, args: &JsonObject) -> Result<CallToolResult, ErrorData> {
        let rel_path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ErrorData::invalid_params("missing 'path' field", None))?;
        let start_line = args
            .get("start_line")
            .and_then(|v| v.as_u64())
            .map(|l| l as usize);
        let end_line = args
            .get("end_line")
            .and_then(|v| v.as_u64())
            .map(|l| l as usize);

        let content = std::fs::read_to_string(rel_path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => {
                ErrorData::invalid_params(format!("file not found: {rel_path}"), None)
            }
            _ => ErrorData::internal_error(format!("failed to read file: {e}"), None),
        })?;

        let lines: Vec<&str> = content.lines().collect();
        let start = start_line.map(|l| l.saturating_sub(1)).unwrap_or(0);
        let end = end_line.map(|l| l.min(lines.len())).unwrap_or(lines.len());

        if start >= lines.len() {
            return Ok(CallToolResult::success(vec![ContentBlock::text("")]));
        }

        let excerpt = lines[start..end].join("\n");
        Ok(CallToolResult::success(vec![ContentBlock::text(excerpt)]))
    }
}

fn to_rmcp_error(e: QueryError) -> ErrorData {
    ErrorData::internal_error(e.to_string(), None)
}

fn format_symbol(sym: &kode_query::SymbolResult) -> serde_json::Value {
    serde_json::json!({
        "name": sym.name,
        "kind": sym.kind.as_str(),
        "file_path": sym.file_path.to_string_lossy(),
        "start_line": sym.start_line,
        "start_column": sym.start_column,
        "end_line": sym.end_line,
        "end_column": sym.end_column,
        "verified": sym.verified,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kode_storage::SqliteBackend;
    use rmcp::model::CallToolRequestParams;

    fn make_handler_with_empty_graph() -> KodeHandler {
        use kode_graph::KnowledgeGraph;
        use kode_storage::RepositoryMetadata;
        use std::time::SystemTime;

        let backend = Box::new(SqliteBackend::in_memory().unwrap());
        let mut storage = RepositoryStorage::open(backend, "test-repo").unwrap();
        let graph = KnowledgeGraph::new(
            Vec::new(),
            Vec::new(),
            std::collections::BTreeMap::new(),
            Vec::new(),
            Vec::new(),
        );
        let metadata = RepositoryMetadata {
            repository_id: "test-repo".into(),
            root: "/test".into(),
            fingerprint: "fp".into(),
            parser_versions: vec![],
            last_updated: SystemTime::now(),
        };
        storage
            .persist(&graph, &metadata, kode_graph::GraphVersion::new(1, 0))
            .unwrap();
        KodeHandler {
            engine: Mutex::new(QueryEngine::new(storage)),
        }
    }

    #[test]
    fn test_list_tools_returns_five_tools() {
        let handler = make_handler_with_empty_graph();
        let result = handler.make_tool_list();
        assert_eq!(result.tools.len(), 5);
        let names: Vec<&str> = result.tools.iter().map(|t| t.name.as_ref()).collect();
        assert!(names.contains(&"find_symbol"));
        assert!(names.contains(&"search_symbols"));
        assert!(names.contains(&"get_symbol_details"));
        assert!(names.contains(&"symbols_by_kind"));
        assert!(names.contains(&"read_file"));
    }

    fn make_request(name: &'static str, args: JsonObject) -> CallToolRequestParams {
        CallToolRequestParams::new(name).with_arguments(args)
    }

    #[test]
    fn test_unknown_tool_returns_error() {
        let handler = make_handler_with_empty_graph();
        let request = make_request("nonexistent_tool", JsonObject::new());
        let result = handler.handle_tool(request);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_symbol_returns_null_when_empty() {
        let handler = make_handler_with_empty_graph();
        let mut args = JsonObject::new();
        args.insert("name".into(), serde_json::Value::String("foo".into()));
        let request = make_request("find_symbol", args);
        let result = handler.handle_tool(request).unwrap();
        let text = result.content.first().unwrap().as_text().unwrap();
        assert_eq!(text.text, "null");
    }

    #[test]
    fn test_find_symbol_missing_name_returns_error() {
        let handler = make_handler_with_empty_graph();
        let request = make_request("find_symbol", JsonObject::new());
        let result = handler.handle_tool(request);
        assert!(result.is_err());
    }

    #[test]
    fn test_search_symbols_empty_query_returns_error() {
        let handler = make_handler_with_empty_graph();
        let mut args = JsonObject::new();
        args.insert("query".into(), serde_json::Value::String("".into()));
        let request = make_request("search_symbols", args);
        let result = handler.handle_tool(request).unwrap();
        let text = result.content.first().unwrap().as_text().unwrap();
        assert!(text.text.contains("query too broad"));
    }

    #[test]
    fn test_search_symbols_invalid_kind_returns_error() {
        let handler = make_handler_with_empty_graph();
        let mut args = JsonObject::new();
        args.insert("query".into(), serde_json::Value::String("foo".into()));
        args.insert("kind".into(), serde_json::Value::String("banana".into()));
        let request = make_request("search_symbols", args);
        let result = handler.handle_tool(request);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_symbol_details_returns_null_when_empty() {
        let handler = make_handler_with_empty_graph();
        let mut args = JsonObject::new();
        args.insert("name".into(), serde_json::Value::String("foo".into()));
        let request = make_request("get_symbol_details", args);
        let result = handler.handle_tool(request).unwrap();
        let text = result.content.first().unwrap().as_text().unwrap();
        assert_eq!(text.text, "null");
    }

    #[test]
    fn test_symbols_by_kind_invalid_kind_returns_error() {
        let handler = make_handler_with_empty_graph();
        let mut args = JsonObject::new();
        args.insert("kind".into(), serde_json::Value::String("banana".into()));
        let request = make_request("symbols_by_kind", args);
        let result = handler.handle_tool(request);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_file_not_found_returns_error() {
        let handler = make_handler_with_empty_graph();
        let mut args = JsonObject::new();
        args.insert(
            "path".into(),
            serde_json::Value::String("/nonexistent/file.rs".into()),
        );
        let request = make_request("read_file", args);
        let result = handler.handle_tool(request);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_file_missing_path_returns_error() {
        let handler = make_handler_with_empty_graph();
        let request = make_request("read_file", JsonObject::new());
        let result = handler.handle_tool(request);
        assert!(result.is_err());
    }
}
