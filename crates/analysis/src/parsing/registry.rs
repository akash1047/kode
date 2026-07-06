use kode_acquisition::Language;

use crate::parsing::parser::Parser;

/// Registry of language-specific parsers with language-keyed dispatch.
///
/// # Dispatch
///
/// [`dispatch`](Self::dispatch) looks up a parser by language. The lookup
/// is O(n) in the number of registered parsers. Registration order does
/// not affect dispatch (each language maps to exactly one parser).
///
/// # Extension
///
/// New parsers implement [`Parser`] and register via [`register`](Self::register).
/// The default registry includes the Rust parser only.
///
/// # Invariants
///
/// - Only one parser per language is expected. If multiple parsers for the
///   same language are registered, the first one wins.
/// - Unrecognized languages return `None` (the outcome is [`ParseOutcome::Skipped`]).
///
/// [`ParseOutcome::Skipped`]: crate::parsing::ParseOutcome
#[derive(Debug)]
pub struct ParserRegistry {
    parsers: Vec<Box<dyn Parser>>,
}

impl ParserRegistry {
    pub fn new() -> Self {
        Self {
            parsers: Vec::new(),
        }
    }

    pub fn register(&mut self, parser: Box<dyn Parser>) {
        self.parsers.push(parser);
    }

    pub fn dispatch(&self, language: &Language) -> Option<&dyn Parser> {
        self.parsers
            .iter()
            .find(|p| p.language() == *language)
            .map(|p| p.as_ref())
    }

    pub fn iter(&self) -> impl Iterator<Item = &dyn Parser> {
        self.parsers.iter().map(|p| p.as_ref())
    }
}

impl Default for ParserRegistry {
    fn default() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(crate::parsing::rust::RustParser));
        registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::sync::Arc;

    use kode_acquisition::{FileMetadata, Language, RepositoryFile};

    use crate::parsing::diagnostic::Severity;
    use crate::parsing::result::ParseOutcome;
    use crate::parsing::syntax::SyntaxTree;
    use crate::parsing::ts;

    #[derive(Debug)]
    struct MockRustParser;

    impl Parser for MockRustParser {
        fn language(&self) -> Language {
            Language::Rust
        }

        fn parse(&self, source: Arc<str>, file: &RepositoryFile) -> ParseOutcome {
            match ts::parse_source(Language::Rust, &source) {
                Ok((backend, diagnostics)) => {
                    let has_errors = diagnostics.iter().any(|d| *d.severity() == Severity::Error);
                    ParseOutcome::Success(SyntaxTree::new(
                        file.relative_path().to_path_buf(),
                        Language::Rust,
                        source,
                        has_errors,
                        diagnostics,
                        "mock".to_string(),
                        None,
                        None,
                        Some("tree-sitter".to_string()),
                        backend,
                    ))
                }
                Err(_) => ParseOutcome::Failed(Vec::new()),
            }
        }
    }

    #[derive(Debug)]
    struct MockGoParser;

    impl Parser for MockGoParser {
        fn language(&self) -> Language {
            Language::Go
        }

        fn parse(&self, _source: Arc<str>, _file: &RepositoryFile) -> ParseOutcome {
            ParseOutcome::Skipped(crate::parsing::result::SkipReason::UnsupportedLanguage)
        }
    }

    fn unknown_file() -> RepositoryFile {
        RepositoryFile::new(Path::new("unknown.xyz"), FileMetadata::new(100, None), None)
    }

    #[test]
    fn empty_registry_returns_no_parser() {
        let registry = ParserRegistry::new();
        assert!(registry.dispatch(&Language::Rust).is_none());
    }

    #[test]
    fn dispatch_finds_parser_by_language() {
        let mut registry = ParserRegistry::new();
        registry.register(Box::new(MockRustParser));
        registry.register(Box::new(MockGoParser));
        assert!(registry.dispatch(&Language::Rust).is_some());
        assert!(registry.dispatch(&Language::Go).is_some());
        assert!(registry.dispatch(&Language::Python).is_none());
    }

    #[test]
    fn first_matching_parser_wins() {
        let mut registry = ParserRegistry::new();
        registry.register(Box::new(MockRustParser));
        registry.register(Box::new(MockGoParser));
        let parser = registry.dispatch(&Language::Rust).unwrap();
        assert_eq!(parser.language(), Language::Rust);
    }

    #[test]
    fn unsupported_language_returns_none() {
        let mut registry = ParserRegistry::new();
        registry.register(Box::new(MockRustParser));
        assert!(registry.dispatch(&Language::Go).is_none());
    }

    #[test]
    fn no_matching_parser_for_unknown_language() {
        let mut registry = ParserRegistry::new();
        registry.register(Box::new(MockRustParser));
        let file = unknown_file();
        let lang = file.language();
        let dispatched = lang.and_then(|l| registry.dispatch(l));
        assert!(dispatched.is_none());
    }

    #[test]
    fn default_registry_includes_rust_parser() {
        let registry = ParserRegistry::default();
        assert!(registry.dispatch(&Language::Rust).is_some());
        assert!(registry.dispatch(&Language::Go).is_none());
    }

    #[test]
    fn registry_iter() {
        let mut registry = ParserRegistry::new();
        registry.register(Box::new(MockRustParser));
        registry.register(Box::new(MockGoParser));
        let count = registry.iter().count();
        assert_eq!(count, 2);
    }
}
