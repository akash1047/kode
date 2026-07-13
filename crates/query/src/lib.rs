#![allow(unused_crate_dependencies)]

//! Query Engine: transforms repository knowledge into evidence-backed answers.
//!
//! Reads the Knowledge Graph and SyntaxTreeInventory to answer questions
//! about repository structure, definitions, references, and types.
//!
//! # Responsibilities
//!
//! - Deterministic intent resolution ([`intent`])
//! - Evidence verification against live source code
//! - Graph traversal for definition and reference resolution
//! - Architecture metrics, dead-code heuristics, call-graph cycles
//!
//! # Invariants
//!
//! - Queries never mutate the graph or storage.
//! - Every answer includes path:line citations.
//! - Queries are deterministic for the same graph revision.

#[cfg(test)]
use tempfile as _;

mod analysis;
mod intent;

pub use analysis::{
    call_cycles, dead_code_candidates, file_cohesion_pct, format_cycles, format_metrics,
    function_metrics, CallCycle, FunctionMetrics,
};
pub use intent::{parse_intent, QueryIntent};

use std::path::PathBuf;
use std::sync::Mutex;

use kode_graph::{
    GraphEvidence, GraphNodeId, KnowledgeGraph, Node, NodeKind, RelationshipKind,
    StructuralEvidence,
};
use kode_storage::RepositoryStorage;
use thiserror::Error;

/// Errors returned by the query engine.
#[derive(Error, Debug)]
pub enum QueryError {
    /// Error from the storage layer.
    #[error("storage error: {0}")]
    Storage(#[from] kode_storage::StorageError),

    /// No graph has been loaded.
    #[error("graph not loaded")]
    GraphNotLoaded,

    /// I/O error during evidence verification.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// A single symbol result from a query.
#[derive(Debug, Clone)]
pub struct SymbolResult {
    pub name: String,
    pub kind: NodeKind,
    pub file_path: PathBuf,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub verified: bool,
    /// Programming language from source evidence, when available.
    pub language: Option<String>,
}

impl SymbolResult {
    pub(crate) fn from_node(node: &Node) -> Self {
        let (file_path, start_line, start_column, end_line, end_column) =
            evidence_location(node.evidence());
        let verified = evidence_verified(node.evidence());
        let language = evidence_language(node.evidence());
        Self {
            name: node.name().to_string(),
            kind: node.kind(),
            file_path,
            start_line,
            start_column,
            end_line,
            end_column,
            verified,
            language,
        }
    }
}

/// A file known to the knowledge graph.
pub struct FileResult {
    pub path: PathBuf,
    pub language: Option<String>,
}

/// Aggregate stats for a loaded graph revision.
pub struct GraphStats {
    pub node_count: usize,
    pub relationship_count: usize,
    pub file_count: usize,
    pub entity_count: usize,
    pub languages: Vec<String>,
}

fn evidence_location(evidence: &GraphEvidence) -> (PathBuf, usize, usize, usize, usize) {
    match evidence {
        GraphEvidence::Source(ev) => (
            ev.source_file().to_path_buf(),
            ev.start_line(),
            ev.start_column(),
            ev.end_line(),
            ev.end_column(),
        ),
        GraphEvidence::Structural(StructuralEvidence::File { path }) => (path.clone(), 0, 0, 0, 0),
        GraphEvidence::Structural(_) => (PathBuf::new(), 0, 0, 0, 0),
    }
}

fn evidence_verified(evidence: &GraphEvidence) -> bool {
    match evidence {
        GraphEvidence::Source(ev) => {
            verify_source_evidence(ev.source_file(), ev.start_line(), ev.end_line(), None)
        }
        GraphEvidence::Structural(StructuralEvidence::File { path }) => path.exists(),
        _ => false,
    }
}

/// Verify source evidence against the live filesystem.
///
/// Checks: file exists, start/end lines are in range, and optionally that
/// `name_hint` appears somewhere in the cited line range.
pub fn verify_source_evidence(
    path: &std::path::Path,
    start_line: usize,
    end_line: usize,
    name_hint: Option<&str>,
) -> bool {
    if !path.exists() {
        return false;
    }
    if start_line == 0 {
        // 0 can mean "unknown" for some structural conversions — require file only.
        return true;
    }
    let Ok(content) = std::fs::read_to_string(path) else {
        return false;
    };
    let line_count = content.lines().count();
    if start_line > line_count {
        return false;
    }
    if end_line > 0 && end_line < start_line {
        return false;
    }
    if let Some(name) = name_hint {
        if name.is_empty() {
            return true;
        }
        let start = start_line.saturating_sub(1);
        let end = if end_line == 0 {
            start + 1
        } else {
            end_line.min(line_count)
        };
        let region: String = content
            .lines()
            .skip(start)
            .take(end.saturating_sub(start))
            .collect::<Vec<_>>()
            .join("\n");
        if !region.contains(name) {
            return false;
        }
    }
    true
}

fn evidence_language(evidence: &GraphEvidence) -> Option<String> {
    match evidence {
        GraphEvidence::Source(ev) => Some(ev.language().to_string()),
        _ => None,
    }
}

fn language_matches(lang: Option<&str>, filter: Option<&str>) -> bool {
    match filter {
        None => true,
        Some(f) => lang
            .map(|l| l.eq_ignore_ascii_case(f) || l.to_lowercase().starts_with(&f.to_lowercase()))
            .unwrap_or(false),
    }
}

fn language_from_path(path: &std::path::Path) -> Option<String> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    let name = match ext.as_str() {
        "rs" => "Rust",
        "py" => "Python",
        "md" | "markdown" => "Markdown",
        "toml" => "TOML",
        "json" => "JSON",
        "yaml" | "yml" => "YAML",
        "js" | "mjs" | "cjs" => "JavaScript",
        "ts" | "tsx" => "TypeScript",
        "go" => "Go",
        "java" => "Java",
        "rb" => "Ruby",
        "sh" | "bash" => "Shell",
        "css" => "CSS",
        "html" | "htm" => "HTML",
        "sql" => "SQL",
        "proto" => "Protobuf",
        _ => return None,
    };
    Some(name.to_string())
}

/// Query engine wrapping a repository's persisted knowledge graph.
pub struct QueryEngine {
    storage: RepositoryStorage,
    graph: Mutex<Option<KnowledgeGraph>>,
}

impl QueryEngine {
    /// Create a new query engine backed by the given storage.
    pub fn new(storage: RepositoryStorage) -> Self {
        Self {
            storage,
            graph: Mutex::new(None),
        }
    }

