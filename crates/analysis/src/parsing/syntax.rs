use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use kode_acquisition::Language;

use crate::parsing::diagnostic::Diagnostic;
use crate::parsing::result::ParseOutcome;
use crate::parsing::ts::SyntaxBackend;

/// Immutable result of parsing a single source file.
///
/// Carries the parsed source text, diagnostics, parser metadata, and the
/// underlying Tree-sitter syntax tree for downstream analysis.
///
/// # Invariants
///
/// - Source text is shared via [`Arc<str>`] to avoid duplication when the
///   same source is referenced by multiple consumers.
/// - The backend syntax tree is preserved for symbol extraction but is
///   otherwise opaque to consumers outside the parsing subsystem.
/// - `has_errors` is derived from diagnostics: `true` iff any diagnostic
///   has [`Severity::Error`].
///
/// [`Severity::Error`]: crate::parsing::Severity
#[derive(Debug, Clone)]
pub struct SyntaxTree {
    relative_path: PathBuf,
    language: Language,
    source: Arc<str>,
    has_errors: bool,
    diagnostics: Vec<Diagnostic>,
    parser_name: String,
    parser_version: Option<String>,
    grammar_version: Option<String>,
    backend_id: Option<String>,
    #[allow(dead_code)]
    backend: SyntaxBackend,
}

