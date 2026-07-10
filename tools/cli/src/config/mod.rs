use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use thiserror::Error;

const CONFIG_FILE: &str = "config.toml";

#[derive(Debug, Clone)]
pub struct Config {
    _path: PathBuf,
    data: BTreeMap<String, toml::Value>,
}

impl Config {
    pub fn load(repo_root: &Path) -> Result<Self, ConfigError> {
        let path = repo_root.join(".kode").join(CONFIG_FILE);
        if !path.exists() {
            return Ok(Self {
                _path: path,
                data: BTreeMap::new(),
            });
        }
        let content = std::fs::read_to_string(&path)?;
        let value: toml::Value = toml::from_str(&content)?;
        let data = flatten_value(&value);
        Ok(Self { _path: path, data })
    }

    pub fn get(&self, key: &str) -> Option<&toml::Value> {
        self.data.get(key)
    }
}

pub fn init(repo_root: &Path) -> Result<PathBuf, ConfigError> {
    let dir = repo_root.join(".kode");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(CONFIG_FILE);
    if !path.exists() {
        std::fs::write(&path, "# kode configuration\n")?;
    }
    Ok(path)
}

pub fn set(repo_root: &Path, key: &str, value: &str) -> Result<(), ConfigError> {
    let dir = repo_root.join(".kode");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(CONFIG_FILE);

    let mut data: toml::Value = if path.exists() {
        let content = std::fs::read_to_string(&path)?;
        toml::from_str(&content).unwrap_or(toml::Value::Table(toml::map::Map::new()))
    } else {
        toml::Value::Table(toml::map::Map::new())
    };

    set_nested(&mut data, key, value);

    let content = toml::to_string_pretty(&data)?;
    std::fs::write(&path, content)?;
    Ok(())
}

fn set_nested(value: &mut toml::Value, key: &str, val: &str) {
    let parts: Vec<&str> = key.split('.').collect();
    let mut current = value;
    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            if let toml::Value::Table(ref mut t) = current {
                t.insert(part.to_string(), toml::Value::String(val.to_string()));
            }
        } else {
            let next = if let toml::Value::Table(ref mut t) = current {
                t.entry(part.to_string())
                    .or_insert_with(|| toml::Value::Table(toml::map::Map::new()))
            } else {
                return;
            };
            current = next;
        }
    }
}

fn flatten_value(value: &toml::Value) -> BTreeMap<String, toml::Value> {
    let mut result = BTreeMap::new();
    flatten_rec(String::new(), value, &mut result);
    result
}

