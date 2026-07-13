use kode_acquisition::Language;
use kode_app::ScanResult;
use kode_query::FileResult;

pub struct FilesView {
    pub entries: Vec<(String, String)>,
}

impl FilesView {
    pub fn from_scan_result(result: &ScanResult, filter: Option<&Language>) -> Self {
        let entries = result
            .snapshot
            .files()
            .iter()
            .filter(|f| {
                if let Some(lang) = filter {
                    f.language() == Some(lang)
                } else {
                    true
                }
            })
            .map(|f| {
                let lang = f.language().map(|l| l.to_string()).unwrap_or_default();
                (lang, f.relative_path().display().to_string())
            })
            .collect();
        Self { entries }
    }

    /// Build a files view from knowledge-graph file nodes.
    pub fn from_file_results(files: &[FileResult]) -> Self {
        let entries = files
            .iter()
            .map(|f| {
                (
                    f.language.clone().unwrap_or_default(),
                    f.path.display().to_string(),
                )
            })
            .collect();
        Self { entries }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kode_acquisition::*;
    use kode_app::ScanStatistics;
    use std::path::PathBuf;
    use std::time::Duration;

    fn make_result_with_files(files: Vec<RepositoryFile>) -> ScanResult {
        let repo = Repository::new(".").unwrap();
        let ws = Workspace {
            kind: WorkspaceKind::None,
        };
        let snapshot = SnapshotBuilder::new()
            .repository(repo)
            .workspace(ws)
            .file_inventory(FileInventory::new(files))
            .build()
            .unwrap();
        ScanResult {
            snapshot,
            statistics: ScanStatistics {
                elapsed: Duration::from_secs(0),
                files_discovered: 0,
                directories: 0,
                manifests: 0,
                languages: 0,
                parsed: 0,
                recovered: 0,
                skipped: 0,
                failed: 0,
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

    fn file(path: &str, lang: Option<Language>) -> RepositoryFile {
        RepositoryFile::new(PathBuf::from(path), FileMetadata::new(100, None), lang)
    }

    #[test]
    fn test_files_view_no_filter() {
        let result = make_result_with_files(vec![
            file("main.rs", Some(Language::Rust)),
            file("lib.rs", Some(Language::Rust)),
            file("README.md", Some(Language::Markdown)),
        ]);
        let view = FilesView::from_scan_result(&result, None);
        assert_eq!(view.entries.len(), 3);
    }

    #[test]
    fn test_files_view_filter_by_language() {
        let result = make_result_with_files(vec![
            file("main.rs", Some(Language::Rust)),
            file("README.md", Some(Language::Markdown)),
        ]);
        let view = FilesView::from_scan_result(&result, Some(&Language::Rust));
        assert_eq!(view.entries.len(), 1);
        assert_eq!(view.entries[0].1, "main.rs");
    }

    #[test]
    fn test_files_view_empty() {
        let result = make_result_with_files(vec![]);
        let view = FilesView::from_scan_result(&result, None);
        assert!(view.entries.is_empty());
    }

    #[test]
    fn test_files_view_no_match_filter() {
        let result = make_result_with_files(vec![file("main.rs", Some(Language::Rust))]);
        let view = FilesView::from_scan_result(&result, Some(&Language::Python));
        assert!(view.entries.is_empty());
    }
}
