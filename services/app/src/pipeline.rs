//! Scan pipeline — the stable application entry point.
//!
//! # Ownership
//!
//! - [`run_scan`] owns the full scan lifecycle: repository discovery, source
//!   loading, parsing, fact extraction, knowledge graph construction, and
//!   storage persistence (Stages 1-5).
//! - Callers receive a [`ScanResult`] containing a [`RepositorySnapshot`],
//!   [`ScanStatistics`], and optionally the built [`KnowledgeGraph`] and
//!   [`GraphRevision`].
//! - Internal artifacts (syntax trees, source buffers, extracted facts) are
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

use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

use kode_acquisition::{Repository, RepositoryDiscovery, RepositorySnapshot};
use kode_analysis::extraction::{ExtractionOrchestrator, ExtractorRegistry, RepositoryFacts};
use kode_analysis::parsing::{
    ParseOutcome, ParserRegistry, ParsingOrchestrator, SourceInventory, SyntaxTreeInventory,
};
use kode_graph::{GraphBuilder, GraphVersion, KnowledgeGraph, RepositoryContext};
use kode_storage::{GraphRevision, RepositoryMetadata, RepositoryStorage, SqliteBackend};

/// Default cache directory name (relative to repository root).
const CACHE_DIR: &str = ".kode";
/// Default database file name within the cache directory.
const DB_FILE: &str = "cache.db";

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
    /// Entities extracted during Stage 3.
    pub entities_extracted: usize,
    /// Nodes in the constructed knowledge graph (Stage 4).
    pub graph_nodes: usize,
    /// Relationships in the constructed knowledge graph (Stage 4).
    pub graph_relationships: usize,
    /// Storage revision ID if persistence succeeded (Stage 5).
    pub storage_revision: Option<u64>,
    /// Path to the storage database file.
    pub storage_path: Option<String>,
}

/// Public result of a scan pipeline run.
///
/// Contains only the fields stable across backends. Internal artifacts
/// such as [`SourceInventory`] and [`SyntaxTreeInventory`] are not exposed
/// here; they are consumed during statistics computation and dropped.
pub struct ScanResult {
    pub snapshot: RepositorySnapshot,
    pub statistics: ScanStatistics,
    /// The built knowledge graph, if Stage 4 completed.
    pub graph: Option<KnowledgeGraph>,
    /// The persisted graph revision, if Stage 5 completed.
    pub revision: Option<GraphRevision>,
}

/// Run the full scan pipeline: discovery, source loading, parsing, fact
/// extraction, knowledge graph construction, and storage persistence.
///
/// The pipeline stores its cache in `<repository-root>/.kode/cache.db`.
pub fn run_scan(path: Option<&str>) -> Result<ScanResult, Box<dyn std::error::Error>> {
    let start = Instant::now();

    let repo_path = path.unwrap_or(".");
    let repository = Repository::new(repo_path)?;

    // ── Stage 1: Discovery ──────────────────────────────────────────────

    let snapshot = RepositoryDiscovery::default().run(&repository)?;

    // ── Stage 2: Parsing ────────────────────────────────────────────────

    let sources = SourceInventory::from_snapshot(&snapshot)?;

    let parser_orchestrator = ParsingOrchestrator::new(ParserRegistry::default());
    let tree_inventory = parser_orchestrator.run(&snapshot, &sources);

    let parse_stats = collect_parse_stats(&snapshot, &tree_inventory);

    // ── Stage 3: Fact Extraction ────────────────────────────────────────

    let extractor_orchestrator = ExtractionOrchestrator::new(ExtractorRegistry::default());
    let facts: RepositoryFacts = extractor_orchestrator.run(&tree_inventory);
    let entity_count = facts.entity_count();

    // ── Stage 4: Knowledge Graph Construction ───────────────────────────

    let ctx = RepositoryContext::from_facts(&facts);
    let (graph, graph_nodes, graph_relationships) = match GraphBuilder::build(&facts, &ctx) {
        Ok(g) => {
            let n = g.node_count();
            let r = g.relationship_count();
            (Some(g), n, r)
        }
        Err(errors) => {
            eprintln!(
                "Graph construction encountered {} validation error(s); proceeding without graph",
                errors.len()
            );
            (None, 0, 0)
        }
    };

    // ── Stage 5: Storage Persistence ────────────────────────────────────

    let (revision, storage_revision, storage_path) = if let Some(ref graph) = graph {
        match persist_graph(&repository, graph) {
            Ok(rev) => (
                Some(rev.clone()),
                Some(rev.revision_id),
                Some(storage_path_str(&repository)),
            ),
            Err(e) => {
                eprintln!("Storage persistence failed: {e}; continuing without cache");
                (None, None, None)
            }
        }
    } else {
        (None, None, None)
    };

    let elapsed = start.elapsed();

    let statistics = ScanStatistics {
        elapsed,
        files_discovered: parse_stats.files_discovered,
        directories: parse_stats.directories,
        manifests: parse_stats.manifests,
        languages: parse_stats.languages,
        parsed: parse_stats.parsed,
        recovered: parse_stats.recovered,
        skipped: parse_stats.skipped,
        failed: parse_stats.failed,
        entities_extracted: entity_count,
        graph_nodes,
        graph_relationships,
        storage_revision,
        storage_path,
    };

    Ok(ScanResult {
        snapshot,
        statistics,
        graph,
        revision,
    })
}

