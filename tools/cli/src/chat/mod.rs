//! Interactive and one-shot chat backed by `kode-agent`.

mod state;
mod tui;
mod ui;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use kode_agent::{
    Agent, AgentConfig, OpenAiCompatProvider, ProviderKind, SharedIndex, SymbolIndex,
};
use kode_query::QueryEngine;
use kode_storage::{RepositoryStorage, SqliteBackend};
use tokio::sync::Mutex;

/// Result of a one-shot chat turn (for CLI text/json formatters).
pub struct ChatView {
    pub answer: String,
}

/// Entry point for `kode chat` / `kode chat -m`.
pub fn handle_chat(
    path: Option<&str>,
    message: Option<&str>,
    _no_color: bool,
) -> Result<ChatView, Box<dyn std::error::Error>> {
    let repo_path = path.unwrap_or(".");
    let root = Path::new(repo_path);
    let absolute = root
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(repo_path));

    let chat_cfg = crate::config::ChatConfig::load(root);
    let agent_cfg = agent_config_from_chat(&chat_cfg)?;
    let provider = Box::new(OpenAiCompatProvider::new(&agent_cfg));

    let symbol_index = try_load_symbol_index(&absolute);
    let has_index = symbol_index.is_some();

    let agent = Agent::new(agent_cfg.clone(), provider, symbol_index, absolute.clone());

    match message {
        Some(query) => {
            let rt = tokio::runtime::Runtime::new()?;
            let mut agent = agent;
            let answer = rt.block_on(async { agent.run(query).await })?;
            Ok(ChatView { answer })
        }
        None => {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            let agent = Arc::new(Mutex::new(agent));
            let model = agent_cfg.model.clone();
            let provider_name = agent_cfg.provider.to_string();
            if !has_index {
                tracing::info!("chat: no graph index; filesystem tools only");
            }
            rt.block_on(tui::run(agent, model, provider_name))
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            Ok(ChatView {
                answer: String::new(),
            })
        }
    }
}

fn agent_config_from_chat(
    chat: &crate::config::ChatConfig,
) -> Result<AgentConfig, Box<dyn std::error::Error>> {
    let mut provider: ProviderKind = chat.provider.parse().unwrap_or(ProviderKind::OpenAI);

    // Infer Ollama from base URL / model when provider was left at default.
    let base_hint = chat.api_base.as_deref().unwrap_or("");
    if base_hint.contains("ollama.com") || base_hint.contains("localhost:11434") {
        provider = ProviderKind::Ollama;
    } else if chat.model.contains(':') && chat.provider == "openai" {
        // Models like `nemotron-3-nano:30b` are Ollama-style tags.
        if chat
            .api_base
            .as_deref()
            .is_some_and(|b| b.contains("ollama"))
        {
            provider = ProviderKind::Ollama;
        }
    }

    let api_key = chat
        .api_key()
        .or_else(|| std::env::var("OLLAMA_API_KEY").ok())
        .or_else(|| std::env::var("OPENAI_API_KEY").ok());

    if api_key.is_none() && !matches!(provider, ProviderKind::Ollama) {
        return Err(
            "No API key configured. Set OPENAI_API_KEY / OLLAMA_API_KEY or chat.api_key in .kode/config.toml"
                .into(),
        );
    }

    let api_base = chat.api_base.clone().or_else(|| {
        if matches!(provider, ProviderKind::Ollama) {
            std::env::var("OLLAMA_HOST")
                .ok()
                .map(|h| h.trim_end_matches('/').to_string())
                .or_else(|| std::env::var("OLLAMA_API_BASE_URL").ok())
                .or_else(|| {
                    // Default Ollama Cloud OpenAI-compatible root when key looks set for cloud.
                    if std::env::var("OLLAMA_API_KEY").is_ok() || chat.api_key_value.is_some() {
                        Some("https://ollama.com/v1".into())
                    } else {
                        Some("http://localhost:11434/v1".into())
                    }
                })
        } else {
            None
        }
    });

    Ok(AgentConfig {
        provider,
        model: chat.model.clone(),
        api_key,
        api_base,
        max_turns: 10,
        timeout_secs: 120,
        temperature: 0.3,
        max_tokens: None,
    })
}

fn try_load_symbol_index(absolute: &Path) -> Option<SharedIndex> {
    let db_path = absolute.join(".kode").join("cache.db");
    if !db_path.exists() {
        return None;
    }
    let backend = SqliteBackend::open(&db_path).ok()?;
    let repo_id = absolute.display().to_string();
    let storage = RepositoryStorage::open(Box::new(backend), &repo_id).ok()?;
    let engine = QueryEngine::new(storage);
    let index = SymbolIndex::from_query_engine(&engine).ok()?;
    Some(Arc::new(index))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_view_construction() {
        let view = ChatView {
            answer: "test answer".into(),
        };
        assert_eq!(view.answer, "test answer");
    }

    #[test]
    fn agent_config_ollama_allows_no_key() {
        let chat = crate::config::ChatConfig {
            provider: "ollama".into(),
            model: "llama3".into(),
            api_key_value: None,
            api_base: Some("http://localhost:11434/v1".into()),
            theme: "dark".into(),
        };
        let cfg = agent_config_from_chat(&chat).unwrap();
        assert!(matches!(cfg.provider, ProviderKind::Ollama));
        assert_eq!(cfg.model, "llama3");
    }

    #[test]
    fn agent_config_infers_ollama_from_base_url() {
        let chat = crate::config::ChatConfig {
            provider: "openai".into(), // mis-set default
            model: "nemotron-3-nano:30b".into(),
            api_key_value: Some("test-key".into()),
            api_base: Some("https://ollama.com/api/v1".into()),
            theme: "dark".into(),
        };
        let cfg = agent_config_from_chat(&chat).unwrap();
        assert!(matches!(cfg.provider, ProviderKind::Ollama));
        // Provider still receives the raw base; OpenAiCompatProvider normalizes.
        assert_eq!(cfg.api_base.as_deref(), Some("https://ollama.com/api/v1"));
    }

    #[test]
    fn agent_config_openai_requires_key() {
        let chat = crate::config::ChatConfig {
            provider: "openai".into(),
            model: "gpt-4o".into(),
            api_key_value: None,
            api_base: None,
            theme: "dark".into(),
        };
        match agent_config_from_chat(&chat) {
            Ok(cfg) => assert!(cfg.api_key.is_some()),
            Err(e) => assert!(e.to_string().contains("API key")),
        }
    }
}
