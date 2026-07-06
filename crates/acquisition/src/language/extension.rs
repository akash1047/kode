use std::path::Path;

use crate::language::detector::LanguageDetector;
use crate::language::model::Language;

pub struct ExtensionLanguageDetector;

impl LanguageDetector for ExtensionLanguageDetector {
    fn detect(&self, path: &Path) -> Option<Language> {
        let ext = path.extension()?.to_str()?.to_lowercase();
        match ext.as_str() {
            "rs" => Some(Language::Rust),
            "md" | "markdown" => Some(Language::Markdown),
            "toml" => Some(Language::Toml),
            "json" => Some(Language::Json),
            "yaml" | "yml" => Some(Language::Yaml),
            "py" => Some(Language::Python),
            "js" => Some(Language::JavaScript),
            "ts" | "tsx" => Some(Language::TypeScript),
            "go" => Some(Language::Go),
            "java" => Some(Language::Java),
            "rb" => Some(Language::Ruby),
            "sh" | "bash" | "zsh" => Some(Language::Shell),
            "css" => Some(Language::Css),
            "html" | "htm" => Some(Language::Html),
            "sql" => Some(Language::Sql),
            "proto" => Some(Language::Protobuf),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn rust_extension() {
        let detector = ExtensionLanguageDetector;
        assert_eq!(
            detector.detect(&PathBuf::from("main.rs")),
            Some(Language::Rust)
        );
    }

    #[test]
    fn markdown_extensions() {
        let detector = ExtensionLanguageDetector;
        assert_eq!(
            detector.detect(&PathBuf::from("README.md")),
            Some(Language::Markdown)
        );
        assert_eq!(
            detector.detect(&PathBuf::from("CHANGELOG.markdown")),
            Some(Language::Markdown)
        );
    }

    #[test]
    fn toml_extension() {
        let detector = ExtensionLanguageDetector;
        assert_eq!(
            detector.detect(&PathBuf::from("Cargo.toml")),
            Some(Language::Toml)
        );
    }

    #[test]
    fn unknown_extension_returns_none() {
        let detector = ExtensionLanguageDetector;
        assert_eq!(detector.detect(&PathBuf::from("file.xyz")), None);
    }

    #[test]
    fn no_extension_returns_none() {
        let detector = ExtensionLanguageDetector;
        assert_eq!(detector.detect(&PathBuf::from("Makefile")), None);
    }

    #[test]
    fn language_display() {
        assert_eq!(Language::Rust.to_string(), "Rust");
        assert_eq!(Language::Python.to_string(), "Python");
        assert_eq!(Language::Markdown.to_string(), "Markdown");
    }
}
