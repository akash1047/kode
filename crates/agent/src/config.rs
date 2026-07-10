use std::fmt;

/// Known provider identifiers with fixed base URLs.
/// Custom names like `"mistral"`, `"together"` use `Custom` + `api_base` override.
#[derive(Debug, Clone, PartialEq)]
pub enum ProviderKind {
    OpenAI,
    DeepSeek,
    Groq,
    OpenRouter,
    Ollama,
    Custom(String),
}

impl ProviderKind {
    /// Known base URL for this provider.
    /// Returns `None` for `Custom` and `Ollama` — user must provide `api_base`
    /// or accept defaults.
    pub fn known_base_url(&self) -> Option<&'static str> {
        match self {
            ProviderKind::OpenAI => Some("https://api.openai.com/v1/chat/completions"),
            ProviderKind::DeepSeek => Some("https://api.deepseek.com/v1/chat/completions"),
            ProviderKind::Groq => Some("https://api.groq.com/openai/v1/chat/completions"),
            ProviderKind::OpenRouter => Some("https://openrouter.ai/api/v1/chat/completions"),
            ProviderKind::Ollama => None,
            ProviderKind::Custom(_) => None,
        }
    }
}

impl std::str::FromStr for ProviderKind {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "openai" => Self::OpenAI,
            "deepseek" => Self::DeepSeek,
            "groq" => Self::Groq,
            "openrouter" => Self::OpenRouter,
            "ollama" => Self::Ollama,
            custom => Self::Custom(custom.to_string()),
        })
    }
}

impl fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OpenAI => write!(f, "openai"),
            Self::DeepSeek => write!(f, "deepseek"),
            Self::Groq => write!(f, "groq"),
            Self::OpenRouter => write!(f, "openrouter"),
            Self::Ollama => write!(f, "ollama"),
            Self::Custom(name) => write!(f, "{name}"),
        }
    }
}

/// Configures a single agent session.
///
/// Config-source-agnostic — CLI reads from toml/env, MCP builds from its own
/// config. The agent crate never reads env vars or files directly.
#[derive(Clone)]
pub struct AgentConfig {
    pub provider: ProviderKind,
    pub model: String,
    pub api_key: Option<String>,
    pub max_turns: usize,
    pub timeout_secs: u64,
    pub api_base: Option<String>,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            provider: ProviderKind::OpenAI,
            model: "gpt-4o".into(),
            api_key: None,
            max_turns: 10,
            timeout_secs: 30,
            api_base: None,
            temperature: 0.3,
            max_tokens: None,
        }
    }
}

impl fmt::Debug for AgentConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AgentConfig")
            .field("provider", &self.provider)
            .field("model", &self.model)
            .field("api_key", &self.api_key.as_deref().map(|_| "***"))
            .field("max_turns", &self.max_turns)
            .field("timeout_secs", &self.timeout_secs)
            .field("api_base", &self.api_base)
            .field("temperature", &self.temperature)
            .field("max_tokens", &self.max_tokens)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_kind_from_str() {
        assert_eq!(
            "openai".parse::<ProviderKind>().unwrap(),
            ProviderKind::OpenAI
        );
        assert_eq!(
            "OPENAI".parse::<ProviderKind>().unwrap(),
            ProviderKind::OpenAI
        );
        assert_eq!(
            "deepseek".parse::<ProviderKind>().unwrap(),
            ProviderKind::DeepSeek
        );
        assert_eq!("groq".parse::<ProviderKind>().unwrap(), ProviderKind::Groq);
        assert_eq!(
            "openrouter".parse::<ProviderKind>().unwrap(),
            ProviderKind::OpenRouter
        );
        assert_eq!(
            "ollama".parse::<ProviderKind>().unwrap(),
            ProviderKind::Ollama
        );
    }

    #[test]
    fn test_provider_kind_custom() {
        let kind: ProviderKind = "mistral".parse().unwrap();
        assert_eq!(kind, ProviderKind::Custom("mistral".into()));
    }

    #[test]
    fn test_provider_kind_display_roundtrip() {
        for name in &["openai", "deepseek", "groq", "openrouter", "ollama"] {
            let kind: ProviderKind = name.parse().unwrap();
            assert_eq!(kind.to_string().as_str(), *name);
        }
        let custom = ProviderKind::Custom("xyz".into());
        assert_eq!(custom.to_string(), "xyz");
    }

    #[test]
    fn test_known_base_url() {
        assert_eq!(
            ProviderKind::OpenAI.known_base_url(),
            Some("https://api.openai.com/v1/chat/completions")
        );
        assert_eq!(
            ProviderKind::DeepSeek.known_base_url(),
            Some("https://api.deepseek.com/v1/chat/completions")
        );
        assert_eq!(
            ProviderKind::Groq.known_base_url(),
            Some("https://api.groq.com/openai/v1/chat/completions")
        );
        assert_eq!(
            ProviderKind::OpenRouter.known_base_url(),
            Some("https://openrouter.ai/api/v1/chat/completions")
        );
        assert_eq!(ProviderKind::Ollama.known_base_url(), None);
        assert_eq!(ProviderKind::Custom("x".into()).known_base_url(), None);
    }

    #[test]
    fn test_agent_config_default() {
        let cfg = AgentConfig::default();
        assert_eq!(cfg.provider, ProviderKind::OpenAI);
        assert_eq!(cfg.model, "gpt-4o");
        assert!(cfg.api_key.is_none());
        assert_eq!(cfg.max_turns, 10);
        assert_eq!(cfg.timeout_secs, 30);
        assert_eq!(cfg.temperature, 0.3);
        assert!(cfg.api_base.is_none());
        assert!(cfg.max_tokens.is_none());
    }

    #[test]
    fn test_agent_config_debug_redacts_key() {
        let cfg = AgentConfig {
            api_key: Some("sk-secret123".into()),
            ..AgentConfig::default()
        };
        let debug_str = format!("{:?}", cfg);
        assert!(!debug_str.contains("sk-secret123"));
        assert!(debug_str.contains("***"));
    }

    #[test]
    fn test_agent_config_debug_no_key() {
        let cfg = AgentConfig::default();
        let debug_str = format!("{:?}", cfg);
        assert!(debug_str.contains("None"));
    }
}
