use crate::presenter::cache::CacheStatusView;

pub fn format(view: &CacheStatusView) -> String {
    format!(
        "Cache Status\n\
         \x20 Repository: {repo}\n\
         \x20 Schema: {schema}\n\
         \x20 Graph version: {gver}\n\
         \x20 Revisions: {revs}\n\
         \x20 Latest revision: {latest}\n\
         \x20 Nodes: {nodes}\n\
         \x20 Relationships: {rels}\n",
        repo = view.repository_id,
        schema = view.schema_version,
        gver = view.graph_version,
        revs = view.revision_count,
        latest = view
            .latest_revision
            .map_or("none".into(), |r| r.to_string()),
        nodes = view.total_nodes,
        rels = view.total_relationships,
    )
}
