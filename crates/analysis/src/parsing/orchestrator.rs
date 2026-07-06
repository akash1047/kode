use std::sync::Arc;

use kode_acquisition::RepositorySnapshot;

use crate::parsing::diagnostic::Diagnostic;
use crate::parsing::registry::ParserRegistry;
use crate::parsing::result::{ParseOutcome, SkipReason};
use crate::parsing::source::SourceInventory;
use crate::parsing::syntax::{FileParseOutcome, SyntaxTreeInventory};

/// Pipeline Stage 2 orchestrator: transforms a [`RepositorySnapshot`] into a [`SyntaxTreeInventory`].
///
/// # Pipeline
///
/// 1. For each file in the snapshot, look up its language.
/// 2. Dispatch to the appropriate [`Parser`] via [`ParserRegistry`].
/// 3. Collect all outcomes into a [`SyntaxTreeInventory`].
///
/// # Ownership
///
/// This orchestrator owns parser dispatch and outcome collection. It consumes
/// immutable inputs and produces an immutable output.
///
/// # Failure Behavior
///
/// - Individual file parse failures never abort the entire run.
/// - Files without a detected language are skipped (not failed).
/// - Files without a registered parser are skipped (not failed).
///
/// [`Parser`]: super::Parser
pub struct ParsingOrchestrator {
    parser_registry: ParserRegistry,
}

impl ParsingOrchestrator {
    pub fn new(parser_registry: ParserRegistry) -> Self {
        Self { parser_registry }
    }

    pub fn run(
        &self,
        snapshot: &RepositorySnapshot,
        sources: &SourceInventory,
    ) -> SyntaxTreeInventory {
        let mut outcomes = Vec::with_capacity(snapshot.files().len());

        for file in snapshot.files() {
            let outcome = self.parse_file(file, sources);
            outcomes.push(outcome);
        }

        SyntaxTreeInventory::new(outcomes)
    }

    fn parse_file(
        &self,
        file: &kode_acquisition::RepositoryFile,
        sources: &SourceInventory,
    ) -> FileParseOutcome {
        let relative_path = file.relative_path().to_path_buf();
        let language = file.language().cloned();

        let Some(lang) = &language else {
            return FileParseOutcome::new(
                relative_path,
                None,
                ParseOutcome::Skipped(SkipReason::UnsupportedLanguage),
            );
        };

        let Some(source_arc) = sources.get(file.relative_path()) else {
            return FileParseOutcome::new(
                relative_path,
                language,
                ParseOutcome::Failed(vec![Diagnostic::error(
                    format!("source not found: {}", file.relative_path().display()),
                    0,
                    0,
                    0,
                    0,
                )]),
            );
        };

        let Some(parser) = self.parser_registry.dispatch(lang) else {
            return FileParseOutcome::new(
                relative_path,
                language,
                ParseOutcome::Skipped(SkipReason::UnsupportedLanguage),
            );
        };

        let outcome = parser.parse(Arc::clone(source_arc), file);
        FileParseOutcome::new(relative_path, language, outcome)
    }
}

