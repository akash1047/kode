use std::time::SystemTime;

pub use kode_graph::model::GraphVersion;

/// Storage schema version — incremented on breaking schema changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchemaVersion {
    pub major: u16,
    pub minor: u16,
}

impl SchemaVersion {
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }
}

impl std::fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// Immutable value representing one persisted repository state.
///
/// Consumers in later stages (Analysis, Query Engine, MCP) should prefer
/// consuming this over raw storage handles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphRevision {
    pub revision_id: u64,
    pub repository_id: String,
    pub graph_version: GraphVersion,
    pub node_count: usize,
    pub relationship_count: usize,
    pub content_hash: String,
    pub created_at: SystemTime,
}

/// Identity and metadata about a persisted repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryMetadata {
    pub repository_id: String,
    pub root: String,
    pub fingerprint: String,
    pub parser_versions: Vec<String>,
    pub last_updated: SystemTime,
}

/// Cache metadata for a repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheMetadata {
    pub repository_id: String,
    pub latest_revision: Option<u64>,
    pub revision_count: usize,
    pub schema_version: SchemaVersion,
    pub graph_version: GraphVersion,
    pub last_accessed: Option<SystemTime>,
    pub total_nodes: usize,
    pub total_relationships: usize,
}