fn flatten_rec(prefix: String, value: &toml::Value, result: &mut BTreeMap<String, toml::Value>) {
    match value {
        toml::Value::Table(t) => {
            for (k, v) in t {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                flatten_rec(key, v, result);
            }
        }
        _ => {
            result.insert(prefix, value.clone());
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatConfig {
    pub provider: String,
    pub model: String,
    pub api_key_value: Option<String>,
    pub api_base: Option<String>,

    pub theme: String,
}

impl ChatConfig {
    pub fn load(repo_root: &Path) -> Self {
        let config = Config::load(repo_root).ok();
        let provider = config
            .as_ref()
            .and_then(|c| c.get("chat.provider"))
            .and_then(|v| v.as_str())
            .unwrap_or("openai")
            .to_string();
        let model = config
            .as_ref()
            .and_then(|c| c.get("chat.model"))
            .and_then(|v| v.as_str())
            .unwrap_or("gpt-4o")
            .to_string();
        let api_key_value = config
            .as_ref()
            .and_then(|c| c.get("chat.api_key"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let api_base = config
            .as_ref()
            .and_then(|c| c.get("chat.base_url"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let theme = config
            .as_ref()
            .and_then(|c| c.get("chat.theme"))
            .and_then(|v| v.as_str())
            .unwrap_or("dark")
            .to_string();
        Self {
            provider,
            model,
            api_key_value,
            api_base,
            theme,
        }
    }

    pub fn api_key(&self) -> Option<String> {
        // Direct value from config takes priority
        if let Some(ref key) = self.api_key_value {
            return Some(key.clone());
        }
        // Fallback to env var
        let env_var = match self.provider.as_str() {
            "anthropic" => "ANTHROPIC_API_KEY",
            "cohere" => "CO_API_KEY",
            "groq" => "GROQ_API_KEY",
            "deepseek" => "DEEPSEEK_API_KEY",
            "ollama" => return None,
            _ => "OPENAI_API_KEY",
        };
        std::env::var(env_var).ok()
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("config parse error: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("config serialize error: {0}")]
    Serialize(#[from] toml::ser::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn repo_root() -> (TempDir, PathBuf) {
        let dir = TempDir::new().unwrap();
        let root = dir.path().to_path_buf();
        (dir, root)
    }

    #[test]
    fn test_config_load_missing_file_returns_empty() {
        let (_keep, root) = repo_root();
        let config = Config::load(&root).unwrap();
        assert!(config.get("anything").is_none());
    }

    #[test]
    fn test_config_load_valid_toml() {
        let (_keep, root) = repo_root();
        let kode_dir = root.join(".kode");
        std::fs::create_dir_all(&kode_dir).unwrap();
        std::fs::write(
            kode_dir.join("config.toml"),
            r#"chat.provider = "anthropic""#,
        )
        .unwrap();
        let config = Config::load(&root).unwrap();
        let val = config.get("chat.provider").unwrap();
        assert_eq!(val.as_str().unwrap(), "anthropic");
    }

    #[test]
    fn test_config_get_missing_key() {
        let (_keep, root) = repo_root();
        let config = Config::load(&root).unwrap();
        assert!(config.get("nonexistent.key").is_none());
    }

    #[test]
    fn test_init_creates_dot_kode_dir_and_config_file() {
        let (_keep, root) = repo_root();
        let path = init(&root).unwrap();
        assert!(path.exists());
        assert_eq!(path.file_name().unwrap().to_str().unwrap(), "config.toml");
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("kode configuration"));
    }

    #[test]
    fn test_init_is_idempotent() {
        let (_keep, root) = repo_root();
        let p1 = init(&root).unwrap();
        let p2 = init(&root).unwrap();
        assert_eq!(p1, p2);
    }

    #[test]
    fn test_set_creates_file_and_sets_value() {
        let (_keep, root) = repo_root();
        set(&root, "chat.provider", "ollama").unwrap();
        let config = Config::load(&root).unwrap();
        let val = config.get("chat.provider").unwrap();
        assert_eq!(val.as_str().unwrap(), "ollama");
    }

    #[test]
    fn test_set_nested_key() {
        let (_keep, root) = repo_root();
        set(&root, "chat.model", "gpt-4").unwrap();
        let config = Config::load(&root).unwrap();
        let val = config.get("chat.model").unwrap();
        assert_eq!(val.as_str().unwrap(), "gpt-4");
    }

    #[test]
    fn test_set_overwrites_existing_value() {
        let (_keep, root) = repo_root();
        set(&root, "chat.provider", "openai").unwrap();
        set(&root, "chat.provider", "anthropic").unwrap();
        let config = Config::load(&root).unwrap();
        let val = config.get("chat.provider").unwrap();
        assert_eq!(val.as_str().unwrap(), "anthropic");
    }

    #[test]
    fn test_flatten_value_simple() {
        let toml_str = r#"key = "val""#;
        let value: toml::Value = toml::from_str(toml_str).unwrap();
        let flat = flatten_value(&value);
        assert_eq!(flat.len(), 1);
        assert_eq!(flat.get("key").unwrap().as_str().unwrap(), "val");
    }

    #[test]
    fn test_flatten_value_nested() {
        let toml_str = r#"[chat]
provider = "openai"
model = "gpt-4""#;
        let value: toml::Value = toml::from_str(toml_str).unwrap();
        let flat = flatten_value(&value);
        assert_eq!(flat.len(), 2);
        assert_eq!(
            flat.get("chat.provider").unwrap().as_str().unwrap(),
            "openai"
        );
        assert_eq!(flat.get("chat.model").unwrap().as_str().unwrap(), "gpt-4");
    }

    #[test]
    fn test_set_nested_flat_key() {
        let mut value = toml::Value::Table(toml::map::Map::new());
        set_nested(&mut value, "name", "test");
        assert_eq!(value.get("name").unwrap().as_str().unwrap(), "test");
    }

    #[test]
    fn test_set_nested_dotted_key() {
        let mut value = toml::Value::Table(toml::map::Map::new());
        set_nested(&mut value, "a.b.c", "deep");
        let a = value.get("a").unwrap().as_table().unwrap();
        let b = a.get("b").unwrap().as_table().unwrap();
        assert_eq!(b.get("c").unwrap().as_str().unwrap(), "deep");
    }

    #[test]
    fn test_chat_config_defaults() {
        let (_keep, root) = repo_root();
        let cc = ChatConfig::load(&root);
        assert_eq!(cc.provider, "openai");
        assert_eq!(cc.model, "gpt-4o");
        assert!(cc.api_key_value.is_none());
        assert!(cc.api_base.is_none());
        assert_eq!(cc.theme, "dark");
    }

    #[test]
    fn test_chat_config_reads_from_file() {
        let (_keep, root) = repo_root();
        let kode_dir = root.join(".kode");
        std::fs::create_dir_all(&kode_dir).unwrap();
        std::fs::write(
            kode_dir.join("config.toml"),
            r#"[chat]
provider = "anthropic"
model = "claude-3"
theme = "light"
api_key = "sk-ant-xxx""#,
        )
        .unwrap();
        let cc = ChatConfig::load(&root);
        assert_eq!(cc.provider, "anthropic");
        assert_eq!(cc.model, "claude-3");
        assert_eq!(cc.api_key_value.as_deref(), Some("sk-ant-xxx"));
        assert!(cc.api_base.is_none());
        assert_eq!(cc.theme, "light");
    }

    #[test]
    fn test_chat_config_api_key_falls_back_to_env() {
        let (_keep, root) = repo_root();
        let cc = ChatConfig::load(&root);
        assert!(cc.api_key_value.is_none());
        // No env var set, should return None
        assert!(cc.api_key().is_none());
    }

    #[test]
    fn test_chat_config_api_key_from_value() {
        let (_keep, _root) = repo_root();
        let cc = ChatConfig {
            provider: "openai".into(),
            model: "gpt-4o".into(),
            api_key_value: Some("sk-from-config".into()),
            api_base: None,
            theme: "dark".into(),
        };
        assert_eq!(cc.api_key(), Some("sk-from-config".into()));
    }
}
