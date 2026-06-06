use std::path::Path;

use crate::cli::McpTransport;
use crate::mcp;

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
