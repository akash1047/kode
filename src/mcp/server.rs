use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use serde_json::{Value, json};

use crate::cache::{Cache, revalidate};
use crate::chat;
use crate::config::KodeConfig;

use super::agent;
use super::evidence::{self, EvidenceCollector};
use super::protocol::{self, PROTOCOL_VERSION, Request, RpcError, err, ok};
use super::tools;

pub struct McpState {
    pub api_key: String,
    pub model_name: String,
    pub root: PathBuf,
    pub cache: Arc<Mutex<Cache>>,
    counter: AtomicUsize,
}

impl McpState {
    pub fn next_session_counter(&self) -> usize {
        self.counter.fetch_add(1, Ordering::SeqCst)
    }
}

pub fn boot(root: &Path) -> Result<Arc<McpState>> {
    let cfg = KodeConfig::load();
    let chat_cfg = cfg.chat.as_ref();
    let def = cfg.default.as_ref();
    let api_key = std::env::var(chat::API_KEY_ENV)
        .ok()
        .or_else(|| chat_cfg.and_then(|c| c.api_key.clone()))
        .or_else(|| def.and_then(|d| d.api_key.clone()))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "{} not set and not in .kode/config.toml [chat] or [default]",
                chat::API_KEY_ENV
            )
        })?;
    let model_name = std::env::var(chat::MODEL_ENV)
        .ok()
        .or_else(|| chat_cfg.and_then(|c| c.model.clone()))
        .or_else(|| def.and_then(|d| d.model.clone()))
        .unwrap_or_else(|| chat::DEFAULT_MODEL.to_string());

    let mut cache = Cache::open(root)?;
    let report = revalidate::refresh(&mut cache)?;
    eprintln!(
        "kode mcp: model={} indexed {} files ({} symbols across {} files) at {}",
        model_name,
        report.scanned,
        report.symbols_total,
        report.symbols_files,
        root.display()
    );

    Ok(Arc::new(McpState {
        api_key,
        model_name,
        root: root.to_path_buf(),
        cache: Arc::new(Mutex::new(cache)),
        counter: AtomicUsize::new(1),
    }))
}

pub async fn handle(state: &Arc<McpState>, req: Request) -> Option<protocol::Response> {
    let is_notification = req.id.is_none();
    let id = req.id.clone().unwrap_or(Value::Null);

    let result: Result<Value, (i32, String)> = match req.method.as_str() {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": protocol::server_info(),
        })),
        "notifications/initialized" | "notifications/cancelled" => return None,
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools::list() })),
        "tools/call" => call_tool(state, &req.params).await,
        "resources/list" => Ok(json!({ "resources": [] })),
        "prompts/list" => Ok(json!({ "prompts": [] })),
        other => Err((
            RpcError::METHOD_NOT_FOUND,
            format!("method not found: {other}"),
        )),
    };

    if is_notification {
        return None;
    }

    match result {
        Ok(value) => Some(ok(id, value)),
        Err((code, msg)) => Some(err(id, code, msg)),
    }
}

async fn call_tool(
    state: &Arc<McpState>,
    params: &Value,
) -> Result<Value, (i32, String)> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or((RpcError::INVALID_PARAMS, "missing 'name'".to_string()))?;
    let empty = json!({});
    let args = params.get("arguments").unwrap_or(&empty);

    match name {
        "ask" => {
            let question = args
                .get("question")
                .and_then(Value::as_str)
                .ok_or((
                    RpcError::INVALID_PARAMS,
                    "missing 'question'".to_string(),
                ))?
                .trim();
            if question.is_empty() {
                return Ok(tools::wrap_text("question is empty".to_string(), true));
            }
            match ask(state, question).await {
                Ok(answer) => Ok(tools::wrap_text(answer, false)),
                Err(e) => Ok(tools::wrap_text(format!("agent error: {e:#}"), true)),
            }
        }
        other => Err((
            RpcError::INVALID_PARAMS,
            format!("unknown tool: {other}"),
        )),
    }
}

async fn ask(state: &Arc<McpState>, question: &str) -> Result<String> {
    let counter = state.next_session_counter();
    let collector = EvidenceCollector::new();
    let session_chars = Arc::new(Mutex::new(0usize));
    let (runner, session_id) = agent::build_mcp_runner(
        &state.api_key,
        &state.model_name,
        &state.root,
        counter,
        state.cache.clone(),
        collector.clone(),
        session_chars,
    )
    .await?;
    eprintln!(
        "kode mcp: ask session={session_id} q={}",
        truncate(question, 120)
    );
    let answer = chat::one_shot(&runner, &session_id, question).await?;
    let footer = {
        let guard = collector.lock().expect("collector mutex poisoned");
        evidence::format_sources_footer(&guard)
    };
    Ok(format!("{}{}", answer.trim_end(), footer))
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let head: String = s.chars().take(max).collect();
    format!("{head}…")
}
