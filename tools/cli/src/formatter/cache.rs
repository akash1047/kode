use crate::presenter::cache::CacheStatusView;

use super::style;

pub fn format(view: &CacheStatusView) -> String {
    let latest = view
        .latest_revision
        .map_or("none".into(), |r| r.to_string());
    let mut out = style::header("cache");
    out.push_str(&format!(
        "  ● {} · schema {} · graph {}\n  ● {} revisions (latest: {}) · {} nodes · {} relationships\n",
        view.repository_id,
        view.schema_version,
        view.graph_version,
        view.revision_count,
        latest,
        view.total_nodes,
        view.total_relationships,
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presenter::cache::CacheStatusView;

    fn view(latest: Option<u64>) -> CacheStatusView {
        CacheStatusView {
            repository_id: "test-repo".into(),
            revision_count: 5,
            total_nodes: 1000,
            total_relationships: 500,
            schema_version: "1.0".into(),
            graph_version: "2.0".into(),
            latest_revision: latest,
        }
    }

    #[test]
    fn test_cache_format_with_latest() {
        let out = format(&view(Some(42)));
        assert!(out.contains("test-repo"));
        assert!(out.contains("5 revisions"));
        assert!(out.contains("42"));
    }

    #[test]
    fn test_cache_format_no_latest() {
        let out = format(&view(None));
        assert!(out.contains("none"));
    }
}
