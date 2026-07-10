use futures_util::StreamExt;
use reqwest::Client;
use serde_json::json;

use crate::config::ChatConfig;

use super::ChatError;

#[derive(Debug)]
pub enum StreamEvent {
    Token(String),
    Done,
}

pub struct ChatEngine {
    client: Client,
    model: String,
    api_base: String,
    api_key: Option<String>,
}

impl ChatEngine {
    pub fn new(config: &ChatConfig) -> Result<Self, ChatError> {
        let api_key = config.api_key();
        if api_key.is_none() && config.provider != "ollama" {
            return Err(ChatError::Engine(
                "No API key configured. Set OPENAI_API_KEY environment variable or configure chat.api_key in .kode/config.toml"
                    .into(),
            ));
        }
        let api_base = config
            .api_base
            .clone()
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
        Ok(Self {
            client: Client::new(),
            model: config.model.clone(),
            api_base,
            api_key,
        })
    }

    pub fn complete(&self, history: &[(String, String)], query: &str) -> Result<String, ChatError> {
        let mut messages: Vec<serde_json::Value> = history
            .iter()
            .map(|(role, content)| json!({"role": role, "content": content}))
            .collect();
        messages.push(json!({"role": "user", "content": query}));

        let body = json!({
            "model": self.model,
            "messages": messages,
        });

        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| ChatError::Engine(format!("Failed to start runtime: {e}")))?;

        runtime.block_on(self.send_request(body))
    }

    async fn send_request(&self, body: serde_json::Value) -> Result<String, ChatError> {
        let mut req = self
            .client
            .post(format!("{}/chat/completions", self.api_base))
            .json(&body);

        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let resp = req
            .send()
            .await
            .map_err(|e| ChatError::Engine(format!("Request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(ChatError::Engine(format!(
                "API error ({}): {}",
                status, text
            )));
        }

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ChatError::Engine(format!("Parse error: {e}")))?;

        Ok(data["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string())
    }

    pub fn complete_stream(
        &self,
        history: &[(String, String)],
        query: &str,
    ) -> Result<tokio::sync::mpsc::Receiver<StreamEvent>, ChatError> {
        let mut messages: Vec<serde_json::Value> = history
            .iter()
            .map(|(role, content)| json!({"role": role, "content": content}))
            .collect();
        messages.push(json!({"role": "user", "content": query}));

        let body = json!({
            "model": self.model,
            "messages": messages,
            "stream": true,
        });

        let (tx, rx) = tokio::sync::mpsc::channel(128);
        let client = self.client.clone();
        let api_base = self.api_base.clone();
        let api_key = self.api_key.clone();

        tokio::spawn(async move {
            let mut req = client
                .post(format!("{}/chat/completions", api_base))
                .json(&body);

            if let Some(ref key) = api_key {
                req = req.header("Authorization", format!("Bearer {}", key));
            }

            match req.send().await {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        let status = resp.status();
                        let text = resp.text().await.unwrap_or_default();
                        let _ = tx
                            .send(StreamEvent::Token(format!("Error ({}): {}", status, text)))
                            .await;
                        let _ = tx.send(StreamEvent::Done).await;
                        return;
                    }

                    let mut buffer = String::new();
                    let mut stream = resp.bytes_stream();
                    while let Some(chunk) = stream.next().await {
                        match chunk {
                            Ok(bytes) => {
                                let text = String::from_utf8_lossy(&bytes);
                                buffer.push_str(&text);
                                while let Some(newline) = buffer.find('\n') {
                                    let line = buffer[..newline].trim().to_string();
                                    buffer = buffer[newline + 1..].to_string();
                                    if let Some(data) = line.strip_prefix("data: ") {
                                        if data == "[DONE]" {
                                            let _ = tx.send(StreamEvent::Done).await;
                                            return;
                                        }
                                        if let Ok(val) =
                                            serde_json::from_str::<serde_json::Value>(data)
                                        {
                                            if let Some(content) =
                                                val["choices"][0]["delta"]["content"].as_str()
                                            {
                                                if !content.is_empty() {
                                                    let _ = tx
                                                        .send(StreamEvent::Token(
                                                            content.to_string(),
                                                        ))
                                                        .await;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                let _ = tx
                                    .send(StreamEvent::Token(format!("Stream error: {}", e)))
                                    .await;
                                let _ = tx.send(StreamEvent::Done).await;
                                return;
                            }
                        }
                    }
                    let _ = tx.send(StreamEvent::Done).await;
                }
                Err(e) => {
                    let _ = tx
                        .send(StreamEvent::Token(format!("Request failed: {}", e)))
                        .await;
                    let _ = tx.send(StreamEvent::Done).await;
                }
            }
        });

        Ok(rx)
    }
}