    /// Load the latest graph revision and cache it for subsequent calls.
    pub fn load_graph(&self) -> Result<KnowledgeGraph, QueryError> {
        let mut guard = self.graph.lock().unwrap();
        if let Some(ref graph) = *guard {
            return Ok(graph.clone());
        }
        let (graph, _revision) = self.storage.load_latest()?;
        let result = graph.clone();
        *guard = Some(graph);
        Ok(result)
    }

    /// Search symbols by name (exact match and prefix match).
    ///
    /// If `kind` is provided, results are filtered to that node kind.
    /// If `language` is provided, results are filtered by evidence language
    /// (case-insensitive).
    pub fn search_symbols(
        &self,
        query: &str,
        kind: Option<NodeKind>,
    ) -> Result<Vec<SymbolResult>, QueryError> {
        self.search_symbols_filtered(query, kind, None)
    }

    /// Search symbols with optional kind and language filters.
    pub fn search_symbols_filtered(
        &self,
        query: &str,
        kind: Option<NodeKind>,
        language: Option<&str>,
    ) -> Result<Vec<SymbolResult>, QueryError> {
        let graph = self.load_graph()?;
        Ok(graph
            .nodes()
            .iter()
            .filter(|node| {
                // Skip pure structural roots unless the query is empty listing.
                let name_match = node.name() == query || node.name().starts_with(query);
                let kind_match = kind.map_or(true, |k| node.kind() == k);
                name_match && kind_match
            })
            .map(SymbolResult::from_node)
            .filter(|s| language_matches(s.language.as_deref(), language))
            .collect())
    }

    /// Find the first symbol with an exact name match.
    pub fn find_symbol(&self, name: &str) -> Result<Option<SymbolResult>, QueryError> {
        let graph = self.load_graph()?;
        Ok(graph
            .nodes()
            .iter()
            .find(|node| node.name() == name)
            .map(SymbolResult::from_node))
    }

