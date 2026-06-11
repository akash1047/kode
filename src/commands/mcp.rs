use std::path::Path;

use crate::cli::{McpPreset, McpTransport};
use crate::mcp;

pub fn init(root: &Path, preset: McpPreset) {
    super::ensure_dir(root);

    let abs_root = std::fs::canonicalize(root)
        .unwrap_or_else(|_| root.to_path_buf());
    let serve_path = abs_root.to_string_lossy().into_owned();

    let (rel_path, content) = match preset {
        McpPreset::Claude => (
            ".mcp.json".to_string(),
            serde_json::json!({
                "mcpServers": {
                    "kode": {
                        "command": "kode",
                        "args": ["mcp", "serve", serve_path]
                    }
                }
            }),
        ),
        McpPreset::Vscode => (
            ".vscode/mcp.json".to_string(),
            serde_json::json!({
                "servers": {
                    "kode": {
                        "type": "stdio",
                        "command": "kode",
                        "args": ["mcp", "serve", serve_path]
                    }
                }
            }),
        ),
        McpPreset::Cursor => (
            ".cursor/mcp.json".to_string(),
            serde_json::json!({
                "mcpServers": {
                    "kode": {
                        "command": "kode",
                        "args": ["mcp", "serve", serve_path]
                    }
                }
            }),
        ),
        McpPreset::Zed => (
            ".zed/settings.json".to_string(),
            serde_json::json!({
                "context_servers": {
                    "kode": {
                        "command": "kode",
                        "args": ["mcp", "serve", serve_path]
                    }
                }
            }),
        ),
    };

    let dest = root.join(&rel_path);
    if dest.exists() {
        eprintln!("{} already exists: {}", rel_path, dest.display());
        eprintln!("edit it directly, or delete it and re-run init");
        std::process::exit(1);
    }
    if let Some(parent) = dest.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!("failed to create {}: {e}", parent.display());
            std::process::exit(1);
        }
    }
    let text = serde_json::to_string_pretty(&content).expect("json serialize") + "\n";
    if let Err(e) = std::fs::write(&dest, text) {
        eprintln!("failed to write {rel_path}: {e}");
        std::process::exit(1);
    }
    println!("created: {}", dest.display());
}

pub async fn serve(root: &Path, transport: McpTransport, port: u16) {
    super::ensure_dir(root);
    let state = match mcp::boot(root) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("kode mcp: fatal: {e:#}");
            std::process::exit(1);
        }
    };

    let result = match transport {
        McpTransport::Stdio => mcp::stdio::serve(state).await,
        McpTransport::Http => mcp::http::serve(state, port).await,
    };

    if let Err(e) = result {
        eprintln!("kode mcp: fatal: {e:#}");
        std::process::exit(1);
    }
}
