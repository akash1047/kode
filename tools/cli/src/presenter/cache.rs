use kode_storage::CacheMetadata;

pub struct CacheStatusView {
    pub revision_count: usize,
    pub total_nodes: usize,
    pub total_relationships: usize,
    pub schema_version: String,
    pub graph_version: String,
    pub latest_revision: Option<u64>,
    pub repository_id: String,
}

impl CacheStatusView {
    pub fn from_metadata(meta: &CacheMetadata) -> Self {
        Self {
            revision_count: meta.revision_count,
            total_nodes: meta.total_nodes,
            total_relationships: meta.total_relationships,
            schema_version: meta.schema_version.to_string(),
            graph_version: meta.graph_version.to_string(),
            latest_revision: meta.latest_revision,
            repository_id: meta.repository_id.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kode_graph::GraphVersion;
    use kode_storage::SchemaVersion;

    fn make_meta(latest: Option<u64>) -> CacheMetadata {
        CacheMetadata {
            repository_id: "test-repo".into(),
            revision_count: 5,
            total_nodes: 1000,
            total_relationships: 500,
            schema_version: SchemaVersion::new(1, 0),
            graph_version: GraphVersion::new(2, 0),
            latest_revision: latest,
            last_accessed: None,
        }
    }

    #[test]
    fn test_cache_view_with_latest_revision() {
        let meta = make_meta(Some(42));
        let view = CacheStatusView::from_metadata(&meta);
        assert_eq!(view.repository_id, "test-repo");
        assert_eq!(view.revision_count, 5);
        assert_eq!(view.total_nodes, 1000);
        assert_eq!(view.total_relationships, 500);
        assert_eq!(view.schema_version, "1.0");
        assert_eq!(view.graph_version, "2.0");
        assert_eq!(view.latest_revision, Some(42));
    }

    #[test]
    fn test_cache_view_without_latest_revision() {
        let meta = make_meta(None);
        let view = CacheStatusView::from_metadata(&meta);
        assert_eq!(view.latest_revision, None);
    }
}
