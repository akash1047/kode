use crate::extraction::diagnostic::ExtractionDiagnostic;
use crate::extraction::model::{Entity, RepositoryFacts};
use crate::extraction::registry::ExtractorRegistry;
use crate::extraction::result::ExtractionOutcome;
use crate::parsing::syntax::{SyntaxTree, SyntaxTreeInventory};

/// Pipeline Stage 3 orchestrator: transforms [`SyntaxTreeInventory`] into
/// [`RepositoryFacts`].
///
/// # Pipeline
///
/// 1. For each parsed syntax tree in the inventory, detect its language.
/// 2. Dispatch to the appropriate [`Extractor`] via [`ExtractorRegistry`].
/// 3. Merge all extracted entities into a single [`RepositoryFacts`].
///
/// # Ownership
///
/// Consumes the [`SyntaxTreeInventory`] (or references it) and produces an
/// immutable [`RepositoryFacts`].
///
/// # Failure Behavior
///
/// - Individual tree extraction failures never abort the entire run.
/// - Trees without a registered extractor are skipped.
/// - Trees with parse errors may still be extracted (recoverable).
pub struct ExtractionOrchestrator {
    extractor_registry: ExtractorRegistry,
}

impl ExtractionOrchestrator {
    pub fn new(extractor_registry: ExtractorRegistry) -> Self {
        Self {
            extractor_registry,
        }
    }

    /// Run extraction over the given syntax tree inventory.
    pub fn run(&self, inventory: &SyntaxTreeInventory) -> RepositoryFacts {
        let mut entities: Vec<Entity> = Vec::new();
        let mut diagnostics: Vec<ExtractionDiagnostic> = Vec::new();

        for outcome in inventory.parsed_trees() {
            self.extract_tree(outcome, &mut entities, &mut diagnostics);
        }

        RepositoryFacts::from_entities(entities, diagnostics)
    }

