use kode_graph::KnowledgeGraph;

use crate::error::StorageError;
use crate::model::*;

/// Abstract persistence backend for graph revisions.
///
/// Implementations handle physical storage details (e.g. SQLite, in-memory)
/// while the trait defines a revision-oriented, transactional interface.
///
/// # Invariants
///
/// * [`save_revision`](StorageBackend::save_revision) must persist all data
///   atomically — no partial revisions on failure.
/// * A committed revision is never mutated in place.
/// * [`load_graph`](StorageBackend::load_graph) must reconstruct a
///   [`KnowledgeGraph`] whose traversal methods produce identical results
///   to the original.
pub trait StorageBackend: Send {
    /// Initialise the backend (create schema tables if needed).
    fn initialize(&mut self) -> Result<(), StorageError>;

    /// Persist an entire graph revision atomically.
    fn save_revision(
        &mut self,
        repository_id: &str,
        metadata: &RepositoryMetadata,
        graph: &KnowledgeGraph,
        graph_version: GraphVersion,
    ) -> Result<GraphRevision, StorageError>;

    /// Return the latest revision ID for a repository, if any.
    fn load_latest_revision_id(&self, repository_id: &str) -> Result<Option<u64>, StorageError>;

    /// Load revision metadata without reconstructing the graph.
    fn load_revision_metadata(&self, revision_id: u64) -> Result<GraphRevision, StorageError>;

    /// Load a full graph for the given revision.
    fn load_graph(&self, revision_id: u64)
        -> Result<(KnowledgeGraph, GraphRevision), StorageError>;

    /// Load the latest graph revision for a repository.
    fn load_latest_graph(
        &self,
        repository_id: &str,
    ) -> Result<(KnowledgeGraph, GraphRevision), StorageError>;

    /// List all revisions for a repository, newest first.
    fn list_revisions(&self, repository_id: &str) -> Result<Vec<GraphRevision>, StorageError>;

    /// Remove all data for a repository.
    fn remove_repository(&mut self, repository_id: &str) -> Result<(), StorageError>;

    /// Return cache metadata for a repository.
    fn cache_metadata(&self, repository_id: &str) -> Result<CacheMetadata, StorageError>;

    /// Return the stored content fingerprint for a repository, if any.
    fn repository_fingerprint(&self, repository_id: &str) -> Result<Option<String>, StorageError>;

    /// Return the schema version detected in the backend.
    fn schema_version(&self) -> Result<SchemaVersion, StorageError>;
}