/// Data from parsing statistics collection.
struct ParseStats {
    files_discovered: usize,
    directories: usize,
    manifests: usize,
    languages: usize,
    parsed: usize,
    recovered: usize,
    skipped: usize,
    failed: usize,
}

fn collect_parse_stats(
    snapshot: &RepositorySnapshot,
    inventory: &SyntaxTreeInventory,
) -> ParseStats {
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

    ParseStats {
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

/// Build the cache database path for a repository.
fn cache_db_path(repository: &Repository) -> PathBuf {
    repository.root().join(CACHE_DIR).join(DB_FILE)
}

/// String representation of the storage path (for display).
fn storage_path_str(repository: &Repository) -> String {
    cache_db_path(repository).display().to_string()
}

/// Persist the knowledge graph to the local cache.
fn persist_graph(
    repository: &Repository,
    graph: &KnowledgeGraph,
) -> Result<GraphRevision, Box<dyn std::error::Error>> {
    let db_path = cache_db_path(repository);

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let backend = SqliteBackend::open(&db_path)?;
    let repo_id = repository
        .identity()
        .map(|id| id.as_str().to_string())
        .unwrap_or_else(|| repository.root().display().to_string());

    let mut storage = RepositoryStorage::open(Box::new(backend), &repo_id)?;

    // Collect unique source files from fact evidence to build metadata.
    let root = repository.root().display().to_string();
    let metadata = RepositoryMetadata {
        repository_id: repo_id,
        root,
        fingerprint: String::new(),
        parser_versions: vec![],
        last_updated: SystemTime::now(),
    };

    let revision = storage.persist(graph, &metadata, GraphVersion::CURRENT)?;
    Ok(revision)
}

impl ScanStatistics {
    pub fn compute(
        snapshot: &RepositorySnapshot,
        inventory: &SyntaxTreeInventory,
        facts: &RepositoryFacts,
        graph: Option<&KnowledgeGraph>,
        storage_revision: Option<u64>,
        storage_path: Option<String>,
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

        let (graph_nodes, graph_relationships) = graph
            .map(|g| (g.node_count(), g.relationship_count()))
            .unwrap_or((0, 0));

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
            entities_extracted: facts.entity_count(),
            graph_nodes,
            graph_relationships,
            storage_revision,
            storage_path,
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

    fn empty_facts() -> RepositoryFacts {
        RepositoryFacts::from_entities(Vec::new(), Vec::new())
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
        let facts = empty_facts();
        let stats = ScanStatistics::compute(
            &snapshot,
            &inventory,
            &facts,
            None,
            None,
            None,
            Duration::from_secs(0),
        );

        assert_eq!(stats.files_discovered, 0);
        assert_eq!(stats.directories, 0);
        assert_eq!(stats.manifests, 0);
        assert_eq!(stats.languages, 0);
        assert_eq!(stats.parsed, 0);
        assert_eq!(stats.recovered, 0);
        assert_eq!(stats.skipped, 0);
        assert_eq!(stats.failed, 0);
        assert_eq!(stats.entities_extracted, 0);
        assert_eq!(stats.graph_nodes, 0);
        assert_eq!(stats.graph_relationships, 0);
        assert_eq!(stats.storage_revision, None);
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
        let facts = empty_facts();
        let stats = ScanStatistics::compute(
            &snapshot,
            &inventory,
            &facts,
            None,
            None,
            None,
            Duration::from_secs(1),
        );

        assert_eq!(stats.files_discovered, 2);
        assert_eq!(stats.parsed, 0);
        assert_eq!(stats.recovered, 0);
        assert_eq!(stats.skipped, 1);
        assert_eq!(stats.failed, 1);
        assert_eq!(stats.entities_extracted, 0);
    }
}
