use kode_graph::KnowledgeGraph;

use crate::backend::StorageBackend;
use crate::error::StorageError;
use crate::model::*;

/// Repository-scoped storage service.
///
/// Wraps a [`StorageBackend`] with a fixed repository identity, providing
/// a focused API for persisting and loading graph revisions for that
/// repository.
///
/// # Lifecycle
///
/// 1. [`RepositoryStorage::open`] initialises the backend and binds to a
///    repository.
/// 2. [`persist`](RepositoryStorage::persist) commits a graph revision.
/// 3. [`load_latest`](RepositoryStorage::load_latest) retrieves the most
///    recent revision.
/// 4. [`clear`](RepositoryStorage::clear) removes all data for the
///    repository.
pub struct RepositoryStorage {
    backend: Box<dyn StorageBackend>,
    repository_id: String,
}

impl RepositoryStorage {
    /// Open storage for the given repository.
    ///
    /// Calls [`StorageBackend::initialize`] to ensure the schema is ready.
    pub fn open(
        mut backend: Box<dyn StorageBackend>,
        repository_id: impl Into<String>,
    ) -> Result<Self, StorageError> {
        backend.initialize()?;
        Ok(Self {
            backend,
            repository_id: repository_id.into(),
        })
    }

    /// Persist a graph revision.
    ///
    /// All data is written atomically. On failure the backend is left in
    /// its previous state.
    pub fn persist(
        &mut self,
        graph: &KnowledgeGraph,
        metadata: &RepositoryMetadata,
        graph_version: GraphVersion,
    ) -> Result<GraphRevision, StorageError> {
        self.backend
            .save_revision(&self.repository_id, metadata, graph, graph_version)
    }

    /// Load the latest graph revision.
    ///
    /// Returns an error if no revisions exist for this repository.
    pub fn load_latest(&self) -> Result<(KnowledgeGraph, GraphRevision), StorageError> {
        self.backend.load_latest_graph(&self.repository_id)
    }

    /// Load a specific revision by ID.
    pub fn load(&self, revision_id: u64) -> Result<(KnowledgeGraph, GraphRevision), StorageError> {
        self.backend.load_graph(revision_id)
    }

    /// List all revisions for this repository, newest first.
    pub fn revisions(&self) -> Result<Vec<GraphRevision>, StorageError> {
        self.backend.list_revisions(&self.repository_id)
    }

    /// Return cache metadata for this repository.
    pub fn metadata(&self) -> Result<CacheMetadata, StorageError> {
        self.backend.cache_metadata(&self.repository_id)
    }

    /// Return the stored repository content fingerprint used for incremental skip.
    pub fn fingerprint(&self) -> Result<Option<String>, StorageError> {
        self.backend.repository_fingerprint(&self.repository_id)
    }

    /// Replace per-file content hashes.
    pub fn save_file_hashes(&mut self, hashes: &[(String, String)]) -> Result<(), StorageError> {
        self.backend.save_file_hashes(&self.repository_id, hashes)
    }

    /// Load per-file content hashes.
    pub fn load_file_hashes(&self) -> Result<Vec<(String, String)>, StorageError> {
        self.backend.load_file_hashes(&self.repository_id)
    }

    /// Persist facts JSON for incremental merges.
    pub fn save_facts_json(&mut self, facts_json: &str) -> Result<(), StorageError> {
        self.backend
            .save_facts_json(&self.repository_id, facts_json)
    }

    /// Load facts JSON if present.
    pub fn load_facts_json(&self) -> Result<Option<String>, StorageError> {
        self.backend.load_facts_json(&self.repository_id)
    }

