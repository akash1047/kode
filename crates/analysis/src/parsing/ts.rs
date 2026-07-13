use std::sync::Arc;

use kode_acquisition::Language;

use crate::parsing::diagnostic::Diagnostic;
use crate::parsing::error::Error;

/// Opaque container for the Tree-sitter syntax tree.
///
/// Encapsulates the Tree-sitter backend so that downstream consumers
/// never depend on parser implementation details. The tree is preserved
/// for symbol extraction but is otherwise opaque.
///
/// # Ownership
///
/// The underlying [`tree_sitter::Tree`] is reference-counted to allow
/// sharing across multiple consumers without cloning the AST.
#[derive(Debug, Clone)]
pub(crate) struct SyntaxBackend {
    #[allow(dead_code)]
    pub(crate) tree: Arc<tree_sitter::Tree>,
}

impl SyntaxBackend {
    pub(crate) fn new(tree: tree_sitter::Tree) -> Self {
        Self {
            tree: Arc::new(tree),
        }
    }
}

/// Parse source text using Tree-sitter for the given language.
///
/// Returns the syntax backend and any diagnostics. Diagnostics include
/// both error nodes (unexpected syntax) and missing nodes (expected tokens).
///
/// # Errors
///
/// - Returns [`Error::ParserInit`] if the language grammar cannot be loaded.
/// - Returns [`Error::UnsupportedLanguage`] if no grammar is registered.
/// - Returns [`Error::Internal`] if Tree-sitter produces no tree.
pub(crate) fn parse_source(
    language: Language,
    source: &str,
) -> Result<(SyntaxBackend, Vec<Diagnostic>), Error> {
    let mut parser = tree_sitter::Parser::new();
    let ts_language = resolve_language(language.clone())?;
    parser
        .set_language(&ts_language)
        .map_err(|e| Error::ParserInit {
            language: language.to_string(),
            detail: e.to_string(),
        })?;

    let tree = parser
        .parse(source.as_bytes(), None)
        .ok_or_else(|| Error::Internal("tree-sitter parser returned no tree".to_string()))?;

    let diagnostics = extract_diagnostics(&tree, source);

    Ok((SyntaxBackend::new(tree), diagnostics))
}

fn resolve_language(language: Language) -> Result<tree_sitter::Language, Error> {
    match language {
        Language::Rust => Ok(tree_sitter_rust::LANGUAGE.into()),
        Language::Python => Ok(tree_sitter_python::LANGUAGE.into()),
        _ => Err(Error::UnsupportedLanguage(language.to_string())),
    }
}

fn extract_diagnostics(tree: &tree_sitter::Tree, source: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    collect_errors(tree.root_node(), source, &mut diagnostics);
    diagnostics
}

fn collect_errors(node: tree_sitter::Node, source: &str, diagnostics: &mut Vec<Diagnostic>) {
    if node.is_error() {
        let start = node.start_position();
        let end = node.end_position();
        let text = node.utf8_text(source.as_bytes()).unwrap_or("");
        diagnostics.push(Diagnostic::error(
            format!("unexpected syntax: `{text}`"),
            start.row + 1,
            start.column + 1,
            end.row + 1,
            end.column + 1,
        ));
    }
    if node.is_missing() {
        let start = node.start_position();
        let end = node.end_position();
        diagnostics.push(Diagnostic::error(
            format!("expected {}", node.kind()),
            start.row + 1,
            start.column + 1,
            end.row + 1,
            end.column + 1,
        ));
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_errors(child, source, diagnostics);
    }
}
