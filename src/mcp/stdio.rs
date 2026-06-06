use std::sync::Arc;

use anyhow::Result;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use super::protocol::{Request, RpcError, err};
use super::server::{McpState, handle};

pub async fn serve(state: Arc<McpState>) -> Result<()> {
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut stdout = tokio::io::stdout();
    let mut line = String::new();

    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<Request>(trimmed) {
            Ok(req) => handle(&state, req).await,
            Err(e) => Some(err(
                Value::Null,
                RpcError::PARSE_ERROR,
                format!("parse error: {e}"),
            )),
        };

        if let Some(resp) = response {
            let payload = serde_json::to_string(&resp)?;
            stdout.write_all(payload.as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }
    }

    Ok(())
}
