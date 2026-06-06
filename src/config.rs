use std::path::PathBuf;

use serde::Deserialize;

/// Top-level configuration loaded from `~/.config/kode/config.toml`.
#[derive(Debug, Default, Deserialize)]
pub struct KodeConfig {
    pub default: Option<DefaultSection>,
    pub chat: Option<ChatSection>,
    pub summarize: Option<SummarizeSection>,
}

/// Fallback values used when a command-specific section omits a field.
#[derive(Debug, Deserialize)]
pub struct DefaultSection {
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
}

/// Configuration for the `kode chat` command.
#[derive(Debug, Deserialize)]
pub struct ChatSection {
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
}

/// Configuration for the file-summarisation pass (cache drawer 3).
#[derive(Debug, Deserialize)]
pub struct SummarizeSection {
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub max_input_chars: Option<usize>,
}

impl KodeConfig {
    /// Load config from disk. Returns `Default` if the file is absent or unparseable.
    pub fn load() -> Self {
        config_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }
}

/// ~/.config/kode/config.toml
pub fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("kode").join("config.toml"))
}