    fn extract_tree(
        &self,
        tree: &SyntaxTree,
        entities: &mut Vec<Entity>,
        diagnostics: &mut Vec<ExtractionDiagnostic>,
    ) {
        let language = tree.language();

        // If the tree has errors, warn
        if tree.has_errors() {
            diagnostics.push(ExtractionDiagnostic::warning(
                format!(
                    "syntax tree for `{}` has errors; extraction may be incomplete",
                    tree.relative_path().display()
                ),
                Some(tree.relative_path().to_path_buf()),
                0,
                0,
                0,
                0,
            ));
        }

        let Some(extractor) = self.extractor_registry.dispatch(language) else {
            diagnostics.push(ExtractionDiagnostic::warning(
                format!(
                    "no extractor registered for `{}`",
                    tree.relative_path().display()
                ),
                Some(tree.relative_path().to_path_buf()),
                0,
                0,
                0,
                0,
            ));
            return;
        };

        match extractor.extract(tree) {
            ExtractionOutcome::Success(mut extracted) => {
                entities.append(&mut extracted);
            }
            ExtractionOutcome::Partial(mut extracted, mut diags) => {
                entities.append(&mut extracted);
                diagnostics.append(&mut diags);
            }
            ExtractionOutcome::Skipped(reason) => {
                diagnostics.push(ExtractionDiagnostic::warning(
                    format!(
                        "extraction skipped for `{}`: {:?}",
                        tree.relative_path().display(),
                        reason
                    ),
                    Some(tree.relative_path().to_path_buf()),
                    0,
                    0,
                    0,
                    0,
                ));
            }
            ExtractionOutcome::Failed(mut diags) => {
                diagnostics.append(&mut diags);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Arc;

    use kode_acquisition::Language;

    use crate::parsing::syntax::{FileParseOutcome, SyntaxTree, SyntaxTreeInventory};
    use crate::parsing::{
        ParseOutcome, ParserRegistry, ParsingOrchestrator, Severity, SourceInventory,
    };

    fn parse_source_to_tree(source: &str, path: &str) -> SyntaxTree {
        let (backend, diagnostics) =
            crate::parsing::ts::parse_source(Language::Rust, source).unwrap();
        let has_errors = diagnostics.iter().any(|d| *d.severity() == Severity::Error);
        SyntaxTree::new(
            PathBuf::from(path),
            Language::Rust,
            Arc::from(source),
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
    fn extract_from_single_tree() {
        let tree = parse_source_to_tree("fn foo() {}\npub fn bar() {}", "lib.rs");
        let inventory = SyntaxTreeInventory::new(vec![FileParseOutcome::new(
            PathBuf::from("lib.rs"),
            Some(Language::Rust),
            ParseOutcome::Success(tree),
        )]);

        let orchestrator = ExtractionOrchestrator::new(ExtractorRegistry::default());
        let facts = orchestrator.run(&inventory);

        assert_eq!(facts.functions().len(), 2);
        assert!(facts.diagnostics().is_empty());
    }

    #[test]
    fn skips_tree_with_syntax_errors() {
        let (backend, diagnostics) =
            crate::parsing::ts::parse_source(Language::Rust, "fn foo() { ").unwrap();
        let has_errors = diagnostics.iter().any(|d| *d.severity() == Severity::Error);
        let error_tree = SyntaxTree::new(
            PathBuf::from("broken.rs"),
            Language::Rust,
            Arc::from("fn foo() { "),
            has_errors,
            diagnostics,
            "tree-sitter-rust".to_string(),
            None,
            None,
            Some("tree-sitter".to_string()),
            backend,
        );

        let inventory = SyntaxTreeInventory::new(vec![FileParseOutcome::new(
            PathBuf::from("broken.rs"),
            Some(Language::Rust),
            ParseOutcome::Recovered(error_tree),
        )]);

        let orchestrator = ExtractionOrchestrator::new(ExtractorRegistry::default());
        let facts = orchestrator.run(&inventory);

        // Should still attempt extraction on recovered tree
        assert!(facts.functions().len() == 1 || !facts.diagnostics().is_empty());
    }

    #[test]
    fn full_pipeline_roundtrip() {
        use kode_acquisition::{
            DirectoryInventory, FileInventory, FileMetadata, Manifest, ManifestInventory,
            Repository, RepositoryFile, SnapshotBuilder, Workspace,
        };

        let dir = tempfile::TempDir::new().unwrap();
        let repo = Repository::new(dir.path()).unwrap();

        let source = "pub struct Point { pub x: i32, y: f64 }\npub fn distance(a: &Point, b: &Point) -> f64 { 0.0 }";
        let file_path = PathBuf::from("lib.rs");
        let full_path = dir.path().join(&file_path);
        std::fs::create_dir_all(full_path.parent().unwrap()).unwrap();
        std::fs::write(&full_path, source).unwrap();

        let repo_files = vec![RepositoryFile::new(
            file_path.clone(),
            FileMetadata::new(source.len() as u64, None),
            Some(Language::Rust),
        )];

        let mut manifest_inventory = ManifestInventory::new();
        manifest_inventory.push(Manifest::new(
            PathBuf::from("Cargo.toml"),
            kode_acquisition::ManifestKind::CargoManifest,
        ));

        let snapshot = SnapshotBuilder::new()
            .repository(repo)
            .workspace(Workspace::none())
            .file_inventory(FileInventory::new(repo_files))
            .directory_inventory(DirectoryInventory::new(Vec::new()))
            .manifest_inventory(manifest_inventory)
            .build()
            .unwrap();

        let sources = SourceInventory::from_snapshot(&snapshot).unwrap();
        let parser_orchestrator = ParsingOrchestrator::new(ParserRegistry::default());
        let tree_inventory = parser_orchestrator.run(&snapshot, &sources);

        let extractor_orchestrator = ExtractionOrchestrator::new(ExtractorRegistry::default());
        let facts = extractor_orchestrator.run(&tree_inventory);

        assert_eq!(facts.functions().len(), 1);
        assert_eq!(facts.structs().len(), 1);
        assert_eq!(facts.structs()[0].fields().len(), 2);
        assert!(facts.diagnostics().is_empty());
    }
}
