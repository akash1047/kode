//! Tree-sitter Python parser.

use std::sync::Arc;

use kode_acquisition::{Language, RepositoryFile};

use crate::parsing::diagnostic::{Diagnostic, Severity};
use crate::parsing::parser::Parser;
use crate::parsing::result::ParseOutcome;
use crate::parsing::syntax::SyntaxTree;
use crate::parsing::ts;

/// Parser for Python source files using the Tree-sitter Python grammar.
#[derive(Debug)]
pub struct PythonParser;

impl Parser for PythonParser {
    fn language(&self) -> Language {
        Language::Python
    }

    fn parse(&self, source: Arc<str>, file: &RepositoryFile) -> ParseOutcome {
        match ts::parse_source(Language::Python, &source) {
            Ok((backend, diagnostics)) => {
                let has_errors = diagnostics.iter().any(|d| *d.severity() == Severity::Error);
                let syntax_tree = SyntaxTree::new(
                    file.relative_path().to_path_buf(),
                    Language::Python,
                    source,
                    has_errors,
                    diagnostics,
                    "tree-sitter-python".to_string(),
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
