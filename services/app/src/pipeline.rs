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

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

use kode_acquisition::{Repository, RepositoryDiscovery, RepositorySnapshot};
use kode_analysis::extraction::{
    ExtractionOrchestrator, ExtractorRegistry, FactsCache, RepositoryFacts,
};
use kode_analysis::parsing::{
    ParseOutcome, ParserRegistry, ParsingOrchestrator, SourceInventory, SyntaxTreeInventory,
};
use kode_common::hash::Fnv1aHasher;
use kode_graph::{GraphBuilder, GraphVersion, KnowledgeGraph, RepositoryContext};
use kode_storage::{GraphRevision, RepositoryMetadata, RepositoryStorage, SqliteBackend};
use std::hash::{Hash, Hasher};

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
    /// True when stages 2–5 were skipped because the content fingerprint matched.
    pub cache_hit: bool,
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
/// When the repository content fingerprint matches the last cached revision,
/// stages 2–5 are skipped and the stored graph is returned (`cache_hit`).
/// Use `kode scan --full` (clears cache) to force a rebuild.
///
/// The pipeline stores its cache in `<repository-root>/.kode/cache.db`.
pub fn run_scan(path: Option<&str>) -> Result<ScanResult, Box<dyn std::error::Error>> {
    let start = Instant::now();

    let repo_path = path.unwrap_or(".");
    let repository = Repository::new(repo_path)?;

    // ── Stage 1: Discovery ──────────────────────────────────────────────

    tracing::info!("Stage 1: Discovery — discovering repository sources");
    let snapshot = RepositoryDiscovery::default().run(&repository)?;
    let file_hashes = compute_file_content_hashes(&repository, &snapshot)?;
    let fingerprint = fingerprint_from_hashes(&file_hashes);

    // ── Incremental fast path ───────────────────────────────────────────
    if let Some((graph, revision)) = try_cache_hit(&repository, &fingerprint) {
        tracing::info!(
            "Incremental skip — fingerprint match, reusing revision {}",
            revision.revision_id
        );
        let elapsed = start.elapsed();
        let entity_count = graph
            .nodes()
            .iter()
            .filter(|n| {
                !matches!(
                    n.kind(),
                    kode_graph::NodeKind::Repository
                        | kode_graph::NodeKind::Workspace
                        | kode_graph::NodeKind::File
                )
            })
            .count();
        let statistics = ScanStatistics {
            elapsed,
            files_discovered: snapshot.files().len(),
            directories: snapshot.directories().len(),
            manifests: snapshot.manifests().len(),
            languages: snapshot.languages().len(),
            parsed: 0,
            recovered: 0,
            skipped: snapshot.files().len(),
            failed: 0,
            entities_extracted: entity_count,
            graph_nodes: graph.node_count(),
            graph_relationships: graph.relationship_count(),
            storage_revision: Some(revision.revision_id),
            storage_path: Some(storage_path_str(&repository)),
            cache_hit: true,
        };
        return Ok(ScanResult {
            snapshot,
            statistics,
            graph: Some(graph),
            revision: Some(revision),
        });
    }

    // ── Stages 2–3: Parse + extract (full or per-file incremental) ─────

    let (facts, parse_stats) = extract_facts_incremental(&repository, &snapshot, &file_hashes)?;

    let entity_count = facts.entity_count();
    tracing::info!(
        "Stage 3: Fact extraction — extracted {} entities (parsed {} files)",
        entity_count,
        parse_stats.parsed
    );

    // ── Stage 4: Knowledge Graph Construction ───────────────────────────

    let ctx = RepositoryContext::from_facts(&facts);
    let (graph, graph_nodes, graph_relationships) = match GraphBuilder::build(&facts, &ctx) {
        Ok(g) => {
            let n = g.node_count();
            let r = g.relationship_count();
            (Some(g), n, r)
        }
        Err(errors) => {
            tracing::warn!(
                "Graph construction encountered {} validation error(s); proceeding without graph",
                errors.len()
            );
            (None, 0, 0)
        }
    };

    tracing::info!(
        "Stage 4: Knowledge graph — {} nodes, {} relationships",
        graph_nodes,
        graph_relationships,
    );

    // ── Stage 5: Storage Persistence ────────────────────────────────────

    let (revision, storage_revision, storage_path) = if let Some(ref graph) = graph {
        match persist_graph_and_facts(&repository, graph, &fingerprint, &file_hashes, &facts) {
            Ok(rev) => (
                Some(rev.clone()),
                Some(rev.revision_id),
                Some(storage_path_str(&repository)),
            ),
            Err(e) => {
                tracing::error!(error = %e, "Storage persistence failed; continuing without cache");
                (None, None, None)
            }
        }
    } else {
        (None, None, None)
    };

    if let Some(rev) = storage_revision {
        tracing::info!("Stage 5: Storage persistence complete — revision {}", rev);
    }

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
        cache_hit: false,
    };

    Ok(ScanResult {
        snapshot,
        statistics,
        graph,
        revision,
    })
}