    /// Remove all cached data for this repository.
    pub fn clear(&mut self) -> Result<(), StorageError> {
        self.backend.remove_repository(&self.repository_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sqlite::SqliteBackend;
    use kode_graph::model::*;
    use kode_graph::{EntityId, Evidence, Language, Visibility};
    use std::collections::BTreeMap;
    use std::path::Path;
    use std::path::PathBuf;
    use std::time::SystemTime;

    fn sample_graph() -> KnowledgeGraph {
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
        let entity_id = EntityId::from_location(
            &Language::Rust,
            "function",
            Path::new("src/lib.rs"),
            "hello",
            42,
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
        let func = Node::entity(
            entity_id,
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

        let nodes = vec![repo, ws, file, func];
        let mut node_by_id: BTreeMap<GraphNodeId, usize> = BTreeMap::new();
        for (i, n) in nodes.iter().enumerate() {
            node_by_id.insert(*n.id(), i);
        }

        let rels = vec![
            Relationship::structural(
                GraphNodeId::Structural(repo_id),
                GraphNodeId::Structural(ws_id),
                RelationshipKind::Contains,
                RelationshipMetadata::new(),
                StructuralEvidence::Workspace {
                    name: "test-ws".into(),
                },
            ),
            Relationship::structural(
                GraphNodeId::Structural(ws_id),
                GraphNodeId::Structural(file_id),
                RelationshipKind::Contains,
                RelationshipMetadata::new(),
                StructuralEvidence::File {
                    path: PathBuf::from("src/lib.rs"),
                },
            ),
            Relationship::with_source(
                GraphNodeId::Structural(file_id),
                GraphNodeId::Entity(entity_id),
                RelationshipKind::Declares,
                RelationshipMetadata::new(),
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
            ),
        ];

        let n = nodes.len();
        let mut outgoing: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut incoming: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (ri, r) in rels.iter().enumerate() {
            if let Some(&idx) = node_by_id.get(r.source()) {
                outgoing[idx].push(ri);
            }
            if let Some(&idx) = node_by_id.get(r.target()) {
                incoming[idx].push(ri);
            }
        }

        KnowledgeGraph::new(nodes, rels, node_by_id, outgoing, incoming)
    }

    #[test]
    fn service_persist_and_load() {
        let backend = Box::new(SqliteBackend::in_memory().unwrap());
        let mut storage = RepositoryStorage::open(backend, "service-test").unwrap();

        let graph = sample_graph();
        let metadata = RepositoryMetadata {
            repository_id: "service-test".into(),
            root: "/test".into(),
            fingerprint: "fp".into(),
            parser_versions: vec![],
            last_updated: SystemTime::now(),
        };

        let revision = storage
            .persist(&graph, &metadata, GraphVersion::new(1, 0))
            .unwrap();
        assert_eq!(revision.node_count, 4);

        let (loaded, _) = storage.load_latest().unwrap();
        assert_eq!(loaded.node_count(), 4);
        assert_eq!(loaded.relationship_count(), 3);
    }

    #[test]
    fn service_revisions_list() {
        let backend = Box::new(SqliteBackend::in_memory().unwrap());
        let mut storage = RepositoryStorage::open(backend, "list-test").unwrap();

        let empty = KnowledgeGraph::new(
            Vec::new(),
            Vec::new(),
            BTreeMap::new(),
            Vec::new(),
            Vec::new(),
        );
        let metadata = RepositoryMetadata {
            repository_id: "list-test".into(),
            root: "/".into(),
            fingerprint: "fp".into(),
            parser_versions: vec![],
            last_updated: SystemTime::now(),
        };

        storage
            .persist(&empty, &metadata, GraphVersion::new(1, 0))
            .unwrap();
        storage
            .persist(&sample_graph(), &metadata, GraphVersion::new(1, 0))
            .unwrap();

        let revisions = storage.revisions().unwrap();
        assert_eq!(revisions.len(), 2);
    }

    #[test]
    fn service_clear() {
        let backend = Box::new(SqliteBackend::in_memory().unwrap());
        let mut storage = RepositoryStorage::open(backend, "clear-test").unwrap();

        let empty = KnowledgeGraph::new(
            Vec::new(),
            Vec::new(),
            BTreeMap::new(),
            Vec::new(),
            Vec::new(),
        );
        let metadata = RepositoryMetadata {
            repository_id: "clear-test".into(),
            root: "/".into(),
            fingerprint: "fp".into(),
            parser_versions: vec![],
            last_updated: SystemTime::now(),
        };

        storage
            .persist(&empty, &metadata, GraphVersion::new(1, 0))
            .unwrap();
        storage.clear().unwrap();

        let err = storage.load_latest();
        assert!(matches!(err, Err(StorageError::RepositoryNotFound(_))));
    }
}