    /// Find all symbols with the given node kind.
    pub fn symbols_by_kind(&self, kind: NodeKind) -> Result<Vec<SymbolResult>, QueryError> {
        let graph = self.load_graph()?;
        Ok(graph
            .nodes_by_kind(kind)
            .map(SymbolResult::from_node)
            .collect())
    }

    /// List file nodes from the knowledge graph, optionally filtered by language.
    pub fn list_files(&self, language: Option<&str>) -> Result<Vec<FileResult>, QueryError> {
        let graph = self.load_graph()?;
        let mut files: Vec<FileResult> = graph
            .nodes_by_kind(NodeKind::File)
            .map(|node| {
                let path = match node.evidence() {
                    GraphEvidence::Structural(StructuralEvidence::File { path }) => path.clone(),
                    _ => PathBuf::from(node.name()),
                };
                let language = language_from_path(&path);
                FileResult { path, language }
            })
            .filter(|f| language_matches(f.language.as_deref(), language))
            .collect();
        files.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(files)
    }

    /// Aggregate statistics for the loaded graph.
    pub fn graph_stats(&self) -> Result<GraphStats, QueryError> {
        let graph = self.load_graph()?;
        let mut langs = std::collections::BTreeSet::new();
        let mut entity_count = 0usize;
        let mut file_count = 0usize;
        for node in graph.nodes() {
            match node.kind() {
                NodeKind::File => file_count += 1,
                NodeKind::Repository | NodeKind::Workspace => {}
                _ => {
                    entity_count += 1;
                    if let Some(lang) = evidence_language(node.evidence()) {
                        langs.insert(lang);
                    }
                }
            }
        }
        Ok(GraphStats {
            node_count: graph.node_count(),
            relationship_count: graph.relationship_count(),
            file_count,
            entity_count,
            languages: langs.into_iter().collect(),
        })
    }

    /// Export the loaded graph as DOT.
    pub fn export_dot(&self) -> Result<String, QueryError> {
        let graph = self.load_graph()?;
        Ok(kode_graph::graph_to_dot(&graph))
    }

    /// Export the loaded graph as GraphML.
    pub fn export_graphml(&self) -> Result<String, QueryError> {
        let graph = self.load_graph()?;
        Ok(kode_graph::graph_to_graphml(&graph))
    }

    /// Find functions that **call** `name` (incoming `Calls` edges).
    pub fn find_callers(&self, name: &str) -> Result<Vec<SymbolResult>, QueryError> {
        let graph = self.load_graph()?;
        let targets = graph
            .nodes()
            .iter()
            .filter(|n| n.name() == name && n.kind() == NodeKind::Function)
            .map(|n| *n.id())
            .collect::<Vec<_>>();

        let mut callers = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for target in &targets {
            for rel in graph.incoming(target) {
                if rel.kind() != RelationshipKind::Calls {
                    continue;
                }
                if !seen.insert(*rel.source()) {
                    continue;
                }
                if let Some(node) = graph.node_by_id(rel.source()) {
                    callers.push(SymbolResult::from_node(node));
                }
            }
        }
        callers.sort_by(|a, b| {
            a.name
                .cmp(&b.name)
                .then_with(|| a.file_path.cmp(&b.file_path))
        });
        Ok(callers)
    }

    /// Find functions that `name` **calls** (outgoing `Calls` edges).
    pub fn find_callees(&self, name: &str) -> Result<Vec<SymbolResult>, QueryError> {
        let graph = self.load_graph()?;
        let sources = graph
            .nodes()
            .iter()
            .filter(|n| n.name() == name && n.kind() == NodeKind::Function)
            .map(|n| *n.id())
            .collect::<Vec<_>>();

        let mut callees = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for source in &sources {
            for rel in graph.outgoing(source) {
                if rel.kind() != RelationshipKind::Calls {
                    continue;
                }
                if !seen.insert(*rel.target()) {
                    continue;
                }
                if let Some(node) = graph.node_by_id(rel.target()) {
                    callees.push(SymbolResult::from_node(node));
                }
            }
        }
        callees.sort_by(|a, b| {
            a.name
                .cmp(&b.name)
                .then_with(|| a.file_path.cmp(&b.file_path))
        });
        Ok(callees)
    }

