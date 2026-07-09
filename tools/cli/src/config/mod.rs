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

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("config parse error: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("config serialize error: {0}")]
    Serialize(#[from] toml::ser::Error),
}
