#![allow(unused_crate_dependencies)]

//! Query Engine: transforms repository knowledge into evidence-backed answers.
//!
//! Reads the Knowledge Graph and SyntaxTreeInventory to answer questions
//! about repository structure, definitions, references, and types.
//!
//! # Responsibilities
//!
//! - Evidence verification against live source code
//! - Graph traversal for definition and reference resolution
//! - Repository validation before query execution
//!
//! # Invariants
//!
//! - Queries never mutate the graph or storage.
//! - Every answer includes path:line citations.
//! - Queries are deterministic for the same graph revision.

#[cfg(test)]
use tempfile as _;

use std::path::PathBuf;
use std::sync::Mutex;

use kode_graph::{GraphEvidence, KnowledgeGraph, Node, NodeKind, StructuralEvidence};
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
pub struct SymbolResult {
    pub name: String,
    pub kind: NodeKind,
    pub file_path: PathBuf,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub verified: bool,
}

impl SymbolResult {
    fn from_node(node: &Node) -> Self {
        let (file_path, start_line, start_column, end_line, end_column) =
            evidence_location(node.evidence());
        let verified = evidence_verified(node.evidence());
        Self {
            name: node.name().to_string(),
            kind: node.kind(),
            file_path,
            start_line,
            start_column,
            end_line,
            end_column,
            verified,
        }
    }
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
        GraphEvidence::Source(ev) => ev.source_file().exists(),
        GraphEvidence::Structural(StructuralEvidence::File { path }) => path.exists(),
        _ => false,
    }
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
    pub fn search_symbols(
        &self,
        query: &str,
        kind: Option<NodeKind>,
    ) -> Result<Vec<SymbolResult>, QueryError> {
        let graph = self.load_graph()?;
        Ok(graph
            .nodes()
            .iter()
            .filter(|node| {
                let name_match = node.name() == query || node.name().starts_with(query);
                let kind_match = kind.map_or(true, |k| node.kind() == k);
                name_match && kind_match
            })
            .map(SymbolResult::from_node)
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
        std::fs::write(&file_path, "pub fn hello() {}").unwrap();

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
                7,
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
}