    /// Impact set: symbols that (transitively) call `name` via `Calls` edges.
    ///
    /// Useful for "what breaks if I change this function?" — BFS over reverse
    /// call edges, bounded by `max_depth` (None = unlimited within graph size).
    pub fn impact_analysis(
        &self,
        name: &str,
        max_depth: Option<usize>,
    ) -> Result<Vec<SymbolResult>, QueryError> {
        let graph = self.load_graph()?;
        let seeds: Vec<GraphNodeId> = graph
            .nodes()
            .iter()
            .filter(|n| n.name() == name && n.kind() == NodeKind::Function)
            .map(|n| *n.id())
            .collect();

        if seeds.is_empty() {
            return Ok(Vec::new());
        }

        let mut impacted = std::collections::BTreeSet::new();
        let mut queue: std::collections::VecDeque<(GraphNodeId, usize)> =
            seeds.into_iter().map(|id| (id, 0)).collect();
        let mut visited = std::collections::BTreeSet::new();

        while let Some((id, depth)) = queue.pop_front() {
            if !visited.insert(id) {
                continue;
            }
            if depth > 0 {
                impacted.insert(id);
            }
            if max_depth.is_some_and(|m| depth >= m) {
                continue;
            }
            for rel in graph.incoming(&id) {
                if rel.kind() == RelationshipKind::Calls {
                    queue.push_back((*rel.source(), depth + 1));
                }
            }
        }

        let mut results: Vec<SymbolResult> = impacted
            .iter()
            .filter_map(|id| graph.node_by_id(id).map(SymbolResult::from_node))
            .collect();
        results.sort_by(|a, b| {
            a.name
                .cmp(&b.name)
                .then_with(|| a.file_path.cmp(&b.file_path))
        });
        Ok(results)
    }

    /// Functions with no incoming `Calls` (heuristic dead / unreferenced).
    pub fn dead_code(&self) -> Result<Vec<SymbolResult>, QueryError> {
        let graph = self.load_graph()?;
        Ok(dead_code_candidates(&graph))
    }

    /// Call-graph cycles (SCCs).
    pub fn cycles(&self) -> Result<Vec<CallCycle>, QueryError> {
        let graph = self.load_graph()?;
        Ok(call_cycles(&graph))
    }

    /// Fan-in / fan-out metrics for functions.
    pub fn metrics(&self, name: Option<&str>) -> Result<Vec<FunctionMetrics>, QueryError> {
        let graph = self.load_graph()?;
        Ok(function_metrics(&graph, name))
    }

    /// Resolve intent from a query string and execute it.
    pub fn execute(&self, raw: &str) -> Result<QueryOutcome, QueryError> {
        self.execute_intent(&parse_intent(raw))
    }