/// Parse/extract only changed files when a prior facts cache exists.
fn extract_facts_incremental(
    repository: &Repository,
    snapshot: &RepositorySnapshot,
    file_hashes: &[(String, String)],
) -> Result<(RepositoryFacts, ParseStats), Box<dyn std::error::Error>> {
    let parser_orchestrator = ParsingOrchestrator::new(ParserRegistry::default());
    let extractor_orchestrator = ExtractionOrchestrator::new(ExtractorRegistry::default());

    let new_map: HashMap<String, String> = file_hashes.iter().cloned().collect();
    let (old_facts, old_hashes) = load_facts_and_hashes(repository).unwrap_or((None, Vec::new()));

    let mut changed: HashSet<PathBuf> = HashSet::new();
    let mut deleted: HashSet<PathBuf> = HashSet::new();

    if let Some(ref _old) = old_facts {
        let old_map: HashMap<String, String> = old_hashes.into_iter().collect();
        for (path, hash) in &new_map {
            match old_map.get(path) {
                Some(old_h) if old_h == hash => {}
                _ => {
                    changed.insert(PathBuf::from(path));
                }
            }
        }
        for path in old_map.keys() {
            if !new_map.contains_key(path) {
                deleted.insert(PathBuf::from(path));
            }
        }
    }

    let can_partial = old_facts.is_some() && (!changed.is_empty() || !deleted.is_empty());

    if can_partial {
        tracing::info!(
            "Incremental reparse — {} changed, {} deleted files",
            changed.len(),
            deleted.len()
        );
        let mut drop_set = changed.clone();
        drop_set.extend(deleted.iter().cloned());
        let mut facts = old_facts.expect("checked").without_files(&drop_set);

        if changed.is_empty() {
            // Only deletions: no reparse needed.
            let mut parse_stats = empty_parse_stats(snapshot);
            parse_stats.skipped = snapshot.files().len();
            return Ok((facts, parse_stats));
        }

        let sources = SourceInventory::from_snapshot_paths(snapshot, Some(&changed))?;
        let tree_inventory = parser_orchestrator.run_filtered(snapshot, &sources, Some(&changed));
        let mut parse_stats = collect_parse_stats(snapshot, &tree_inventory);
        // Files not in the changed set were skipped intentionally.
        parse_stats.skipped = snapshot
            .files()
            .len()
            .saturating_sub(parse_stats.parsed + parse_stats.recovered + parse_stats.failed);
        let new_facts = extractor_orchestrator.run(&tree_inventory);
        facts.merge(new_facts);
        return Ok((facts, parse_stats));
    }

    // Full parse + extract.
    let sources = SourceInventory::from_snapshot(snapshot)?;
    let tree_inventory = parser_orchestrator.run(snapshot, &sources);
    let parse_stats = collect_parse_stats(snapshot, &tree_inventory);
    tracing::info!(
        "Stage 2: Parsing — parsed {} files, {} failed, {} skipped",
        parse_stats.parsed,
        parse_stats.failed,
        parse_stats.skipped,
    );
    let facts = extractor_orchestrator.run(&tree_inventory);
    Ok((facts, parse_stats))
}

