use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Programming language known to the detection and parsing subsystems.
///
/// Each variant corresponds to a language that can be detected by file
/// extension and, where supported, parsed into a syntax tree.
///
/// # Invariants
///
/// - Variants are ordered for deterministic comparison (via `Ord`).
/// - New variants require corresponding parser implementations.
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

/// Error returned when parsing a language name string fails.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown language: '{0}'")]
pub struct ParseLanguageError(pub String);

impl FromStr for Language {
    type Err = ParseLanguageError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "rust" => Ok(Language::Rust),
            "python" => Ok(Language::Python),
            "markdown" | "md" => Ok(Language::Markdown),
            "toml" => Ok(Language::Toml),
            "json" => Ok(Language::Json),
            "yaml" | "yml" => Ok(Language::Yaml),
            "javascript" | "js" => Ok(Language::JavaScript),
            "typescript" | "ts" => Ok(Language::TypeScript),
            "go" => Ok(Language::Go),
            "java" => Ok(Language::Java),
            "ruby" => Ok(Language::Ruby),
            "shell" | "bash" | "sh" => Ok(Language::Shell),
            "css" => Ok(Language::Css),
            "html" => Ok(Language::Html),
            "sql" => Ok(Language::Sql),
            "protobuf" | "proto" => Ok(Language::Protobuf),
            _ => Err(ParseLanguageError(s.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str_case_insensitive() {
        assert_eq!("rust".parse::<Language>().unwrap(), Language::Rust);
        assert_eq!("Rust".parse::<Language>().unwrap(), Language::Rust);
        assert_eq!("RUST".parse::<Language>().unwrap(), Language::Rust);
        assert_eq!("python".parse::<Language>().unwrap(), Language::Python);
        assert_eq!("js".parse::<Language>().unwrap(), Language::JavaScript);
        assert_eq!("ts".parse::<Language>().unwrap(), Language::TypeScript);
        assert_eq!("md".parse::<Language>().unwrap(), Language::Markdown);
        assert_eq!("sh".parse::<Language>().unwrap(), Language::Shell);
        assert_eq!("proto".parse::<Language>().unwrap(), Language::Protobuf);
    }

    #[test]
    fn from_str_unknown() {
        let err = "unknown".parse::<Language>().unwrap_err();
        assert_eq!(err.to_string(), "unknown language: 'unknown'");
    }

    #[test]
    fn from_str_aliases() {
        assert_eq!("yaml".parse::<Language>().unwrap(), Language::Yaml);
        assert_eq!("yml".parse::<Language>().unwrap(), Language::Yaml);
        assert_eq!(
            "javascript".parse::<Language>().unwrap(),
            Language::JavaScript
        );
        assert_eq!(
            "typescript".parse::<Language>().unwrap(),
            Language::TypeScript
        );
        assert_eq!("bash".parse::<Language>().unwrap(), Language::Shell);
        assert_eq!("protobuf".parse::<Language>().unwrap(), Language::Protobuf);
    }
}
