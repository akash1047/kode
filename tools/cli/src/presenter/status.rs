use kode_app::ScanResult;

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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kode_acquisition::*;
    use kode_app::ScanStatistics;
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
                graph_nodes: 0,
                graph_relationships: 0,
                storage_revision: None,
                storage_path: None,
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
        assert_eq!(view.recovered, 3);
        assert_eq!(view.skipped, 5);
        assert_eq!(view.failed, 2);
    }

    #[test]
    fn test_status_view_zero_values() {
        let result = make_result(0, 0, 0, 0, 0);
        let view = StatusView::from_scan_result(&result);
        assert_eq!(view.files, 0);
        assert_eq!(view.parsed, 0);
    }
}
