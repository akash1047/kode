use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
};
use serde_json::Value;

use super::protocol::{Request, RpcError, err};
use super::server::{McpState, handle};

pub async fn serve(state: Arc<McpState>, port: u16) -> Result<()> {
    let app = Router::new()
        .route("/mcp", post(rpc))
        .with_state(state);

    let addr: SocketAddr = ([127, 0, 0, 1], port).into();
    let listener = tokio::net::TcpListener::bind(addr).await?;
    eprintln!("kode mcp: listening http://{addr}/mcp");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn rpc(
    State(state): State<Arc<McpState>>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let req: Request = match serde_json::from_value(payload) {
        Ok(r) => r,
        Err(e) => {
            let resp = err(
                Value::Null,
                RpcError::PARSE_ERROR,
                format!("parse error: {e}"),
            );
            return (StatusCode::OK, Json(serde_json::to_value(resp).unwrap()));
        }
    };

    match handle(&state, req).await {
        Some(resp) => (StatusCode::OK, Json(serde_json::to_value(resp).unwrap())),
        None => (StatusCode::ACCEPTED, Json(Value::Null)),
    }
}