fn empty_parse_stats(snapshot: &RepositorySnapshot) -> ParseStats {
    ParseStats {
        files_discovered: snapshot.files().len(),
        directories: snapshot.directories().len(),
        manifests: snapshot.manifests().len(),
        languages: snapshot.languages().len(),
        parsed: 0,
        recovered: 0,
        skipped: 0,
        failed: 0,
    }
}

/// Content hash of each file (relative path → FNV hex of bytes).
fn compute_file_content_hashes(
    repository: &Repository,
    snapshot: &RepositorySnapshot,
) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
    let root = repository.root();
    let mut out = Vec::with_capacity(snapshot.files().len());
    for file in snapshot.files() {
        let full = root.join(file.relative_path());
        let bytes = std::fs::read(&full).unwrap_or_default();
        let mut hasher = Fnv1aHasher::new();
        bytes.hash(&mut hasher);
        out.push((
            file.relative_path().display().to_string(),
            format!("{:016x}", hasher.finish()),
        ));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

type FileHashList = Vec<(String, String)>;

fn load_facts_and_hashes(
    repository: &Repository,
) -> Option<(Option<RepositoryFacts>, FileHashList)> {
    let db_path = cache_db_path(repository);
    if !db_path.exists() {
        return None;
    }
    let backend = SqliteBackend::open(&db_path).ok()?;
    let repo_id = repository
        .identity()
        .map(|id| id.as_str().to_string())
        .unwrap_or_else(|| repository.root().display().to_string());
    let storage = RepositoryStorage::open(Box::new(backend), &repo_id).ok()?;
    let hashes = storage.load_file_hashes().ok().unwrap_or_default();
    let facts = storage
        .load_facts_json()
        .ok()
        .flatten()
        .and_then(|json| FactsCache::from_json(&json).ok())
        .and_then(|c| c.to_facts().ok());
    Some((facts, hashes))
}

/// Whole-repo fingerprint from sorted per-file content hashes.
fn fingerprint_from_hashes(file_hashes: &[(String, String)]) -> String {
    let mut hasher = Fnv1aHasher::new();
    for (path, hash) in file_hashes {
        path.hash(&mut hasher);
        hash.hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

/// If storage has a matching fingerprint, load the latest graph and skip rebuild.
fn try_cache_hit(
    repository: &Repository,
    fingerprint: &str,
) -> Option<(KnowledgeGraph, GraphRevision)> {
    let db_path = cache_db_path(repository);
    if !db_path.exists() {
        return None;
    }
    let backend = SqliteBackend::open(&db_path).ok()?;
    let repo_id = repository
        .identity()
        .map(|id| id.as_str().to_string())
        .unwrap_or_else(|| repository.root().display().to_string());
    let storage = RepositoryStorage::open(Box::new(backend), &repo_id).ok()?;
    let stored = storage.fingerprint().ok().flatten()?;
    if stored != fingerprint {
        tracing::debug!(
            stored = %stored,
            current = %fingerprint,
            "fingerprint mismatch — full rescan"
        );
        return None;
    }
    storage.load_latest().ok()
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

/// Persist graph, file hashes, and facts cache.
fn persist_graph_and_facts(
    repository: &Repository,
    graph: &KnowledgeGraph,
    fingerprint: &str,
    file_hashes: &[(String, String)],
    facts: &RepositoryFacts,
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

    let root = repository.root().display().to_string();
    let metadata = RepositoryMetadata {
        repository_id: repo_id,
        root,
        fingerprint: fingerprint.to_string(),
        parser_versions: vec!["tree-sitter-rust".into(), "tree-sitter-python".into()],
        last_updated: SystemTime::now(),
    };

    let revision = storage.persist(graph, &metadata, GraphVersion::CURRENT)?;
    storage.save_file_hashes(file_hashes)?;
    if let Ok(json) = FactsCache::from_facts(facts).to_json() {
        if let Err(e) = storage.save_facts_json(&json) {
            tracing::warn!(error = %e, "failed to persist facts cache");
        }
    }
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
            cache_hit: false,
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
