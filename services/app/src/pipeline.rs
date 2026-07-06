//! Scan pipeline — the stable application entry point.
//!
//! # Ownership
//!
//! - [`run_scan`] owns the full scan lifecycle: repository discovery, source
//!   loading, parsing, and statistics collection.
//! - Callers receive a [`ScanResult`] containing a [`RepositorySnapshot`] and
//!   [`ScanStatistics`]. Internal artifacts (syntax trees, source buffers) are
//!   consumed and dropped before the function returns.
//!
//! # Lifetime
//!
//! A [`ScanResult`] is an immediate-value DTO. It is not attached to any
//! long-lived service or connection and can be safely sent across threads or
//! serialised for caching.
//!
//! # Stability
//!
//! This module is the **stable application API**. The types and functions
//! exported here are guaranteed to remain backwards-compatible within a major
//! version. Internal details (e.g. [`kode_analysis`] types) are not exposed.
//!
//! # Intended consumers
//!
//! - **CLI** (`tools/cli`) — the primary consumer, mapping results to
//!   presentation models.
//! - **MCP server** — will consume scan results to answer queries.
//! - **Third-party integrations** — may consume [`ScanResult`] via the public
//!   crate interface.

use std::time::{Duration, Instant};

use kode_acquisition::{Repository, RepositoryDiscovery, RepositorySnapshot};
use kode_analysis::parsing::{
    ParseOutcome, ParserRegistry, ParsingOrchestrator, SourceInventory, SyntaxTreeInventory,
};

/// Statistics collected during a scan pipeline run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanStatistics {
    pub elapsed: Duration,
    pub files_discovered: usize,
    pub directories: usize,
    pub manifests: usize,
    pub languages: usize,
    pub parsed: usize,
    pub recovered: usize,
    pub skipped: usize,
    pub failed: usize,
}

/// Public result of a scan pipeline run.
///
/// Contains only the fields stable across backends. Internal artifacts
/// such as [`SourceInventory`] and [`SyntaxTreeInventory`] are not exposed
/// here; they are consumed during statistics computation and dropped.
pub struct ScanResult {
    pub snapshot: RepositorySnapshot,
    pub statistics: ScanStatistics,
}

/// Run the full scan pipeline: discovery, source loading, and parsing.
pub fn run_scan(path: Option<&str>) -> Result<ScanResult, Box<dyn std::error::Error>> {
    let start = Instant::now();

    let repo_path = path.unwrap_or(".");
    let repository = Repository::new(repo_path)?;

    let snapshot = RepositoryDiscovery::default().run(&repository)?;

    let sources = SourceInventory::from_snapshot(&snapshot)?;

    let parser_orchestrator = ParsingOrchestrator::new(ParserRegistry::default());
    let tree_inventory = parser_orchestrator.run(&snapshot, &sources);

    let elapsed = start.elapsed();
    let statistics = ScanStatistics::compute(&snapshot, &tree_inventory, elapsed);

    Ok(ScanResult {
        snapshot,
        statistics,
    })
}

impl ScanStatistics {
    pub fn compute(
        snapshot: &RepositorySnapshot,
        inventory: &SyntaxTreeInventory,
        elapsed: Duration,
    ) -> Self {
        let mut parsed = 0;
        let mut recovered = 0;
        let mut skipped = 0;
        let mut failed = 0;

        for outcome in inventory.iter() {
            match outcome.kind() {
                ParseOutcome::Success(_) => parsed += 1,
                ParseOutcome::Recovered(_) => recovered += 1,
                ParseOutcome::Skipped(_) => skipped += 1,
                ParseOutcome::Failed(_) => failed += 1,
            }
        }

        Self {
            elapsed,
            files_discovered: snapshot.files().len(),
            directories: snapshot.directories().len(),
            manifests: snapshot.manifests().len(),
            languages: snapshot.languages().len(),
            parsed,
            recovered,
            skipped,
            failed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use kode_acquisition::{
        DirectoryInventory, FileInventory, FileMetadata, Language, ManifestInventory,
        RepositoryFile, SnapshotBuilder, Workspace,
    };
    use kode_analysis::parsing::{Diagnostic, FileParseOutcome, SkipReason};

    fn dummy_repo() -> Repository {
        Repository::new(".").unwrap()
    }

    #[test]
    fn statistics_empty_repository() {
        let repo = dummy_repo();
        let snapshot = SnapshotBuilder::new()
            .repository(repo)
            .workspace(Workspace::none())
            .file_inventory(FileInventory::new(Vec::new()))
            .directory_inventory(DirectoryInventory::new(Vec::new()))
            .manifest_inventory(ManifestInventory::new())
            .build()
            .unwrap();

        let inventory = SyntaxTreeInventory::new(Vec::new());
        let stats = ScanStatistics::compute(&snapshot, &inventory, Duration::from_secs(0));

        assert_eq!(stats.files_discovered, 0);
        assert_eq!(stats.directories, 0);
        assert_eq!(stats.manifests, 0);
        assert_eq!(stats.languages, 0);
        assert_eq!(stats.parsed, 0);
        assert_eq!(stats.recovered, 0);
        assert_eq!(stats.skipped, 0);
        assert_eq!(stats.failed, 0);
    }

    #[test]
    fn statistics_count_skipped_and_failed() {
        let repo = dummy_repo();
        let file_inventory = FileInventory::new(vec![
            RepositoryFile::new(
                PathBuf::from("skipped.py"),
                FileMetadata::new(10, None),
                Some(Language::Python),
            ),
            RepositoryFile::new(
                PathBuf::from("failed.rs"),
                FileMetadata::new(10, None),
                Some(Language::Rust),
            ),
        ]);
        let snapshot = SnapshotBuilder::new()
            .repository(repo)
            .workspace(Workspace::none())
            .file_inventory(file_inventory)
            .directory_inventory(DirectoryInventory::new(Vec::new()))
            .manifest_inventory(ManifestInventory::new())
            .build()
            .unwrap();

        let outcomes = vec![
            FileParseOutcome::new(
                PathBuf::from("skipped.py"),
                Some(Language::Python),
                ParseOutcome::Skipped(SkipReason::UnsupportedLanguage),
            ),
            FileParseOutcome::new(
                PathBuf::from("failed.rs"),
                Some(Language::Rust),
                ParseOutcome::Failed(vec![Diagnostic::error("parse error", 0, 0, 1, 1)]),
            ),
        ];
        let inventory = SyntaxTreeInventory::new(outcomes);
        let stats = ScanStatistics::compute(&snapshot, &inventory, Duration::from_secs(1));

        assert_eq!(stats.files_discovered, 2);
        assert_eq!(stats.parsed, 0);
        assert_eq!(stats.recovered, 0);
        assert_eq!(stats.skipped, 1);
        assert_eq!(stats.failed, 1);
    }
}