/// Parse file content from an in-memory string (bypasses disk I/O).
///
/// Useful for testing and for streams where source text is already in memory.
/// Mirrors the orchestrator's dispatch logic without requiring a full snapshot.
#[allow(dead_code)]
pub(crate) fn parse_file_content(
    source: &str,
    file: &kode_acquisition::RepositoryFile,
    registry: &ParserRegistry,
) -> FileParseOutcome {
    let relative_path = file.relative_path().to_path_buf();
    let language = file.language().cloned();

    let Some(lang) = &language else {
        return FileParseOutcome::new(
            relative_path,
            None,
            ParseOutcome::Skipped(SkipReason::UnsupportedLanguage),
        );
    };

    let Some(parser) = registry.dispatch(lang) else {
        return FileParseOutcome::new(
            relative_path,
            language,
            ParseOutcome::Skipped(SkipReason::UnsupportedLanguage),
        );
    };

    let outcome = parser.parse(Arc::from(source), file);
    FileParseOutcome::new(relative_path, language, outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::path::PathBuf;

    use kode_acquisition::{
        DirectoryInventory, FileInventory, FileMetadata, Language, ManifestInventory, Repository,
        RepositoryFile, SnapshotBuilder, Workspace,
    };

    fn create_snapshot_and_sources(
        files: Vec<(PathBuf, Option<Language>, &str)>,
    ) -> (RepositorySnapshot, SourceInventory, tempfile::TempDir) {
        let dir = tempfile::TempDir::new().unwrap();
        let repo = Repository::new(dir.path()).unwrap();

        let mut repo_files = Vec::new();
        for (path, lang, content) in &files {
            let full_path = dir.path().join(path);
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&full_path, content).unwrap();

            repo_files.push(RepositoryFile::new(
                path.clone(),
                FileMetadata::new(content.len() as u64, None),
                lang.clone(),
            ));
        }

        let snapshot = SnapshotBuilder::new()
            .repository(repo)
            .workspace(Workspace::none())
            .file_inventory(FileInventory::new(repo_files))
            .directory_inventory(DirectoryInventory::new(Vec::new()))
            .manifest_inventory(ManifestInventory::new())
            .build()
            .unwrap();

        let sources = SourceInventory::from_snapshot(&snapshot).unwrap();
        (snapshot, sources, dir)
    }

    #[test]
    fn parses_rust_files() {
        let (snapshot, sources, _dir) = create_snapshot_and_sources(vec![(
            PathBuf::from("main.rs"),
            Some(Language::Rust),
            "fn main() {}",
        )]);

        let orchestrator = ParsingOrchestrator::new(ParserRegistry::default());
        let inventory = orchestrator.run(&snapshot, &sources);

        assert_eq!(inventory.len(), 1);
        let outcome = inventory.get(Path::new("main.rs")).unwrap();
        assert!(matches!(outcome.kind(), ParseOutcome::Success(_)));
    }

    #[test]
    fn skips_unsupported_language() {
        let (snapshot, sources, _dir) = create_snapshot_and_sources(vec![(
            PathBuf::from("main.py"),
            Some(Language::Python),
            "print('hello')",
        )]);

        let orchestrator = ParsingOrchestrator::new(ParserRegistry::default());
        let inventory = orchestrator.run(&snapshot, &sources);

        assert_eq!(inventory.len(), 1);
        let outcome = inventory.get(Path::new("main.py")).unwrap();
        assert!(matches!(
            outcome.kind(),
            ParseOutcome::Skipped(SkipReason::UnsupportedLanguage)
        ));
    }

    #[test]
    fn skips_no_language_files() {
        let (snapshot, sources, _dir) =
            create_snapshot_and_sources(vec![(PathBuf::from("readme.txt"), None, "Hello, World!")]);

        let orchestrator = ParsingOrchestrator::new(ParserRegistry::default());
        let inventory = orchestrator.run(&snapshot, &sources);

        assert_eq!(inventory.len(), 1);
        let outcome = inventory.get(Path::new("readme.txt")).unwrap();
        assert!(matches!(
            outcome.kind(),
            ParseOutcome::Skipped(SkipReason::UnsupportedLanguage)
        ));
    }

    #[test]
    fn mixed_repository() {
        let (snapshot, sources, _dir) = create_snapshot_and_sources(vec![
            (
                PathBuf::from("main.rs"),
                Some(Language::Rust),
                "fn main() {}",
            ),
            (
                PathBuf::from("lib.py"),
                Some(Language::Python),
                "def foo(): pass",
            ),
            (
                PathBuf::from("data.json"),
                Some(Language::Json),
                r#"{"key": "value"}"#,
            ),
        ]);

        let orchestrator = ParsingOrchestrator::new(ParserRegistry::default());
        let inventory = orchestrator.run(&snapshot, &sources);

        assert_eq!(inventory.len(), 3);
        let rs = inventory.get(Path::new("main.rs")).unwrap();
        assert!(matches!(rs.kind(), ParseOutcome::Success(_)));
        let py = inventory.get(Path::new("lib.py")).unwrap();
        assert!(matches!(
            py.kind(),
            ParseOutcome::Skipped(SkipReason::UnsupportedLanguage)
        ));
        let json = inventory.get(Path::new("data.json")).unwrap();
        assert!(matches!(
            json.kind(),
            ParseOutcome::Skipped(SkipReason::UnsupportedLanguage)
        ));
    }

    #[test]
    fn empty_repository_produces_empty_inventory() {
        let (snapshot, sources, _dir) = create_snapshot_and_sources(Vec::new());

        let orchestrator = ParsingOrchestrator::new(ParserRegistry::default());
        let inventory = orchestrator.run(&snapshot, &sources);

        assert!(inventory.is_empty());
    }

    #[test]
    fn individual_file_failure_does_not_abort() {
        let (snapshot, sources, _dir) = create_snapshot_and_sources(vec![
            (
                PathBuf::from("main.rs"),
                Some(Language::Rust),
                "fn main() {}",
            ),
            (
                PathBuf::from("broken.rs"),
                Some(Language::Rust),
                "fn main() { ",
            ),
        ]);

        let orchestrator = ParsingOrchestrator::new(ParserRegistry::default());
        let inventory = orchestrator.run(&snapshot, &sources);

        assert_eq!(inventory.len(), 2);
        let main = inventory.get(Path::new("main.rs")).unwrap();
        assert!(matches!(main.kind(), ParseOutcome::Success(_)));
        let broken = inventory.get(Path::new("broken.rs")).unwrap();
        assert!(matches!(broken.kind(), ParseOutcome::Recovered(_)));
    }

    #[test]
    fn repeated_parsing_produces_identical_output() {
        let (snapshot, sources, _dir) = create_snapshot_and_sources(vec![(
            PathBuf::from("main.rs"),
            Some(Language::Rust),
            "fn main() { let x = 1; }",
        )]);

        let orchestrator = ParsingOrchestrator::new(ParserRegistry::default());
        let inventory1 = orchestrator.run(&snapshot, &sources);
        let inventory2 = orchestrator.run(&snapshot, &sources);

        assert_eq!(inventory1.len(), inventory2.len());
        let trees1: Vec<_> = inventory1.parsed_trees().collect();
        let trees2: Vec<_> = inventory2.parsed_trees().collect();
        assert_eq!(trees1.len(), trees2.len());
        assert_eq!(trees1[0].relative_path(), trees2[0].relative_path());
    }

    #[test]
    fn parse_file_content_in_memory() {
        let registry = ParserRegistry::default();
        let file = RepositoryFile::new(
            Path::new("test.rs"),
            FileMetadata::new(15, None),
            Some(Language::Rust),
        );
        let outcome = parse_file_content("fn main() {}", &file, &registry);
        assert!(matches!(outcome.kind(), ParseOutcome::Success(_)));
    }

    #[test]
    fn parse_file_content_skipped() {
        let registry = ParserRegistry::default();
        let file = RepositoryFile::new(
            Path::new("test.py"),
            FileMetadata::new(15, None),
            Some(Language::Python),
        );
        let outcome = parse_file_content("print('hi')", &file, &registry);
        assert!(matches!(outcome.kind(), ParseOutcome::Skipped(_)));
    }
}
