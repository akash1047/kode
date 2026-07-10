use futures_util::StreamExt;

use crate::config::AgentConfig;
use crate::error::AgentError;
use crate::provider::{
    CompletionOutput, CompletionRequest, Provider, StreamEvent, TokenUsage, ToolCall,
};

/// Provider for all OpenAI-compatible API endpoints.
///
/// Works with: OpenAI, DeepSeek, Groq, OpenRouter, Ollama,
/// and any custom endpoint via `api_base`.
pub struct OpenAiCompatProvider {
    base_url: String,
    api_key: Option<String>,
    model: String,
    client: reqwest::Client,
}

impl OpenAiCompatProvider {
    /// Build a provider from `AgentConfig`.
    ///
    /// URL resolution order:
    /// 1. `config.api_base` (explicit override)
    /// 2. `ProviderKind::known_base_url()`
    /// 3. Ollama default (`http://localhost:11434`) with `/v1/chat/completions` appended
    pub fn new(config: &AgentConfig) -> Self {
        let mut base_url = config
            .api_base
            .clone()
            .or_else(|| config.provider.known_base_url().map(String::from))
            .unwrap_or_else(|| {
                let base = std::env::var("OLLAMA_BASE_URL")
                    .unwrap_or_else(|_| "http://localhost:11434".into());
                format!("{base}/v1/chat/completions")
            });

        if !base_url.ends_with("/chat/completions") {
            if base_url.ends_with('/') {
                base_url.push_str("chat/completions");
            } else {
                base_url.push_str("/chat/completions");
            }
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .expect("valid reqwest client");

        Self {
            base_url,
            api_key: config.api_key.clone(),
            model: config.model.clone(),
            client,
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    fn build_request_body(&self, req: &CompletionRequest, stream: bool) -> serde_json::Value {
        let messages: Vec<serde_json::Value> = req
            .messages
            .iter()
            .map(|m| {
                let mut msg = serde_json::json!({
                    "role": m.role,
                    "content": m.content,
                });
                if let Some(ref calls) = m.tool_calls {
                    let formatted: Vec<serde_json::Value> = calls
                        .iter()
                        .map(|tc| {
                            serde_json::json!({
                                "id": tc.id,
                                "type": "function",
                                "function": {
                                    "name": tc.name,
                                    "arguments": tc.arguments.to_string(),
                                }
                            })
                        })
                        .collect();
                    msg["tool_calls"] = serde_json::json!(formatted);
                }
                if let Some(ref id) = m.tool_call_id {
                    msg["tool_call_id"] = serde_json::json!(id);
                }
                msg
            })
            .collect();

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "temperature": req.temperature,
        });

        if !req.tools.is_empty() {
            body["tools"] = serde_json::json!(req.tools);
        }
        if let Some(max_tokens) = req.max_tokens {
            body["max_tokens"] = serde_json::json!(max_tokens);
        }
        if stream {
            body["stream"] = serde_json::json!(true);
        }
        body
    }

    fn parse_tool_calls(value: &serde_json::Value) -> Vec<ToolCall> {
        value["tool_calls"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|tc| {
                        Some(ToolCall {
                            id: tc["id"].as_str()?.to_string(),
                            name: tc["function"]["name"].as_str()?.to_string(),
                            arguments: tc["function"]["arguments"]
                                .as_str()
                                .and_then(|s| serde_json::from_str(s).ok())
                                .unwrap_or(serde_json::Value::Null),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn parse_usage(value: &serde_json::Value) -> Option<TokenUsage> {
        value["usage"].as_object().map(|u| TokenUsage {
            prompt: u["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            completion: u["completion_tokens"].as_u64().unwrap_or(0) as u32,
            total: u["total_tokens"].as_u64().unwrap_or(0) as u32,
        })
    }
}

#[async_trait::async_trait]
impl Provider for OpenAiCompatProvider {
    async fn complete(&self, req: CompletionRequest) -> Result<CompletionOutput, AgentError> {
        let body = self.build_request_body(&req, req.stream);

        let mut http_req = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .json(&body);

        if let Some(ref key) = self.api_key {
            http_req = http_req.header("Authorization", format!("Bearer {key}"));
        }

        let resp = http_req
            .send()
            .await
            .map_err(|e| AgentError::Provider(format!("LLM request failed: {e}")))?;

        let status = resp.status();

        if req.stream {
            return Self::handle_streaming_response(resp, status).await;
        }

        let text = resp
            .text()
            .await
            .map_err(|e| AgentError::Provider(format!("Failed to read response: {e}")))?;

        if !status.is_success() {
            return Err(AgentError::Provider(format!(
                "LLM API error ({status}): {text}"
            )));
        }

        let json: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| AgentError::Provider(format!("Failed to parse response: {e}")))?;

        let choice = &json["choices"][0];
        let message = &choice["message"];
        let content = message["content"].as_str().unwrap_or_default().to_string();
        let tool_calls = Self::parse_tool_calls(message);
        let usage = Self::parse_usage(&json);

        Ok(CompletionOutput::Full {
            text: content,
            tool_calls,
            usage,
        })
    }

    fn name(&self) -> &str {
        "openai-compatible"
    }
}

impl OpenAiCompatProvider {
    async fn handle_streaming_response(
        resp: reqwest::Response,
        status: reqwest::StatusCode,
    ) -> Result<CompletionOutput, AgentError> {
        let (tx, rx) = tokio::sync::mpsc::channel::<StreamEvent>(64);

        tokio::spawn(async move {
            if !status.is_success() {
                let text = resp.text().await.unwrap_or_default();
                let _ = tx
                    .send(StreamEvent::Done {
                        text: format!("LLM API error ({status}): {text}"),
                        tool_calls: vec![],
                    })
                    .await;
                return;
            }

            let mut full_text = String::new();
            let mut tool_calls: Vec<ToolCall> = Vec::new();
            let mut stream = resp.bytes_stream();

            while let Some(chunk) = stream.next().await {
                let chunk = match chunk {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = tx
                            .send(StreamEvent::Done {
                                text: format!("Stream error: {e}"),
                                tool_calls: vec![],
                            })
                            .await;
                        return;
                    }
                };

                let text = String::from_utf8_lossy(&chunk);
                for line in text.lines() {
                    let line = line.trim();
                    if line.is_empty() || line == "data: [DONE]" {
                        continue;
                    }
                    if let Some(data) = line.strip_prefix("data: ") {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                            if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                                full_text.push_str(content);
                                let _ = tx.send(StreamEvent::Token(content.to_string())).await;
                            }

                            let delta = &json["choices"][0]["delta"];
                            let parsed = Self::parse_tool_calls(delta);
                            if !parsed.is_empty() {
                                for tc in &parsed {
                                    let _ = tx.send(StreamEvent::ToolCall(tc.clone())).await;
                                }
                                tool_calls.extend(parsed);
                            }
                        }
                    }
                }
            }

            let _ = tx
                .send(StreamEvent::Done {
                    text: full_text,
                    tool_calls,
                })
                .await;
        });

        Ok(CompletionOutput::Streamed(rx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> AgentConfig {
        AgentConfig {
            provider: crate::config::ProviderKind::OpenAI,
            model: "gpt-4o".into(),
            api_key: Some("sk-test".into()),
            ..AgentConfig::default()
        }
    }

    #[test]
    fn test_new_uses_known_url() {
        let config = test_config();
        let provider = OpenAiCompatProvider::new(&config);
        assert_eq!(
            provider.base_url,
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(provider.model, "gpt-4o");
        assert_eq!(provider.api_key, Some("sk-test".into()));
    }

    #[test]
    fn test_new_uses_api_base_override() {
        let config = AgentConfig {
            api_base: Some("https://custom.example.com/v1".into()),
            ..test_config()
        };
        let provider = OpenAiCompatProvider::new(&config);
        assert_eq!(
            provider.base_url,
            "https://custom.example.com/v1/chat/completions"
        );
    }

    #[test]
    fn test_new_ollama_default() {
        let config = AgentConfig {
            provider: crate::config::ProviderKind::Ollama,
            api_key: None,
            ..AgentConfig::default()
        };
        let provider = OpenAiCompatProvider::new(&config);
        assert_eq!(
            provider.base_url,
            "http://localhost:11434/v1/chat/completions"
        );
        assert!(provider.api_key.is_none());
    }

    #[test]
    fn test_new_ollama_respects_env() {
        let prev = std::env::var("OLLAMA_BASE_URL").ok();
        std::env::set_var("OLLAMA_BASE_URL", "http://ollama.local:8080");

        let config = AgentConfig {
            provider: crate::config::ProviderKind::Ollama,
            api_key: None,
            ..AgentConfig::default()
        };
        let provider = OpenAiCompatProvider::new(&config);
        assert_eq!(
            provider.base_url,
            "http://ollama.local:8080/v1/chat/completions"
        );

        // Restore
        match prev {
            Some(v) => std::env::set_var("OLLAMA_BASE_URL", v),
            None => std::env::remove_var("OLLAMA_BASE_URL"),
        }
    }

    #[test]
    fn test_build_request_body_no_tools() {
        let req = CompletionRequest {
            messages: vec![],
            tools: vec![],
            temperature: 0.5,
            max_tokens: Some(100),
            stream: false,
        };
        let config = test_config();
        let provider = OpenAiCompatProvider::new(&config);
        let body = provider.build_request_body(&req, false);
        assert_eq!(body["model"], "gpt-4o");
        assert_eq!(body["temperature"], 0.5);
        assert_eq!(body["max_tokens"], 100);
        assert!(body.get("tools").is_none());
        assert!(body.get("stream").is_none());
    }

    #[test]
    fn test_build_request_body_with_tools() {
        let req = CompletionRequest {
            messages: vec![],
            tools: vec![crate::provider::ToolDefinition {
                kind: "function".into(),
                function: crate::provider::ToolFunction {
                    name: "test_tool".into(),
                    description: "A test".into(),
                    parameters: serde_json::json!({}),
                },
            }],
            temperature: 0.3,
            max_tokens: None,
            stream: false,
        };
        let config = test_config();
        let provider = OpenAiCompatProvider::new(&config);
        let body = provider.build_request_body(&req, false);
        assert!(body["tools"].is_array());
        assert_eq!(body["tools"][0]["function"]["name"], "test_tool");
    }

    #[test]
    fn test_build_request_body_streaming() {
        let req = CompletionRequest {
            messages: vec![],
            tools: vec![],
            temperature: 0.3,
            max_tokens: None,
            stream: true,
        };
        let config = test_config();
        let provider = OpenAiCompatProvider::new(&config);
        let body = provider.build_request_body(&req, true);
        assert_eq!(body["stream"], true);
    }

    #[test]
    fn test_no_auth_header_when_no_key() {
        let config = AgentConfig {
            provider: crate::config::ProviderKind::Ollama,
            api_key: None,
            ..AgentConfig::default()
        };
        let provider = OpenAiCompatProvider::new(&config);
        assert!(provider.api_key.is_none());
        // The header is only set when sending; we verify the field is None.
        assert_eq!(provider.api_key, None);
    }

    #[test]
    fn test_parse_tool_calls_empty() {
        let value = serde_json::json!({});
        let calls = OpenAiCompatProvider::parse_tool_calls(&value);
        assert!(calls.is_empty());
    }

    #[test]
    fn test_parse_tool_calls_present() {
        let value = serde_json::json!({
            "tool_calls": [
                {
                    "id": "call_123",
                    "function": {
                        "name": "search_symbols",
                        "arguments": r#"{"query":"hello"}"#
                    }
                }
            ]
        });
        let calls = OpenAiCompatProvider::parse_tool_calls(&value);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "call_123");
        assert_eq!(calls[0].name, "search_symbols");
        assert_eq!(calls[0].arguments["query"], "hello");
    }

    #[test]
    fn test_parse_usage_none() {
        let value = serde_json::json!({});
        assert!(OpenAiCompatProvider::parse_usage(&value).is_none());
    }

    #[test]
    fn test_parse_usage_present() {
        let value = serde_json::json!({
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 20,
                "total_tokens": 30
            }
        });
        let usage = OpenAiCompatProvider::parse_usage(&value).unwrap();
        assert_eq!(usage.prompt, 10);
        assert_eq!(usage.completion, 20);
        assert_eq!(usage.total, 30);
    }

    #[test]
    fn test_name() {
        let config = test_config();
        let provider = OpenAiCompatProvider::new(&config);
        assert_eq!(provider.name(), "openai-compatible");
    }
}
