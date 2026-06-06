use serde_json::{Value, json};

/// Returns the MCP `tools/list` payload describing every tool this server exposes.
pub fn list() -> Value {
    json!([
        {
            "name": "ask",
            "description": "Ask a literal question about the project. An internal agent uses list_files, find_symbol, and read_file to locate and read the relevant source, then returns a grounded answer with a Sources footer (path:line citations for each load-bearing claim). Stateless — each call starts fresh. For ambiguous answers, refine the question and re-ask.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "question": {
                        "type": "string",
                        "description": "Natural-language question about the codebase."
                    }
                },
                "required": ["question"],
                "additionalProperties": false
            }
        }
    ])
}

/// Wrap a text string into an MCP `CallToolResult` JSON envelope.
pub fn wrap_text(text: String, is_error: bool) -> Value {
    json!({
        "content": [
            { "type": "text", "text": text }
        ],
        "isError": is_error,
    })
}
