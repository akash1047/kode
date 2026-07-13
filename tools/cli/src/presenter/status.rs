use kode_app::ScanResult;
use kode_query::GraphStats;
use kode_storage::CacheMetadata;

use super::{format_languages, format_workspace};

pub struct StatusView {
    pub repository_root: String,
    pub workspace: String,
    pub files: usize,
    pub languages: String,
    pub parsed: usize,
    pub recovered: usize,
    pub skipped: usize,
    pub failed: usize,
    /// `"cache"` or `"scan"`.
    pub source: String,
    pub graph_nodes: usize,
    pub graph_relationships: usize,
    pub revision: Option<u64>,
}

impl StatusView {
    pub fn from_scan_result(result: &ScanResult) -> Self {
        let snapshot = &result.snapshot;
        let stats = &result.statistics;
        Self {
            repository_root: snapshot.repository().root().display().to_string(),
            workspace: format_workspace(snapshot),
            files: stats.files_discovered,
            languages: format_languages(snapshot),
            parsed: stats.parsed,
            recovered: stats.recovered,
            skipped: stats.skipped,
            failed: stats.failed,
            source: "scan".into(),
            graph_nodes: stats.graph_nodes,
            graph_relationships: stats.graph_relationships,
            revision: stats.storage_revision,
        }
    }

    /// Build status from a persisted graph index (no re-scan).
    pub fn from_cache(repository_root: String, meta: &CacheMetadata, stats: &GraphStats) -> Self {
        Self {
            repository_root,
            workspace: "(from cache)".into(),
            files: stats.file_count,
            languages: if stats.languages.is_empty() {
                "—".into()
            } else {
                stats.languages.join(", ")
            },
            parsed: stats.entity_count,
            recovered: 0,
            skipped: 0,
            failed: 0,
            source: "cache".into(),
            graph_nodes: meta.total_nodes.max(stats.node_count),
            graph_relationships: meta.total_relationships.max(stats.relationship_count),
            revision: meta.latest_revision,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kode_acquisition::*;
    use kode_app::ScanStatistics;
    use kode_graph::GraphVersion;
    use kode_storage::SchemaVersion;
    use std::time::Duration;

    fn make_result(
        files: usize,
        parsed: usize,
        recovered: usize,
        skipped: usize,
        failed: usize,
    ) -> ScanResult {
        let repo = Repository::new(".").unwrap();
        let ws = Workspace {
            kind: WorkspaceKind::None,
        };
        let snapshot = SnapshotBuilder::new()
            .repository(repo)
            .workspace(ws)
            .build()
            .unwrap();
        ScanResult {
            snapshot,
            statistics: ScanStatistics {
                elapsed: Duration::from_secs(0),
                files_discovered: files,
                directories: 0,
                manifests: 0,
                languages: 0,
                parsed,
                recovered,
                skipped,
                failed,
                entities_extracted: 0,
                graph_nodes: 10,
                graph_relationships: 5,
                storage_revision: Some(1),
                storage_path: None,
                cache_hit: false,
            },
            graph: None,
            revision: None,
        }
    }

    #[test]
    fn test_status_view_normal_values() {
        let result = make_result(50, 40, 3, 5, 2);
        let view = StatusView::from_scan_result(&result);
        assert_eq!(view.files, 50);
        assert_eq!(view.parsed, 40);
        assert_eq!(view.source, "scan");
        assert_eq!(view.graph_nodes, 10);
    }

    #[test]
    fn test_status_view_from_cache() {
        let meta = CacheMetadata {
            repository_id: "r".into(),
            latest_revision: Some(3),
            revision_count: 1,
            schema_version: SchemaVersion::new(1, 0),
            graph_version: GraphVersion::new(1, 0),
            last_accessed: None,
            total_nodes: 100,
            total_relationships: 50,
        };
        let stats = GraphStats {
            node_count: 100,
            relationship_count: 50,
            file_count: 12,
            entity_count: 80,
            languages: vec!["Rust".into()],
        };
        let view = StatusView::from_cache("/repo".into(), &meta, &stats);
        assert_eq!(view.source, "cache");
        assert_eq!(view.files, 12);
        assert_eq!(view.revision, Some(3));
        assert!(view.languages.contains("Rust"));
    }
}
