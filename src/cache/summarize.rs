use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

use crate::config::{DefaultSection, SummarizeSection};

const DEFAULT_BASE_URL: &str = "https://ollama.com/v1";
const DEFAULT_MODEL: &str = "nemotron-3-nano:30b";
const API_KEY_ENV: &str = "KODE_MODEL_API_KEY";
const MODEL_ENV: &str = "KODE_SUMMARIZE_MODEL";
const BASE_URL_ENV: &str = "KODE_SUMMARIZE_BASE_URL";
const DEFAULT_MAX_INPUT_CHARS: usize = 8000;

const FILE_SYSTEM_PROMPT: &str = "You summarize source files for a code index. \
    Output EXACTLY three short lines, no markdown, no preamble:\n\
    purpose: <one sentence — what this file does>\n\
    exports: <comma-separated public names, or `-` if internal>\n\
    notes: <one sentence on side effects, dependencies, or quirks; `-` if none>";

const DIR_SYSTEM_PROMPT: &str = "You summarize directories for a code index based on per-file summaries below. \
    Output EXACTLY two short lines, no markdown:\n\
    role: <one sentence — what this directory is for in the project>\n\
    contents: <one sentence — main subcomponents or themes>";

pub struct SummarizeConfig {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
    pub max_input_chars: usize,
}

impl SummarizeConfig {
    /// Resolution order: env var > [summarize] > [default] > built-in constant.
    pub fn load(section: Option<&SummarizeSection>, defaults: Option<&DefaultSection>) -> Result<Self> {
        let api_key = std::env::var(API_KEY_ENV)
            .ok()
            .or_else(|| section.and_then(|s| s.api_key.clone()))
            .or_else(|| defaults.and_then(|d| d.api_key.clone()))
            .with_context(|| format!("env {API_KEY_ENV} not set and not in config [summarize] or [default]"))?;
        let model = std::env::var(MODEL_ENV)
            .ok()
            .or_else(|| section.and_then(|s| s.model.clone()))
            .or_else(|| defaults.and_then(|d| d.model.clone()))
            .unwrap_or_else(|| DEFAULT_MODEL.to_string());
        let base_url = std::env::var(BASE_URL_ENV)
            .ok()
            .or_else(|| section.and_then(|s| s.base_url.clone()))
            .or_else(|| defaults.and_then(|d| d.base_url.clone()))
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        let max_input_chars = section
            .and_then(|s| s.max_input_chars)
            .unwrap_or(DEFAULT_MAX_INPUT_CHARS);
        Ok(Self {
            api_key,
            model,
            base_url,
            max_input_chars,
        })
    }
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<Msg<'a>>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Serialize)]
struct Msg<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Deserialize)]
struct ChoiceMessage {
    content: String,
    #[serde(default)]
    reasoning: Option<String>,
}

pub async fn summarize_file(
    cfg: &SummarizeConfig,
    rel_path: &str,
    lang: Option<&str>,
    content: &str,
) -> Result<String> {
    let lang = lang.unwrap_or("text");
    let trimmed = truncate(content, cfg.max_input_chars);
    let user = format!("path: {rel_path}\nlanguage: {lang}\n\n---\n{trimmed}\n---");
    call(cfg, FILE_SYSTEM_PROMPT, &user, 1200).await
}

pub async fn summarize_dir(
    cfg: &SummarizeConfig,
    dir_path: &str,
    child_summaries: &[(String, String)],
) -> Result<String> {
    let mut body = String::new();
    body.push_str("directory: ");
    body.push_str(dir_path);
    body.push_str("\n\nchild summaries:\n");
    let mut chars = body.len();
    for (p, s) in child_summaries {
        let block = format!("- {p}\n{s}\n");
        if chars + block.len() > cfg.max_input_chars {
            body.push_str("- … (truncated)\n");
            break;
        }
        chars += block.len();
        body.push_str(&block);
    }
    call(cfg, DIR_SYSTEM_PROMPT, &body, 800).await
}

async fn call(cfg: &SummarizeConfig, system: &str, user: &str, max_tokens: u32) -> Result<String> {
    let url = format!("{}/chat/completions", cfg.base_url.trim_end_matches('/'));
    let body = ChatRequest {
        model: &cfg.model,
        messages: vec![
            Msg {
                role: "system",
                content: system,
            },
            Msg {
                role: "user",
                content: user,
            },
        ],
        temperature: 0.0,
        max_tokens,
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;
    let resp = client
        .post(&url)
        .bearer_auth(&cfg.api_key)
        .json(&body)
        .send()
        .await?;
    let status = resp.status();
    if !status.is_success() {
        let txt = resp.text().await.unwrap_or_default();
        return Err(anyhow!("LLM call failed: {status}: {}", truncate(&txt, 300)));
    }
    let raw = resp.text().await?;
    let parsed: ChatResponse = serde_json::from_str(&raw)
        .with_context(|| format!("LLM response not JSON-decodable. Raw: {}", truncate(&raw, 400)))?;
    let msg = parsed
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("LLM returned no choices. Raw: {}", truncate(&raw, 400)))?
        .message;
    let content = msg.content.trim().to_string();
    if !content.is_empty() {
        return Ok(content);
    }
    if let Some(r) = msg.reasoning {
        let r = r.trim();
        if !r.is_empty() {
            return Ok(r.to_string());
        }
    }
    Err(anyhow!(
        "LLM returned empty content (likely hit max_tokens during reasoning). Raw: {}",
        truncate(&raw, 400)
    ))
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let head: String = s.chars().take(max).collect();
    format!("{head}\n… (truncated)")
}
