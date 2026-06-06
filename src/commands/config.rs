use std::path::PathBuf;
use std::process;

use toml::Value;

use crate::config::config_path;

const DEFAULT_TEMPLATE: &str = "\
[default]
# api_key = \"\"
# model = \"nemotron-3-nano:30b\"
# base_url = \"https://ollama.com/v1\"

[chat]
# api_key = \"\"
# model = \"\"
# base_url = \"\"

[summarize]
# api_key = \"\"
# model = \"\"
# base_url = \"\"
# max_input_chars = 8000
";

pub fn init() {
    let path = resolve_path();
    if path.exists() {
        eprintln!("config already exists: {}", path.display());
        eprintln!("edit it directly, or delete it and re-run init");
        process::exit(1);
    }
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!("failed to create config dir: {e}");
            process::exit(1);
        }
    }
    if let Err(e) = std::fs::write(&path, DEFAULT_TEMPLATE) {
        eprintln!("failed to write config: {e}");
        process::exit(1);
    }
    println!("created: {}", path.display());
}

pub fn show() {
    let path = resolve_path();
    if !path.exists() {
        eprintln!("no config at {} — run `kode config init` to create one", path.display());
        process::exit(1);
    }
    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("failed to read config: {e}");
            process::exit(1);
        }
    };
    print!("{}", censor_api_keys(&raw));
}

pub fn get(key: &str) {
    let key = normalize_key(key);
    let path = resolve_path();
    let raw = path
        .exists()
        .then(|| std::fs::read_to_string(&path).ok())
        .flatten()
        .unwrap_or_default();
    let table: toml::Table = toml::from_str(&raw).unwrap_or_default();
    match get_nested(&table, &key) {
        Some(v) => println!("{}", display_value(v)),
        None => {
            eprintln!("key not found: {key}");
            process::exit(1);
        }
    }
}

pub fn set(key: &str, value: &str) {
    let key = normalize_key(key);
    let path = resolve_path();
    let raw = path
        .exists()
        .then(|| std::fs::read_to_string(&path).ok())
        .flatten()
        .unwrap_or_default();
    let mut table: toml::Table = toml::from_str(&raw).unwrap_or_default();

    let toml_value = parse_value(value);
    if let Err(e) = set_nested(&mut table, &key, toml_value) {
        eprintln!("{e}");
        process::exit(1);
    }

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let serialized = toml::to_string_pretty(&table).expect("toml serialize");
    if let Err(e) = std::fs::write(&path, serialized) {
        eprintln!("failed to write config: {e}");
        process::exit(1);
    }
    let display = if is_sensitive_key(&key) { "****".to_string() } else { value.to_string() };
    println!("set {key} = {display}");
}

fn normalize_key(key: &str) -> String {
    if key.contains('.') {
        key.to_string()
    } else {
        format!("default.{key}")
    }
}

fn censor_api_keys(raw: &str) -> String {
    raw.lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') {
                return line.to_string();
            }
            if let Some(eq) = line.find('=') {
                let key_part = line[..eq].trim();
                if key_part == "api_key" {
                    let indent = &line[..line.len() - trimmed.len()];
                    return format!("{indent}api_key = \"****\"");
                }
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
        + if raw.ends_with('\n') { "\n" } else { "" }
}

fn is_sensitive_key(key: &str) -> bool {
    key.split('.').last() == Some("api_key")
}

fn get_nested<'a>(table: &'a toml::Table, key: &str) -> Option<&'a Value> {
    match key.splitn(2, '.').collect::<Vec<_>>().as_slice() {
        [k] => table.get(*k),
        [section, rest] => table.get(*section)?.as_table().and_then(|t| get_nested(t, rest)),
        _ => None,
    }
}

fn set_nested(table: &mut toml::Table, key: &str, value: Value) -> Result<(), String> {
    match key.splitn(2, '.').collect::<Vec<_>>().as_slice() {
        [k] => {
            table.insert(k.to_string(), value);
            Ok(())
        }
        [section, rest] => {
            let entry = table
                .entry(section.to_string())
                .or_insert_with(|| Value::Table(toml::Table::new()));
            match entry {
                Value::Table(t) => set_nested(t, rest, value),
                _ => Err(format!("'{section}' is not a table")),
            }
        }
        _ => Err(format!("invalid key: {key}")),
    }
}

fn parse_value(s: &str) -> Value {
    if let Ok(n) = s.parse::<i64>() {
        return Value::Integer(n);
    }
    if let Ok(b) = s.parse::<bool>() {
        return Value::Boolean(b);
    }
    Value::String(s.to_string())
}

fn display_value(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Integer(n) => n.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Boolean(b) => b.to_string(),
        other => other.to_string(),
    }
}

fn resolve_path() -> PathBuf {
    config_path().unwrap_or_else(|| PathBuf::from("config.toml"))
}
