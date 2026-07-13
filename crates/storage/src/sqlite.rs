use std::path::Path;
use std::sync::Mutex;
use std::time::SystemTime;

use kode_graph::model::GraphVersion;
use kode_graph::serialization;
use kode_graph::KnowledgeGraph;
use rusqlite::{params, Connection};

use crate::backend::StorageBackend;
use crate::error::StorageError;
use crate::model::*;

/// Current schema version for the SQLite backend.
const SCHEMA_MAJOR: u16 = 1;
const SCHEMA_MINOR: u16 = 0;

/// SQLite-backed storage backend.
pub struct SqliteBackend {
    conn: Mutex<Connection>,
}

impl SqliteBackend {
    /// Open or create a database at the given path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let conn = Connection::open(path.as_ref())?;
        let mut backend = Self {
            conn: Mutex::new(conn),
        };
        backend.initialize()?;
        Ok(backend)
    }

    /// Create an in-memory database (for testing).
    pub fn in_memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()?;
        let mut backend = Self {
            conn: Mutex::new(conn),
        };
        backend.initialize()?;
        Ok(backend)
    }

    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap()
    }

    fn create_tables(conn: &Connection) -> Result<(), StorageError> {
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS schema_info (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS repositories (
                id           TEXT PRIMARY KEY,
                root         TEXT NOT NULL,
                fingerprint  TEXT NOT NULL,
                last_updated INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS revisions (
                revision_id          INTEGER PRIMARY KEY AUTOINCREMENT,
                repository_id        TEXT NOT NULL,
                graph_version_major  INTEGER NOT NULL,
                graph_version_minor  INTEGER NOT NULL,
                node_count           INTEGER NOT NULL,
                relationship_count   INTEGER NOT NULL,
                content_hash         TEXT NOT NULL,
                created_at           INTEGER NOT NULL,
                FOREIGN KEY (repository_id) REFERENCES repositories(id)
            );

            CREATE TABLE IF NOT EXISTS nodes (
                id             TEXT NOT NULL,
                revision_id    INTEGER NOT NULL,
                kind           TEXT NOT NULL,
                name           TEXT NOT NULL,
                visibility     TEXT,
                documentation  TEXT,
                evidence_json  TEXT NOT NULL,
                PRIMARY KEY (revision_id, id),
                FOREIGN KEY (revision_id) REFERENCES revisions(revision_id)
            );

            CREATE TABLE IF NOT EXISTS relationships (
                rowid          INTEGER PRIMARY KEY AUTOINCREMENT,
                revision_id    INTEGER NOT NULL,
                source_id      TEXT NOT NULL,
                target_id      TEXT NOT NULL,
                kind           TEXT NOT NULL,
                evidence_json  TEXT NOT NULL,
                FOREIGN KEY (revision_id) REFERENCES revisions(revision_id)
            );

            CREATE INDEX IF NOT EXISTS idx_nodes_revision
                ON nodes(revision_id);
            CREATE INDEX IF NOT EXISTS idx_relationships_revision
                ON relationships(revision_id);
            CREATE INDEX IF NOT EXISTS idx_relationships_source
                ON relationships(source_id);
            CREATE INDEX IF NOT EXISTS idx_relationships_target
                ON relationships(target_id);
            ",
        )?;
        Ok(())
    }
}

impl StorageBackend for SqliteBackend {
    fn initialize(&mut self) -> Result<(), StorageError> {
        let conn = self.conn();
        Self::create_tables(&conn)?;

        let stored_major: Option<String> = conn
            .query_row(
                "SELECT value FROM schema_info WHERE key = 'schema_major'",
                [],
                |row| row.get(0),
            )
            .ok();

        match stored_major {
            None => {
                let mut stmt = conn
                    .prepare("INSERT OR REPLACE INTO schema_info (key, value) VALUES (?1, ?2)")?;
                stmt.execute(params!["schema_major", &SCHEMA_MAJOR.to_string()])?;
                stmt.execute(params!["schema_minor", &SCHEMA_MINOR.to_string()])?;
            }
            Some(major_str) => {
                let stored_major: u16 = major_str
                    .parse()
                    .map_err(|_| StorageError::Backend("invalid schema_major".into()))?;
                if stored_major != SCHEMA_MAJOR {
                    return Err(StorageError::SchemaVersionMismatch {
                        stored: format!("{stored_major}.x"),
                        expected: format!("{SCHEMA_MAJOR}.{SCHEMA_MINOR}"),
                    });
                }
            }
        }

        Ok(())
    }