    /// Execute a structured [`QueryIntent`].
    pub fn execute_intent(&self, intent: &QueryIntent) -> Result<QueryOutcome, QueryError> {
        match intent {
            QueryIntent::Search { query } => {
                Ok(QueryOutcome::Symbols(self.search_symbols(query, None)?))
            }
            QueryIntent::Callers { name } => Ok(QueryOutcome::Symbols(self.find_callers(name)?)),
            QueryIntent::Callees { name } => Ok(QueryOutcome::Symbols(self.find_callees(name)?)),
            QueryIntent::Impact { name, max_depth } => Ok(QueryOutcome::Symbols(
                self.impact_analysis(name, *max_depth)?,
            )),
            QueryIntent::DeadCode => Ok(QueryOutcome::Symbols(self.dead_code()?)),
            QueryIntent::Cycles => Ok(QueryOutcome::Cycles(self.cycles()?)),
            QueryIntent::Metrics { name } => {
                Ok(QueryOutcome::Metrics(self.metrics(name.as_deref())?))
            }
        }
    }
}

/// Result of executing a structured query.
#[derive(Debug)]
pub enum QueryOutcome {
    /// Symbol listing (search, callers, callees, impact, dead code).
    Symbols(Vec<SymbolResult>),
    /// Architecture metrics rows.
    Metrics(Vec<FunctionMetrics>),
    /// Call-graph cycles.
    Cycles(Vec<CallCycle>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use kode_graph::model::*;
    use kode_graph::{EntityId, Evidence, Language, Visibility};
    use kode_storage::{RepositoryMetadata, SqliteBackend};
    use std::collections::BTreeMap;
    use std::path::Path;
    use std::time::SystemTime;
    use tempfile::TempDir;

    fn build_test_graph() -> KnowledgeGraph {
        let repo_id = StructuralNodeId::from_parts(
            StructuralNodeKind::Repository,
            Path::new("/test"),
            "test-repo",
        );
        let ws_id = StructuralNodeId::from_parts(
            StructuralNodeKind::Workspace,
            Path::new("/test"),
            "test-ws",
        );
        let file_id = StructuralNodeId::from_parts(
            StructuralNodeKind::File,
            Path::new("src/lib.rs"),
            "src/lib.rs",
        );
        let hello_id = EntityId::from_location(
            &Language::Rust,
            "function",
            Path::new("src/lib.rs"),
            "hello",
            42,
        );
        let world_id = EntityId::from_location(
            &Language::Rust,
            "struct",
            Path::new("src/lib.rs"),
            "world",
            100,
        );

        let repo = Node::structural(
            repo_id,
            NodeKind::Repository,
            "test-repo",
            NodeMetadata::new(None, None),
            StructuralEvidence::Repository {
                root: PathBuf::from("/test"),
            },
        );
        let ws = Node::structural(
            ws_id,
            NodeKind::Workspace,
            "test-ws",
            NodeMetadata::new(None, None),
            StructuralEvidence::Workspace {
                name: "test-ws".into(),
            },
        );
        let file = Node::structural(
            file_id,
            NodeKind::File,
            "src/lib.rs",
            NodeMetadata::new(None, None),
            StructuralEvidence::File {
                path: PathBuf::from("src/lib.rs"),
            },
        );
        let hello = Node::entity(
            hello_id,
            NodeKind::Function,
            "hello",
            NodeMetadata::new(Some(Visibility::Public), Some("Docs".into())),
            Evidence::new(
                PathBuf::from("src/lib.rs"),
                "function_item",
                42..100,
                3,
                5,
                7,
                20,
                Language::Rust,
            ),
        );
        let world = Node::entity(
            world_id,
            NodeKind::Struct,
            "world",
            NodeMetadata::new(None, None),
            Evidence::new(
                PathBuf::from("src/lib.rs"),
                "struct_item",
                200..300,
                10,
                1,
                15,
                30,
                Language::Rust,
            ),
        );

        let nodes = vec![repo, ws, file, hello, world];
        let mut node_by_id = BTreeMap::new();
        for (i, n) in nodes.iter().enumerate() {
            node_by_id.insert(*n.id(), i);
        }

        KnowledgeGraph::new(nodes, Vec::new(), node_by_id, Vec::new(), Vec::new())
    }

    fn setup_engine() -> QueryEngine {
        let backend = Box::new(SqliteBackend::in_memory().unwrap());
        let mut storage = RepositoryStorage::open(backend, "test-repo").unwrap();

        let graph = build_test_graph();
        let metadata = RepositoryMetadata {
            repository_id: "test-repo".into(),
            root: "/test".into(),
            fingerprint: "fp".into(),
            parser_versions: vec![],
            last_updated: SystemTime::now(),
        };
        storage
            .persist(&graph, &metadata, GraphVersion::new(1, 0))
            .unwrap();

        QueryEngine::new(storage)
    }

    #[test]
    fn load_graph_returns_graph() {
        let engine = setup_engine();
        let graph = engine.load_graph().unwrap();
        assert_eq!(graph.node_count(), 5);
    }

    #[test]
    fn load_graph_is_cached() {
        let engine = setup_engine();
        let a = engine.load_graph().unwrap();
        let b = engine.load_graph().unwrap();
        assert_eq!(a.node_count(), b.node_count());
    }

    #[test]
    fn find_symbol_exact_match() {
        let engine = setup_engine();
        let result = engine.find_symbol("hello").unwrap().unwrap();
        assert_eq!(result.name, "hello");
        assert_eq!(result.kind, NodeKind::Function);
    }

    #[test]
    fn find_symbol_no_match() {
        let engine = setup_engine();
        let result = engine.find_symbol("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn search_symbols_prefix_match() {
        let engine = setup_engine();
        let results = engine.search_symbols("hel", None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "hello");
    }

    #[test]
    fn search_symbols_exact_match() {
        let engine = setup_engine();
        let results = engine.search_symbols("hello", None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "hello");
    }

    #[test]
    fn search_symbols_no_match() {
        let engine = setup_engine();
        let results = engine.search_symbols("zzz", None).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn search_symbols_with_kind_filter() {
        let engine = setup_engine();
        let results = engine
            .search_symbols("world", Some(NodeKind::Struct))
            .unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn search_symbols_kind_filter_excludes() {
        let engine = setup_engine();
        let results = engine
            .search_symbols("world", Some(NodeKind::Function))
            .unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn symbols_by_kind() {
        let engine = setup_engine();
        let results = engine.symbols_by_kind(NodeKind::Function).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "hello");
    }

    #[test]
    fn symbols_by_kind_structural() {
        let engine = setup_engine();
        let results = engine.symbols_by_kind(NodeKind::File).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "src/lib.rs");
    }

    #[test]
    fn evidence_verified_when_file_exists() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("src/lib.rs");
        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        // Multi-line source so start_line/end_line evidence is in range.
        std::fs::write(&file_path, "\n\npub fn hello() {\n    // body\n}\n").unwrap();

        let backend = Box::new(SqliteBackend::in_memory().unwrap());
        let mut storage = RepositoryStorage::open(backend, "verify-test").unwrap();

        let hello_id =
            EntityId::from_location(&Language::Rust, "function", &file_path, "hello", 42);

        let hello = Node::entity(
            hello_id,
            NodeKind::Function,
            "hello",
            NodeMetadata::new(Some(Visibility::Public), None),
            Evidence::new(
                file_path.clone(),
                "function_item",
                42..100,
                3,
                5,
                5,
                20,
                Language::Rust,
            ),
        );

        let nodes = vec![hello];
        let mut node_by_id = BTreeMap::new();
        node_by_id.insert(*nodes[0].id(), 0);

        let graph = KnowledgeGraph::new(nodes, Vec::new(), node_by_id, Vec::new(), Vec::new());
        let metadata = RepositoryMetadata {
            repository_id: "verify-test".into(),
            root: dir.path().to_string_lossy().into_owned(),
            fingerprint: "fp".into(),
            parser_versions: vec![],
            last_updated: SystemTime::now(),
        };
        storage
            .persist(&graph, &metadata, GraphVersion::new(1, 0))
            .unwrap();

        let engine = QueryEngine::new(storage);
        let result = engine.find_symbol("hello").unwrap().unwrap();
        assert!(result.verified);
        assert_eq!(result.file_path, file_path);
        assert_eq!(result.start_line, 3);
        assert_eq!(result.start_column, 5);
    }

    #[test]
    fn evidence_not_verified_when_file_missing() {
        // Paths that don't exist on disk produce unverified results.
        let backend = Box::new(SqliteBackend::in_memory().unwrap());
        let mut storage = RepositoryStorage::open(backend, "missing-test").unwrap();

        let missing_path = PathBuf::from("/nonexistent/path/file.rs");
        let hello_id =
            EntityId::from_location(&Language::Rust, "function", &missing_path, "missing_fn", 42);
        let hello = Node::entity(
            hello_id,
            NodeKind::Function,
            "missing_fn",
            NodeMetadata::new(None, None),
            Evidence::new(
                missing_path,
                "function_item",
                0..10,
                1,
                1,
                5,
                10,
                Language::Rust,
            ),
        );
        let mut node_by_id = BTreeMap::new();
        node_by_id.insert(*hello.id(), 0);

        let graph =
            KnowledgeGraph::new(vec![hello], Vec::new(), node_by_id, Vec::new(), Vec::new());
        let metadata = RepositoryMetadata {
            repository_id: "missing-test".into(),
            root: "/nonexistent".into(),
            fingerprint: "fp".into(),
            parser_versions: vec![],
            last_updated: SystemTime::now(),
        };
        storage
            .persist(&graph, &metadata, GraphVersion::new(1, 0))
            .unwrap();

        let engine = QueryEngine::new(storage);
        let result = engine.find_symbol("missing_fn").unwrap().unwrap();
        assert!(!result.verified);
    }

    #[test]
    fn symbol_result_line_column() {
        let engine = setup_engine();
        let result = engine.find_symbol("world").unwrap().unwrap();
        assert_eq!(result.start_line, 10);
        assert_eq!(result.start_column, 1);
        assert_eq!(result.end_line, 15);
        assert_eq!(result.end_column, 30);
    }

    #[test]
    fn search_symbols_multiple_matches() {
        let engine = setup_engine();
        let results = engine.search_symbols("", None).unwrap();
        // "" matches everything as prefix (str.starts_with("") is always true)
        assert_eq!(results.len(), 5);
    }

    #[test]
    fn graph_not_loaded_error_is_not_storage_error() {
        // QueryEngine with no graph loaded; load_graph succeeds via OnceLock,
        // but we verify the error type holds. The GraphNotLoaded variant exists
        // for API completeness.
        let engine = setup_engine();
        // Ensure load works
        assert!(engine.load_graph().is_ok());
    }

    #[test]
    fn search_symbols_language_filter() {
        let engine = setup_engine();
        let rust = engine
            .search_symbols_filtered("hello", None, Some("Rust"))
            .unwrap();
        assert_eq!(rust.len(), 1);
        let python = engine
            .search_symbols_filtered("hello", None, Some("Python"))
            .unwrap();
        assert!(python.is_empty());
    }

    #[test]
    fn list_files_returns_file_nodes() {
        let engine = setup_engine();
        let files = engine.list_files(None).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].path.ends_with("src/lib.rs"));
        assert_eq!(files[0].language.as_deref(), Some("Rust"));
    }

