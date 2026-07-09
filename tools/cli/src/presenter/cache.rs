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