impl SyntaxTree {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        relative_path: PathBuf,
        language: Language,
        source: Arc<str>,
        has_errors: bool,
        diagnostics: Vec<Diagnostic>,
        parser_name: String,
        parser_version: Option<String>,
        grammar_version: Option<String>,
        backend_id: Option<String>,
        backend: SyntaxBackend,
    ) -> Self {
        Self {
            relative_path,
            language,
            source,
            has_errors,
            diagnostics,
            parser_name,
            parser_version,
            grammar_version,
            backend_id,
            backend,
        }
    }

    pub fn relative_path(&self) -> &Path {
        &self.relative_path
    }

    pub fn language(&self) -> &Language {
        &self.language
    }

    pub fn has_errors(&self) -> bool {
        self.has_errors
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn parser_name(&self) -> &str {
        &self.parser_name
    }

    pub fn parser_version(&self) -> Option<&str> {
        self.parser_version.as_deref()
    }

    pub fn grammar_version(&self) -> Option<&str> {
        self.grammar_version.as_deref()
    }

    pub fn backend_id(&self) -> Option<&str> {
        self.backend_id.as_deref()
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    #[allow(dead_code)]
    pub(crate) fn backend(&self) -> &SyntaxBackend {
        &self.backend
    }
}

/// Outcome of parsing a single file, bundling path, language, and result.
///
/// Every file in the inventory produces exactly one `FileParseOutcome`,
/// regardless of whether parsing succeeded, recovered, was skipped, or failed.
///
/// # Invariants
///
/// - The `language` field may be `None` when it could not be determined.
/// - The `kind` field discriminates the four possible parse outcomes.
#[derive(Debug, Clone)]
pub struct FileParseOutcome {
    relative_path: PathBuf,
    language: Option<Language>,
    kind: ParseOutcome,
}

impl FileParseOutcome {
    pub fn new(relative_path: PathBuf, language: Option<Language>, kind: ParseOutcome) -> Self {
        Self {
            relative_path,
            language,
            kind,
        }
    }

    pub fn relative_path(&self) -> &Path {
        &self.relative_path
    }

    pub fn language(&self) -> Option<&Language> {
        self.language.as_ref()
    }

    pub fn kind(&self) -> &ParseOutcome {
        &self.kind
    }
}

/// Immutable repository-wide collection of parse outcomes.
///
/// Every [`RepositoryFile`](kode_acquisition::RepositoryFile) produces exactly
/// one [`FileParseOutcome`], even when parsing is skipped or fails.
///
/// # Invariants
///
/// - Outcomes are stored in insertion order (matching snapshot file order).
/// - An index provides O(1) path-based lookup.
/// - Iteration order is deterministic (insertion order).
#[derive(Debug, Clone)]
pub struct SyntaxTreeInventory {
    outcomes: Vec<FileParseOutcome>,
    index: HashMap<PathBuf, usize>,
}

impl SyntaxTreeInventory {
    pub fn new(outcomes: Vec<FileParseOutcome>) -> Self {
        let index = outcomes
            .iter()
            .enumerate()
            .map(|(i, o)| (o.relative_path.clone(), i))
            .collect();
        Self { outcomes, index }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, FileParseOutcome> {
        self.outcomes.iter()
    }

    pub fn len(&self) -> usize {
        self.outcomes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.outcomes.is_empty()
    }

    pub fn get(&self, path: &Path) -> Option<&FileParseOutcome> {
        self.index.get(path).and_then(|&i| self.outcomes.get(i))
    }

    pub fn as_slice(&self) -> &[FileParseOutcome] {
        &self.outcomes
    }

    pub fn parsed_trees(&self) -> impl Iterator<Item = &SyntaxTree> {
        self.outcomes.iter().filter_map(|o| match &o.kind {
            ParseOutcome::Success(tree) | ParseOutcome::Recovered(tree) => Some(tree),
            _ => None,
        })
    }
}

impl IntoIterator for SyntaxTreeInventory {
    type Item = FileParseOutcome;
    type IntoIter = std::vec::IntoIter<FileParseOutcome>;

    fn into_iter(self) -> Self::IntoIter {
        self.outcomes.into_iter()
    }
}

impl<'a> IntoIterator for &'a SyntaxTreeInventory {
    type Item = &'a FileParseOutcome;
    type IntoIter = std::slice::Iter<'a, FileParseOutcome>;

    fn into_iter(self) -> Self::IntoIter {
        self.outcomes.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsing::result::SkipReason;
    use crate::parsing::ts;

    fn sample_tree(path: &str) -> SyntaxTree {
        let (backend, diagnostics) = ts::parse_source(Language::Rust, "fn main() {}").unwrap();
        let has_errors = diagnostics
            .iter()
            .any(|d| *d.severity() == crate::parsing::diagnostic::Severity::Error);
        SyntaxTree::new(
            PathBuf::from(path),
            Language::Rust,
            Arc::from("fn main() {}"),
            has_errors,
            diagnostics,
            "tree-sitter-rust".to_string(),
            None,
            None,
            Some("tree-sitter".to_string()),
            backend,
        )
    }

    #[test]
    fn syntax_tree_accessors() {
        let st = sample_tree("src/main.rs");
        assert_eq!(st.relative_path(), Path::new("src/main.rs"));
        assert_eq!(st.language(), &Language::Rust);
        assert!(!st.has_errors());
        assert!(st.diagnostics().is_empty());
        assert_eq!(st.parser_name(), "tree-sitter-rust");
        assert!(st.parser_version().is_none());
        assert!(st.grammar_version().is_none());
        assert_eq!(st.backend_id(), Some("tree-sitter"));
    }

    #[test]
    fn syntax_tree_detects_errors() {
        let (backend, diagnostics) = ts::parse_source(Language::Rust, "fn main() { ").unwrap();
        let has_errors = diagnostics
            .iter()
            .any(|d| *d.severity() == crate::parsing::diagnostic::Severity::Error);
        let st = SyntaxTree::new(
            PathBuf::from("broken.rs"),
            Language::Rust,
            Arc::from("fn main() { "),
            has_errors,
            diagnostics,
            "tree-sitter-rust".to_string(),
            None,
            None,
            Some("tree-sitter".to_string()),
            backend,
        );
        assert!(st.has_errors());
    }

    #[test]
    fn inventory_deterministic_ordering() {
        let outcomes = vec![
            FileParseOutcome::new(
                PathBuf::from("b.rs"),
                Some(Language::Rust),
                ParseOutcome::Success(sample_tree("b.rs")),
            ),
            FileParseOutcome::new(
                PathBuf::from("a.rs"),
                Some(Language::Rust),
                ParseOutcome::Success(sample_tree("a.rs")),
            ),
        ];
        let inv = SyntaxTreeInventory::new(outcomes);
        assert_eq!(
            inv.get(Path::new("a.rs")).unwrap().relative_path(),
            Path::new("a.rs")
        );
        assert_eq!(inv.len(), 2);
    }

    #[test]
    fn inventory_parsed_trees_only() {
        let outcomes = vec![
            FileParseOutcome::new(
                PathBuf::from("ok.rs"),
                Some(Language::Rust),
                ParseOutcome::Success(sample_tree("ok.rs")),
            ),
            FileParseOutcome::new(
                PathBuf::from("bad.py"),
                Some(Language::Python),
                ParseOutcome::Skipped(SkipReason::UnsupportedLanguage),
            ),
        ];
        let inv = SyntaxTreeInventory::new(outcomes);
        let parsed: Vec<_> = inv.parsed_trees().collect();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].relative_path(), Path::new("ok.rs"));
    }

    #[test]
    fn inventory_lookup() {
        let outcomes = vec![
            FileParseOutcome::new(
                PathBuf::from("lib.rs"),
                Some(Language::Rust),
                ParseOutcome::Success(sample_tree("lib.rs")),
            ),
            FileParseOutcome::new(
                PathBuf::from("main.rs"),
                Some(Language::Rust),
                ParseOutcome::Success(sample_tree("main.rs")),
            ),
        ];
        let inv = SyntaxTreeInventory::new(outcomes);
        assert!(inv.get(Path::new("main.rs")).is_some());
        assert!(inv.get(Path::new("nonexistent.rs")).is_none());
    }

    #[test]
    fn inventory_empty_default() {
        let inv = SyntaxTreeInventory::new(Vec::new());
        assert!(inv.is_empty());
        assert_eq!(inv.len(), 0);
    }

    #[test]
    fn file_parse_outcome_accessors() {
        let outcome = FileParseOutcome::new(
            PathBuf::from("test.rs"),
            Some(Language::Rust),
            ParseOutcome::Skipped(SkipReason::UnsupportedLanguage),
        );
        assert_eq!(outcome.relative_path(), Path::new("test.rs"));
        assert_eq!(outcome.language(), Some(&Language::Rust));
    }
}
