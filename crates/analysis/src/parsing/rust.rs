use std::sync::Arc;

use kode_acquisition::{Language, RepositoryFile};

use crate::parsing::diagnostic::{Diagnostic, Severity};
use crate::parsing::parser::Parser;
use crate::parsing::result::ParseOutcome;
use crate::parsing::syntax::SyntaxTree;
use crate::parsing::ts;

/// Parser for Rust source files using the Tree-sitter Rust grammar.
///
/// Dispatches to the architecture-specific Tree-sitter parse function
/// and wraps the result into a [`SyntaxTree`] with parser metadata.
///
/// # Outcome Classification
///
/// - No diagnostics with `Error` severity → [`ParseOutcome::Success`]
/// - Any diagnostic with `Error` severity → [`ParseOutcome::Recovered`]
/// - Parser initialization failure → [`ParseOutcome::Failed`]
#[derive(Debug)]
pub struct RustParser;

impl Parser for RustParser {
    fn language(&self) -> Language {
        Language::Rust
    }

    fn parse(&self, source: Arc<str>, file: &RepositoryFile) -> ParseOutcome {
        match ts::parse_source(Language::Rust, &source) {
            Ok((backend, diagnostics)) => {
                let has_errors = diagnostics.iter().any(|d| *d.severity() == Severity::Error);
                let syntax_tree = SyntaxTree::new(
                    file.relative_path().to_path_buf(),
                    Language::Rust,
                    source,
                    has_errors,
                    diagnostics,
                    "tree-sitter-rust".to_string(),
                    None,
                    None,
                    Some("tree-sitter".to_string()),
                    backend,
                );

                if has_errors {
                    ParseOutcome::Recovered(syntax_tree)
                } else {
                    ParseOutcome::Success(syntax_tree)
                }
            }
            Err(e) => ParseOutcome::Failed(vec![Diagnostic::error(e.to_string(), 0, 0, 0, 0)]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    use kode_acquisition::FileMetadata;

    fn rust_file(content: &str) -> RepositoryFile {
        RepositoryFile::new(
            Path::new("test.rs"),
            FileMetadata::new(content.len() as u64, None),
            Some(Language::Rust),
        )
    }

    #[test]
    fn parse_valid_rust() {
        let parser = RustParser;
        let file = rust_file("fn main() { let x = 1; }");
        let source = Arc::from("fn main() { let x = 1; }");
        let result = parser.parse(source, &file);
        match result {
            ParseOutcome::Success(tree) => {
                assert_eq!(tree.relative_path(), Path::new("test.rs"));
                assert_eq!(tree.language(), &Language::Rust);
                assert!(!tree.has_errors());
                assert!(tree.diagnostics().is_empty());
            }
            _ => panic!("expected Success, got {:?}", result),
        }
    }

    #[test]
    fn parse_invalid_rust() {
        let parser = RustParser;
        let file = rust_file("fn main() { let x = ; }");
        let source = Arc::from("fn main() { let x = ; }");
        let result = parser.parse(source, &file);
        match result {
            ParseOutcome::Recovered(tree) => {
                assert!(tree.has_errors());
                assert!(!tree.diagnostics().is_empty());
            }
            _ => panic!("expected Recovered, got {:?}", result),
        }
    }

    #[test]
    fn parse_empty_file() {
        let parser = RustParser;
        let file = rust_file("");
        let source = Arc::from("");
        let result = parser.parse(source, &file);
        assert!(matches!(result, ParseOutcome::Success(_)));
    }

    #[test]
    fn parse_whitespace_only() {
        let parser = RustParser;
        let file = rust_file("  \n  ");
        let source = Arc::from("  \n  ");
        let result = parser.parse(source, &file);
        assert!(matches!(result, ParseOutcome::Success(_)));
    }

    #[test]
    fn can_parse_rust_file() {
        let parser = RustParser;
        assert_eq!(parser.language(), Language::Rust);
    }

    #[test]
    fn cannot_parse_non_rust_file() {
        let parser = RustParser;
        assert_eq!(parser.language(), Language::Rust);
    }
}