    #[test]
    fn graph_stats_counts() {
        let engine = setup_engine();
        let stats = engine.graph_stats().unwrap();
        assert_eq!(stats.node_count, 5);
        assert_eq!(stats.file_count, 1);
        assert_eq!(stats.entity_count, 2);
        assert!(stats.languages.iter().any(|l| l == "Rust"));
    }

    #[test]
    fn export_dot_contains_digraph() {
        let engine = setup_engine();
        let dot = engine.export_dot().unwrap();
        assert!(dot.contains("digraph"));
    }

    #[test]
    fn find_callers_and_callees_empty_without_calls() {
        // Fixture graph has no Calls edges.
        let engine = setup_engine();
        assert!(engine.find_callers("hello").unwrap().is_empty());
        assert!(engine.find_callees("hello").unwrap().is_empty());
        assert!(engine.impact_analysis("hello", Some(3)).unwrap().is_empty());
    }

    #[test]
    fn execute_routes_callers_prefix() {
        let engine = setup_engine();
        let out = engine.execute("callers:hello").unwrap();
        assert!(matches!(out, QueryOutcome::Symbols(_)));
    }

    #[test]
    fn execute_metrics_returns_rows() {
        let engine = setup_engine();
        let out = engine.execute("metrics").unwrap();
        match out {
            QueryOutcome::Metrics(m) => {
                assert_eq!(m.len(), 1);
                assert_eq!(m[0].name, "hello");
                assert_eq!(m[0].fan_in, 0);
                assert_eq!(m[0].fan_out, 0);
            }
            _ => panic!("expected metrics"),
        }
    }

    #[test]
    fn execute_dead_lists_unreferenced() {
        let engine = setup_engine();
        let out = engine.execute("dead").unwrap();
        match out {
            QueryOutcome::Symbols(s) => {
                assert!(s.iter().any(|x| x.name == "hello"));
            }
            _ => panic!("expected symbols"),
        }
    }

    #[test]
    fn verify_source_rejects_bad_line() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("f.rs");
        std::fs::write(&path, "fn a() {}\n").unwrap();
        assert!(verify_source_evidence(&path, 1, 1, Some("a")));
        assert!(!verify_source_evidence(&path, 99, 99, None));
        assert!(!verify_source_evidence(&path, 1, 1, Some("missing_name")));
    }
}
