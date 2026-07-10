use kode_app::ScanResult;

use super::{format_languages, format_workspace};

pub struct ScanView {
    pub repository_root: String,
    pub workspace: String,
    pub files_discovered: usize,
    pub directories: usize,
    pub languages: String,
    pub manifests: usize,
    pub parsed: usize,
    pub recovered: usize,
    pub skipped: usize,
    pub failed: usize,
    pub elapsed_secs: f64,
}

impl ScanView {
    pub fn from_scan_result(result: &ScanResult) -> Self {
        let snapshot = &result.snapshot;
        let stats = &result.statistics;
        Self {
            repository_root: snapshot.repository().root().display().to_string(),
            workspace: format_workspace(snapshot),
            files_discovered: stats.files_discovered,
            directories: stats.directories,
            languages: format_languages(snapshot),
            manifests: stats.manifests,
            parsed: stats.parsed,
            recovered: stats.recovered,
            skipped: stats.skipped,
            failed: stats.failed,
            elapsed_secs: stats.elapsed.as_secs_f64(),
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
        dirs: usize,
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
                elapsed: Duration::from_secs(1),
                files_discovered: files,
                directories: dirs,
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
    fn test_scan_view_normal_values() {
        let result = make_result(100, 10, 80, 5, 10, 5);
        let view = ScanView::from_scan_result(&result);
        assert_eq!(view.files_discovered, 100);
        assert_eq!(view.directories, 10);
        assert_eq!(view.parsed, 80);
        assert_eq!(view.recovered, 5);
        assert_eq!(view.skipped, 10);
        assert_eq!(view.failed, 5);
        assert_eq!(view.elapsed_secs, 1.0);
    }

    #[test]
    fn test_scan_view_zero_values() {
        let result = make_result(0, 0, 0, 0, 0, 0);
        let view = ScanView::from_scan_result(&result);
        assert_eq!(view.files_discovered, 0);
        assert_eq!(view.parsed, 0);
        assert_eq!(view.elapsed_secs, 1.0);
    }
}
