use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Language {
    Rust,
    Python,
    Markdown,
    Toml,
    Json,
    Yaml,
    JavaScript,
    TypeScript,
    Go,
    Java,
    Ruby,
    Shell,
    Css,
    Html,
    Sql,
    Protobuf,
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Language::Rust => write!(f, "Rust"),
            Language::Python => write!(f, "Python"),
            Language::Markdown => write!(f, "Markdown"),
            Language::Toml => write!(f, "TOML"),
            Language::Json => write!(f, "JSON"),
            Language::Yaml => write!(f, "YAML"),
            Language::JavaScript => write!(f, "JavaScript"),
            Language::TypeScript => write!(f, "TypeScript"),
            Language::Go => write!(f, "Go"),
            Language::Java => write!(f, "Java"),
            Language::Ruby => write!(f, "Ruby"),
            Language::Shell => write!(f, "Shell"),
            Language::Css => write!(f, "CSS"),
            Language::Html => write!(f, "HTML"),
            Language::Sql => write!(f, "SQL"),
            Language::Protobuf => write!(f, "Protobuf"),
        }
    }
}