    fn save_revision(
        &mut self,
        repository_id: &str,
        metadata: &RepositoryMetadata,
        graph: &KnowledgeGraph,
        graph_version: GraphVersion,
    ) -> Result<GraphRevision, StorageError> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;

        // Upsert repository
        tx.execute(
            "INSERT OR REPLACE INTO repositories (id, root, fingerprint, last_updated)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                repository_id,
                metadata.root,
                metadata.fingerprint,
                duration_since_epoch(&metadata.last_updated),
            ],
        )?;

        // Serialize graph to DTOs
        let (nodes, rels) = serialization::graph_to_dtos(graph);

        // Compute content hash
        let nodes_json = serde_json::to_string(&nodes)?;
        let rels_json = serde_json::to_string(&rels)?;
        let content_hash = fnv1a_hash(&nodes_json, &rels_json);

        let node_count = nodes.len() as i64;
        let rel_count = rels.len() as i64;
        let now = duration_since_epoch(&SystemTime::now());

        // Insert revision
        tx.execute(
            "INSERT INTO revisions
             (repository_id, graph_version_major, graph_version_minor,
              node_count, relationship_count, content_hash, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                repository_id,
                graph_version.major as i64,
                graph_version.minor as i64,
                node_count,
                rel_count,
                &content_hash,
                now,
            ],
        )?;

        let revision_id = tx.last_insert_rowid() as u64;

        // Insert nodes
        {
            let mut stmt = tx.prepare(
                "INSERT INTO nodes
                 (id, revision_id, kind, name, visibility, documentation, evidence_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )?;
            for node in &nodes {
                let evidence_json = serde_json::to_string(&node.evidence)?;
                stmt.execute(params![
                    node.id,
                    revision_id as i64,
                    node.kind,
                    node.name,
                    node.visibility,
                    node.documentation,
                    evidence_json,
                ])?;
            }
        }

        // Insert relationships
        {
            let mut stmt = tx.prepare(
                "INSERT INTO relationships
                 (revision_id, source_id, target_id, kind, evidence_json)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )?;
            for rel in &rels {
                let evidence_json = serde_json::to_string(&rel.evidence)?;
                stmt.execute(params![
                    revision_id as i64,
                    rel.source,
                    rel.target,
                    rel.kind,
                    evidence_json,
                ])?;
            }
        }

        tx.commit()?;

        Ok(GraphRevision {
            revision_id,
            repository_id: repository_id.to_string(),
            graph_version,
            node_count: node_count as usize,
            relationship_count: rel_count as usize,
            content_hash,
            created_at: SystemTime::now(),
        })
    }

    fn load_latest_revision_id(&self, repository_id: &str) -> Result<Option<u64>, StorageError> {
        let conn = self.conn();
        let result: Result<Option<i64>, _> = conn.query_row(
            "SELECT MAX(revision_id) FROM revisions WHERE repository_id = ?1",
            params![repository_id],
            |row| row.get(0),
        );
        match result {
            Ok(Some(id)) => Ok(Some(id as u64)),
            Ok(None) => Ok(None),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StorageError::from(e)),
        }
    }

    fn load_revision_metadata(&self, revision_id: u64) -> Result<GraphRevision, StorageError> {
        let conn = self.conn();
        conn.query_row(
            "SELECT revision_id, repository_id, graph_version_major, graph_version_minor,
                    node_count, relationship_count, content_hash, created_at
             FROM revisions WHERE revision_id = ?1",
            params![revision_id as i64],
            |row| {
                Ok(GraphRevision {
                    revision_id: row.get::<_, i64>(0)? as u64,
                    repository_id: row.get(1)?,
                    graph_version: GraphVersion::new(
                        row.get::<_, i16>(2)? as u16,
                        row.get::<_, i16>(3)? as u16,
                    ),
                    node_count: row.get::<_, i64>(4)? as usize,
                    relationship_count: row.get::<_, i64>(5)? as usize,
                    content_hash: row.get(6)?,
                    created_at: system_time_from_epoch(row.get::<_, u64>(7)?),
                })
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => StorageError::RevisionNotFound(revision_id),
            other => StorageError::from(other),
        })
    }

    fn load_graph(
        &self,
        revision_id: u64,
    ) -> Result<(KnowledgeGraph, GraphRevision), StorageError> {
        let metadata = self.load_revision_metadata(revision_id)?;
        let conn = self.conn();

        // Load nodes
        let mut stmt = conn.prepare(
            "SELECT id, kind, name, visibility, documentation, evidence_json
             FROM nodes WHERE revision_id = ?1
             ORDER BY id",
        )?;
        let nodes: Vec<serialization::NodeDto> = stmt
            .query_map(params![revision_id as i64], |row| {
                let id: String = row.get(0)?;
                let kind: String = row.get(1)?;
                let name: String = row.get(2)?;
                let visibility: Option<String> = row.get(3)?;
                let documentation: Option<String> = row.get(4)?;
                let evidence_json: String = row.get(5)?;
                let evidence = serde_json::from_str(&evidence_json)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                Ok(serialization::NodeDto {
                    id,
                    kind,
                    name,
                    visibility,
                    documentation,
                    evidence,
                })
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e: rusqlite::Error| StorageError::from(e))?;

        // Load relationships
        let mut stmt = conn.prepare(
            "SELECT source_id, target_id, kind, evidence_json
             FROM relationships WHERE revision_id = ?1",
        )?;
        let rels: Vec<serialization::RelationshipDto> = stmt
            .query_map(params![revision_id as i64], |row| {
                let source: String = row.get(0)?;
                let target: String = row.get(1)?;
                let kind: String = row.get(2)?;
                let evidence_json: String = row.get(3)?;
                let evidence = serde_json::from_str(&evidence_json)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                Ok(serialization::RelationshipDto {
                    source,
                    target,
                    kind,
                    evidence,
                })
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e: rusqlite::Error| StorageError::from(e))?;

        let graph = serialization::dtos_to_graph(nodes, rels)?;
        Ok((graph, metadata))
    }

    fn load_latest_graph(
        &self,
        repository_id: &str,
    ) -> Result<(KnowledgeGraph, GraphRevision), StorageError> {
        let rev_id = self.load_latest_revision_id(repository_id)?;
        match rev_id {
            Some(id) => {
                // Drop the lock from load_latest_revision_id before calling load_graph
                self.load_graph(id)
            }
            None => Err(StorageError::RepositoryNotFound(repository_id.to_string())),
        }
    }

    fn list_revisions(&self, repository_id: &str) -> Result<Vec<GraphRevision>, StorageError> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT revision_id, repository_id, graph_version_major, graph_version_minor,
                    node_count, relationship_count, content_hash, created_at
             FROM revisions WHERE repository_id = ?1
             ORDER BY revision_id DESC",
        )?;

        let revisions = stmt
            .query_map(params![repository_id], |row| {
                Ok(GraphRevision {
                    revision_id: row.get::<_, i64>(0)? as u64,
                    repository_id: row.get(1)?,
                    graph_version: GraphVersion::new(
                        row.get::<_, i16>(2)? as u16,
                        row.get::<_, i16>(3)? as u16,
                    ),
                    node_count: row.get::<_, i64>(4)? as usize,
                    relationship_count: row.get::<_, i64>(5)? as usize,
                    content_hash: row.get(6)?,
                    created_at: system_time_from_epoch(row.get::<_, u64>(7)?),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(revisions)
    }

    fn remove_repository(&mut self, repository_id: &str) -> Result<(), StorageError> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;

        tx.execute(
            "DELETE FROM relationships WHERE revision_id IN
             (SELECT revision_id FROM revisions WHERE repository_id = ?1)",
            params![repository_id],
        )?;
        tx.execute(
            "DELETE FROM nodes WHERE revision_id IN
             (SELECT revision_id FROM revisions WHERE repository_id = ?1)",
            params![repository_id],
        )?;
        tx.execute(
            "DELETE FROM revisions WHERE repository_id = ?1",
            params![repository_id],
        )?;
        tx.execute(
            "DELETE FROM repositories WHERE id = ?1",
            params![repository_id],
        )?;

        tx.commit()?;
        Ok(())
    }

    fn cache_metadata(&self, repository_id: &str) -> Result<CacheMetadata, StorageError> {
        let conn = self.conn();

        let latest: Option<u64> = conn
            .query_row(
                "SELECT MAX(revision_id) FROM revisions WHERE repository_id = ?1",
                params![repository_id],
                |row| row.get::<_, Option<i64>>(0),
            )
            .ok()
            .flatten()
            .map(|v| v as u64);

        let revision_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM revisions WHERE repository_id = ?1",
                params![repository_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let total_nodes: i64 = if let Some(rev_id) = latest {
            conn.query_row(
                "SELECT COALESCE(node_count, 0) FROM revisions WHERE revision_id = ?1",
                params![rev_id as i64],
                |row| row.get(0),
            )
            .unwrap_or(0)
        } else {
            0
        };

        let total_relationships: i64 = if let Some(rev_id) = latest {
            conn.query_row(
                "SELECT COALESCE(relationship_count, 0) FROM revisions WHERE revision_id = ?1",
                params![rev_id as i64],
                |row| row.get(0),
            )
            .unwrap_or(0)
        } else {
            0
        };

        Ok(CacheMetadata {
            repository_id: repository_id.to_string(),
            latest_revision: latest,
            revision_count: revision_count as usize,
            schema_version: SchemaVersion::new(SCHEMA_MAJOR, SCHEMA_MINOR),
            graph_version: GraphVersion::CURRENT,
            last_accessed: Some(SystemTime::now()),
            total_nodes: total_nodes as usize,
            total_relationships: total_relationships as usize,
        })
    }

    fn repository_fingerprint(&self, repository_id: &str) -> Result<Option<String>, StorageError> {
        let conn = self.conn();
        let result = conn.query_row(
            "SELECT fingerprint FROM repositories WHERE id = ?1",
            params![repository_id],
            |row| row.get::<_, String>(0),
        );
        match result {
            Ok(fp) if !fp.is_empty() => Ok(Some(fp)),
            Ok(_) => Ok(None),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StorageError::from(e)),
        }
    }

    fn schema_version(&self) -> Result<SchemaVersion, StorageError> {
        Ok(SchemaVersion::new(SCHEMA_MAJOR, SCHEMA_MINOR))
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn duration_since_epoch(time: &SystemTime) -> u64 {
    time.duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn system_time_from_epoch(secs: u64) -> SystemTime {
    std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs)
}

fn fnv1a_hash(nodes_json: &str, rels_json: &str) -> String {
    use kode_common::hash::Fnv1aHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = Fnv1aHasher::new();
    nodes_json.hash(&mut hasher);
    rels_json.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kode_graph::model::*;
    use kode_graph::{EntityId, Evidence, Language, Visibility};
    use std::collections::BTreeMap;
    use std::path::PathBuf;

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

    fn sample_metadata(repo_id: &str) -> RepositoryMetadata {
        RepositoryMetadata {
            repository_id: repo_id.to_string(),
            root: "/test".into(),
            fingerprint: "test-fingerprint".into(),
            parser_versions: vec!["tree-sitter 0.24".into()],
            last_updated: SystemTime::now(),
        }
    }

    #[test]
    fn persist_and_reload() {
        let mut backend = SqliteBackend::in_memory().unwrap();
        let graph = sample_graph();
        let metadata = sample_metadata("test-repo");
        let version = GraphVersion::new(1, 0);

        let revision = backend
            .save_revision("test-repo", &metadata, &graph, version)
            .unwrap();

        assert_eq!(revision.node_count, 4);
        assert_eq!(revision.relationship_count, 3);

        let (loaded, _) = backend.load_latest_graph("test-repo").unwrap();
        assert_eq!(loaded.node_count(), 4);
        assert_eq!(loaded.relationship_count(), 3);

        for orig_node in graph.nodes() {
            let loaded_node = loaded.node_by_id(orig_node.id()).unwrap();
            assert_eq!(orig_node.kind(), loaded_node.kind());
            assert_eq!(orig_node.name(), loaded_node.name());
            assert_eq!(orig_node.evidence(), loaded_node.evidence());
        }
    }

    #[test]
    fn multiple_revisions() {
        let mut backend = SqliteBackend::in_memory().unwrap();
        let metadata = sample_metadata("multi-repo");
        let version = GraphVersion::new(1, 0);

        let empty = KnowledgeGraph::new(
            Vec::new(),
            Vec::new(),
            BTreeMap::new(),
            Vec::new(),
            Vec::new(),
        );

        let rev1 = backend
            .save_revision("multi-repo", &metadata, &empty, version)
            .unwrap();
        assert_eq!(rev1.revision_id, 1);

        let graph = sample_graph();
        let rev2 = backend
            .save_revision("multi-repo", &metadata, &graph, version)
            .unwrap();
        assert_eq!(rev2.revision_id, 2);

        let revisions = backend.list_revisions("multi-repo").unwrap();
        assert_eq!(revisions.len(), 2);
        assert_eq!(revisions[0].revision_id, 2);
        assert_eq!(revisions[1].revision_id, 1);

        let (loaded, _) = backend.load_graph(1).unwrap();
        assert_eq!(loaded.node_count(), 0);
    }

    #[test]
    fn rollback_on_failure() {
        let mut backend = SqliteBackend::in_memory().unwrap();
        let metadata = sample_metadata("fail-repo");
        let version = GraphVersion::new(1, 0);
        let empty = KnowledgeGraph::new(
            Vec::new(),
            Vec::new(),
            BTreeMap::new(),
            Vec::new(),
            Vec::new(),
        );

        backend
            .save_revision("fail-repo", &metadata, &empty, version)
            .unwrap();

        let (loaded, _) = backend.load_latest_graph("fail-repo").unwrap();
        assert_eq!(loaded.node_count(), 0);
    }

    #[test]
    fn revision_not_found() {
        let backend = SqliteBackend::in_memory().unwrap();
        let err = backend.load_revision_metadata(999);
        assert!(matches!(err, Err(StorageError::RevisionNotFound(999))));
    }

    #[test]
    fn repository_not_found() {
        let backend = SqliteBackend::in_memory().unwrap();
        let err = backend.load_latest_graph("nonexistent");
        assert!(matches!(err, Err(StorageError::RepositoryNotFound(_))));
    }

    #[test]
    fn remove_repository() {
        let mut backend = SqliteBackend::in_memory().unwrap();
        let metadata = sample_metadata("remove-me");
        let version = GraphVersion::new(1, 0);
        let empty = KnowledgeGraph::new(
            Vec::new(),
            Vec::new(),
            BTreeMap::new(),
            Vec::new(),
            Vec::new(),
        );

        backend
            .save_revision("remove-me", &metadata, &empty, version)
            .unwrap();

        backend.remove_repository("remove-me").unwrap();

        let err = backend.load_latest_graph("remove-me");
        assert!(matches!(err, Err(StorageError::RepositoryNotFound(_))));
        assert_eq!(backend.list_revisions("remove-me").unwrap().len(), 0);
    }

    #[test]
    fn cache_metadata() {
        let mut backend = SqliteBackend::in_memory().unwrap();
        let graph = sample_graph();
        let metadata = sample_metadata("cache-test");
        let version = GraphVersion::new(1, 0);

        backend
            .save_revision("cache-test", &metadata, &graph, version)
            .unwrap();

        let cache = backend.cache_metadata("cache-test").unwrap();
        assert_eq!(cache.latest_revision, Some(1));
        assert_eq!(cache.revision_count, 1);
        assert_eq!(cache.total_nodes, 4);
        assert_eq!(cache.total_relationships, 3);
    }
}
